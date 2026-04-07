#ifndef RUSTY_MEM_HPP
#define RUSTY_MEM_HPP

#include <cstddef>
#include <mutex>
#include <new>
#include <tuple>
#include <type_traits>
#include <unordered_map>
#include <utility>

namespace rusty {
namespace mem {

namespace detail {
inline std::unordered_map<const void*, std::size_t>& forgotten_addresses() {
    static std::unordered_map<const void*, std::size_t> addresses;
    return addresses;
}

inline std::mutex& forgotten_addresses_mutex() {
    static std::mutex mutex;
    return mutex;
}

template<typename T, typename = void>
struct rust_layout_size {
    static constexpr std::size_t value = sizeof(T);
};

// Emulate Rust layout for transpiled fixed-capacity containers that expose:
// - `len_field` length bookkeeping,
// - `xs` fixed storage array,
// - `CAPACITY` compile-time capacity.
// This preserves Rust `mem::size_of` semantics for zero-capacity specializations
// where C++ `std::array<T, 0>` still occupies one byte.
template<typename T>
struct rust_layout_size<
    T,
    std::void_t<decltype(T::CAPACITY),
                decltype(std::declval<T&>().len_field),
                decltype(std::declval<T&>().xs),
                typename std::remove_cvref_t<decltype(std::declval<T&>().xs)>::value_type>> {
    using LenField = std::remove_cvref_t<decltype(std::declval<T&>().len_field)>;
    using Storage = std::remove_cvref_t<decltype(std::declval<T&>().xs)>;
    using Element = typename Storage::value_type;
    static constexpr std::size_t value =
        sizeof(LenField) + std::tuple_size_v<Storage> * sizeof(Element);
};
} // namespace detail

template<typename T>
class ManuallyDrop {
private:
    struct InitTag {};

    alignas(T) unsigned char storage_[sizeof(T)];
    bool initialized_ = false;

    explicit ManuallyDrop(T&& value, InitTag) : initialized_(true) {
        new (storage_) T(std::move(value));
    }

    explicit ManuallyDrop(const T& value, InitTag) : initialized_(true) {
        new (storage_) T(value);
    }

    T* ptr() noexcept {
        return std::launder(reinterpret_cast<T*>(storage_));
    }

    const T* ptr() const noexcept {
        return std::launder(reinterpret_cast<const T*>(storage_));
    }

public:
    ManuallyDrop() noexcept = default;
    ManuallyDrop(const ManuallyDrop&) = delete;
    ManuallyDrop& operator=(const ManuallyDrop&) = delete;

    // Intentional no-op destructor: mirrors Rust ManuallyDrop semantics.
    ~ManuallyDrop() = default;

    template<typename U = T>
    static ManuallyDrop<T> new_(U&& value) {
        using Value = std::remove_reference_t<U>;
        if constexpr (std::is_same_v<Value, T>) {
            return ManuallyDrop<T>(std::forward<U>(value), InitTag{});
        } else {
            return ManuallyDrop<T>(T(std::forward<U>(value)), InitTag{});
        }
    }

    T* as_mut_ptr() noexcept {
        return ptr();
    }

    const T* as_ptr() const noexcept {
        return ptr();
    }

    T& operator*() noexcept {
        return *ptr();
    }

    const T& operator*() const noexcept {
        return *ptr();
    }
};

template<typename T>
inline auto manually_drop_new(T&& value)
    -> ManuallyDrop<std::remove_cv_t<std::remove_reference_t<T>>> {
    using Value = std::remove_cv_t<std::remove_reference_t<T>>;
    return ManuallyDrop<Value>::new_(std::forward<T>(value));
}

template<typename T>
constexpr std::size_t size_of() noexcept {
    using Value = std::remove_cv_t<std::remove_reference_t<T>>;
    return detail::rust_layout_size<Value>::value;
}

inline void mark_forgotten_address(const void* address) noexcept {
    if (address == nullptr) {
        return;
    }
    std::lock_guard<std::mutex> lock(detail::forgotten_addresses_mutex());
    auto& addresses = detail::forgotten_addresses();
    addresses[address] += 1;
}

inline bool consume_forgotten_address(const void* address) noexcept {
    if (address == nullptr) {
        return false;
    }
    std::lock_guard<std::mutex> lock(detail::forgotten_addresses_mutex());
    auto& addresses = detail::forgotten_addresses();
    const auto it = addresses.find(address);
    if (it == addresses.end()) {
        return false;
    }
    if (it->second > 1) {
        it->second -= 1;
    } else {
        addresses.erase(it);
    }
    return true;
}

template<typename T, typename U>
inline T replace(T& destination, U&& value) {
    // Build replacement first so aliasing inputs are consumed before we
    // destroy the destination, then reconstruct in-place to avoid requiring
    // copy/move assignment on T.
    T replacement(std::forward<U>(value));
    T old(std::move(destination));
    destination.~T();
    new (&destination) T(std::move(replacement));
    return old;
}

// Rust std::mem::drop consumes a value and destroys it at the end of this call.
template<typename T>
inline void drop(T value) {
    [[maybe_unused]] auto* consume = &value;
    (void)consume;
}

// Rust std::mem::forget consumes a value and intentionally leaks/drop-skips it.
// For drop-enabled transpiled structs, mark the value as forgotten so generated
// destructors can skip user Drop bodies on scope-exit/moved-from states.
template<typename T>
inline void forget(T&& value) noexcept {
    using Value = std::remove_reference_t<T>;
    if constexpr (requires(Value& v) { v.rusty_mark_forgotten(); }) {
        value.rusty_mark_forgotten();
    }
}

} // namespace mem
} // namespace rusty

#endif // RUSTY_MEM_HPP
