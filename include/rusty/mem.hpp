#ifndef RUSTY_MEM_HPP
#define RUSTY_MEM_HPP

#include <cstddef>
#include <new>
#include <type_traits>
#include <utility>

namespace rusty {
namespace mem {

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
    return sizeof(T);
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
inline void drop(T&& value) noexcept {
    using Value = std::remove_reference_t<T>;
    [[maybe_unused]] Value consumed = std::forward<T>(value);
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
