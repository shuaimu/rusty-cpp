#ifndef RUSTY_MEM_HPP
#define RUSTY_MEM_HPP

#include <array>
#include <rusty/maybe_uninit.hpp>
#include <cstdint>
#include <cstddef>
#include <cstring>
#include <limits>
#include <memory>
#include <new>
#include <tuple>
#include <type_traits>
#include <unordered_map>
#include <utility>
#include <variant>
#include <rusty/platform/threading.hpp>

namespace rusty {
namespace mem {

// Rust spells it `mem::MaybeUninit`; the port's class lives at rusty:: root
// (maybe_uninit.hpp). Alias it here so `mem::MaybeUninit<T>` paths resolve.
using ::rusty::MaybeUninit;


namespace detail {
// The global forgotten-address table is GONE under the strict null-state
// convention (see docs/rusty-std-book.md §1.5). Every transpiler-emitted
// owning type now carries a local `mutable bool _rusty_forgotten = false;`
// that its move ctor / destructor / `mem::forget` set, replacing the
// per-element global-table dance that was the 1700-10000x perf cliff.
//
// The `mark_forgotten_key` / `consume_forgotten_key` entry points are kept
// as no-ops so any externally-vendored code that still calls them compiles
// and runs without effect. New transpiled code never calls these.

template<typename T>
inline const void* forgotten_type_tag() noexcept {
    static const int tag = 0;
    return &tag;
}

inline void mark_forgotten_key(const void*, const void*) noexcept {}
inline bool consume_forgotten_key(const void*, const void*) noexcept { return false; }

template<typename T, typename = void>
struct rust_layout_size {
    static constexpr std::size_t value = sizeof(T);
};

template<typename T, typename = void>
struct rust_layout_align {
    static constexpr std::size_t value = alignof(T);
};

template<typename T>
using remove_cvref_t = std::remove_cv_t<std::remove_reference_t<T>>;

template<typename T, typename = void>
struct variant_like {
    using type = remove_cvref_t<T>;
};

template<typename T>
struct variant_like<T, std::void_t<typename remove_cvref_t<T>::variant>> {
    using type = typename remove_cvref_t<T>::variant;
};

template<typename T>
using variant_like_t = typename variant_like<T>::type;

template<typename T>
concept smallvec_like_layout = requires(T& value) {
    value.capacity_field;
    value.data;
    typename std::variant_alternative_t<0, variant_like_t<decltype(value.data)>>;
    typename std::variant_alternative_t<1, variant_like_t<decltype(value.data)>>;
    std::declval<std::variant_alternative_t<0, variant_like_t<decltype(value.data)>>&>()._0;
    std::declval<std::variant_alternative_t<1, variant_like_t<decltype(value.data)>>&>().ptr;
    std::declval<std::variant_alternative_t<1, variant_like_t<decltype(value.data)>>&>().len;
};

template<smallvec_like_layout T>
constexpr std::size_t smallvec_like_rust_layout_size() noexcept {
    using Data = variant_like_t<decltype(std::declval<T&>().data)>;
    using InlineVariant = std::variant_alternative_t<0, Data>;
    using InlineStorage =
        remove_cvref_t<decltype(std::declval<InlineVariant&>()._0)>;

    constexpr std::size_t pointer_size = sizeof(std::uintptr_t);
    constexpr std::size_t inline_bytes = rust_layout_size<InlineStorage>::value;
    constexpr std::size_t rounded_inline =
        ((inline_bytes + pointer_size - 1) / pointer_size) * pointer_size;
    constexpr std::size_t payload_bytes =
        (rounded_inline < pointer_size) ? pointer_size : rounded_inline;
    return 2 * pointer_size + payload_bytes;
}

// Mirror Rust `[T; N]` layout sizing semantics for `std::array<T, N>`.
// In particular, Rust treats `[T; 0]` as size 0 while C++ `std::array<T, 0>`
// commonly has size 1.
template<typename T, std::size_t N>
struct rust_layout_size<std::array<T, N>, void> {
    static constexpr std::size_t value = N * sizeof(T);
};

// Mirror Rust `[T; N]` alignment semantics for `std::array<T, N>`.
// Rust keeps array alignment equal to element alignment even for N=0.
template<typename T, std::size_t N>
struct rust_layout_align<std::array<T, N>, void> {
    static constexpr std::size_t value = alignof(T);
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

template<typename T, typename... Args>
inline void leak_construct(Args&&... args) noexcept {
    void* storage = ::operator new(sizeof(T), std::nothrow);
    if (storage == nullptr) {
        return;
    }
    try {
        new (storage) T(std::forward<Args>(args)...);
    } catch (...) {
        ::operator delete(storage);
    }
}
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

    // Move ctor: transfer the contained value. Mirrors Rust's
    // implicit move semantics for ManuallyDrop<T> (T is moved, the
    // source ManuallyDrop becomes uninitialized).
    ManuallyDrop(ManuallyDrop&& other) noexcept(std::is_nothrow_move_constructible_v<T>)
        : initialized_(other.initialized_) {
        if (other.initialized_) {
            new (storage_) T(std::move(*other.ptr()));
            other.initialized_ = false;
        }
    }
    ManuallyDrop& operator=(ManuallyDrop&& other) noexcept(std::is_nothrow_move_constructible_v<T>) {
        if (this != &other) {
            if (initialized_) {
                ptr()->~T();
            }
            initialized_ = other.initialized_;
            if (other.initialized_) {
                new (storage_) T(std::move(*other.ptr()));
                other.initialized_ = false;
            }
        }
        return *this;
    }

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

    // Rust Deref-coercion shim: transpiled Vec/VecDeque IntoIterator glue
    // spells `me.allocator()` on a ManuallyDrop<coll> (auto-deref in Rust).
    decltype(auto) allocator() const
        requires requires(const T& t) { t.allocator(); }
    {
        return (**this).allocator();
    }

    // Rust's `ManuallyDrop::take(&mut slot) -> T` is a static fn that moves
    // the inner value out, leaving the wrapper logically uninitialized.
    // Transpiler emits it as an instance call `slot.take()` rather than the
    // free-fn form, so expose it as a method here.
    T take() noexcept(std::is_nothrow_move_constructible_v<T>) {
        T result = std::move(*ptr());
        initialized_ = false;
        return result;
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
    if constexpr (detail::smallvec_like_layout<Value>) {
        return detail::smallvec_like_rust_layout_size<Value>();
    }
    return detail::rust_layout_size<Value>::value;
}

template<typename T>
constexpr std::size_t align_of() noexcept {
    using Value = std::remove_cv_t<std::remove_reference_t<T>>;
    return detail::rust_layout_align<Value>::value;
}

template<typename From, typename To>
inline To transmute(From from) {
    using FromValue = std::remove_reference_t<From>;
    static_assert(
        sizeof(FromValue) == sizeof(To),
        "rusty::mem::transmute requires source and destination of equal size");

    alignas(To) unsigned char storage[sizeof(To)];
    std::memcpy(
        static_cast<void*>(storage),
        static_cast<const void*>(std::addressof(from)),
        sizeof(To));
    auto* out_ptr = std::launder(reinterpret_cast<To*>(storage));
    if constexpr (std::is_copy_constructible_v<To>) {
        return *out_ptr;
    } else {
        return std::move(*out_ptr);
    }
}

inline void mark_forgotten_address(const void* address) noexcept {
    detail::mark_forgotten_key(address, nullptr);
}

inline bool consume_forgotten_address(const void* address) noexcept {
    return detail::consume_forgotten_key(address, nullptr);
}

template<typename T>
inline void mark_forgotten_typed(const T* address) noexcept {
    using Value = std::remove_cv_t<std::remove_reference_t<T>>;
    detail::mark_forgotten_key(address, detail::forgotten_type_tag<Value>());
}

template<typename T>
inline bool consume_forgotten_typed(const T* address) noexcept {
    using Value = std::remove_cv_t<std::remove_reference_t<T>>;
    return detail::consume_forgotten_key(address, detail::forgotten_type_tag<Value>());
}

// No-ops under strict null-state. See note in detail::mark_forgotten_key.
inline void clear_forgotten_address_range(const void*, std::size_t) noexcept {}
inline void clear_all_forgotten_addresses() noexcept {}

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

template<typename T>
requires std::is_default_constructible_v<T>
inline T take(T& destination) {
    return replace(destination, T{});
}

template<typename T>
inline void swap(T& left, T& right) noexcept(noexcept(std::swap(left, right))) {
    std::swap(left, right);
}

// Rust deref-coerces `&mut wrapper` args when mem::swap's unification wants
// the wrapped type (`mem::swap(self, &mut guard)` with guard:
// ScopeGuard<Self, F> swaps against *guard). Mirror it for mixed-type
// pairs: deref the side whose operator* yields the other.
template<typename T, typename U>
    requires (!std::is_same_v<T, U>)
inline void swap(T& left, U& right) {
    if constexpr (requires(T& l, U& r) { swap(l, *r); }) {
        swap(left, *right);
    } else {
        swap(*left, right);
    }
}

// Rust std::mem::drop consumes a value and destroys it at the end of this call.
template<typename T>
inline void drop(T value) {
    [[maybe_unused]] auto* consume = &value;
    (void)consume;
}

// Rust std::mem::forget consumes a value and intentionally leaks/drop-skips it.
// For drop-enabled transpiled structs, set the value's local
// `_rusty_forgotten` flag so its destructor short-circuits past the Drop body
// at scope exit. Under the strict null-state convention this is purely local
// state — no global table involved.
template<typename T>
inline void forget(T&& value) noexcept {
    using Value = std::remove_reference_t<T>;
    using Plain = std::remove_cv_t<Value>;
    if constexpr (requires(const Plain& v) { v.rusty_mark_forgotten(); }) {
        // `rusty_mark_forgotten()` is emitted `const` and sets a `mutable`
        // member, so it works uniformly on const and non-const values
        // (including const locals like the `PanicGuard` scope-guard in
        // generated code).
        value.rusty_mark_forgotten();
    } else if constexpr (std::is_move_constructible_v<Plain> && !std::is_const_v<Value>) {
        // Generic ownership-forget fallback for non-guarded owning types (e.g. rusty::VecLegacy):
        // move payload into leaked storage so the source becomes moved-from and no longer owns.
        detail::leak_construct<Plain>(std::move(value));
    } else if constexpr (std::is_copy_constructible_v<Plain>) {
        // Last-resort fallback when move is unavailable; keeps forget surface total.
        detail::leak_construct<Plain>(value);
    }
}

// Rust's std::mem::needs_drop::<T>() — returns true if dropping T has
// any side effect (non-trivial destructor). Used by vec_deque_port's
// Drain::drop and other paths to skip destructor loops over POD types.
template<typename T>
inline constexpr bool needs_drop() noexcept {
    return !std::is_trivially_destructible_v<T>;
}

} // namespace mem

// Rust `Borrow::borrow`. A member borrow() wins (String's Borrow<str>
// port); otherwise the blanket `impl<T> Borrow<T> for T` is an identity
// borrow — forward the reference through (primitives can't spell it as a
// member: `key.borrow()` with K=int in equivalent's blanket Equivalent
// impl).
template<typename T>
decltype(auto) borrow(T&& value) {
    if constexpr (requires { std::forward<T>(value).borrow(); }) {
        return std::forward<T>(value).borrow();
    } else {
        return std::forward<T>(value);
    }
}

} // namespace rusty

#endif // RUSTY_MEM_HPP
