#ifndef RUSTY_MAYBE_UNINIT_HPP
#define RUSTY_MAYBE_UNINIT_HPP

#include <cstddef>
#include <new>
#include <span>
#include <type_traits>
#include <utility>

// MaybeUninit<T> - Uninitialized storage for a value of type T
// Equivalent to Rust's MaybeUninit<T>
//
// This type provides a way to have uninitialized memory that can later
// be initialized with a value. Unlike regular variables, no constructor
// is called when MaybeUninit is created.
//
// Safety: The user is responsible for:
// - Not reading from uninitialized storage
// - Properly initializing before use
// - Properly destroying initialized values

// @safe
namespace rusty {

template<typename T>
class MaybeUninit {
private:
    alignas(T) unsigned char storage_[sizeof(T)];

public:
    // Default constructor - does NOT initialize the value
    MaybeUninit() noexcept = default;

    // Trivial copy/move — MaybeUninit is just raw bytes, so bitwise
    // copy is always safe (no constructors/destructors involved).
    // This is essential so std::array<MaybeUninit<T>, N> remains
    // trivially moveable/copyable, allowing container types like
    // ArrayString to be used as HashMap keys, return values, etc.
    MaybeUninit(const MaybeUninit& other) noexcept {
        __builtin_memcpy(storage_, other.storage_, sizeof(T));
    }
    MaybeUninit& operator=(const MaybeUninit& other) noexcept {
        if (this != &other) __builtin_memcpy(storage_, other.storage_, sizeof(T));
        return *this;
    }
    MaybeUninit(MaybeUninit&& other) noexcept {
        __builtin_memcpy(storage_, other.storage_, sizeof(T));
    }
    MaybeUninit& operator=(MaybeUninit&& other) noexcept {
        if (this != &other) __builtin_memcpy(storage_, other.storage_, sizeof(T));
        return *this;
    }

    // Destructor does NOT destroy the value - caller must do it
    ~MaybeUninit() = default;

    // Get pointer to the storage (for placement new)
    std::add_pointer_t<T> as_mut_ptr() noexcept {
        return std::launder(reinterpret_cast<std::add_pointer_t<T>>(storage_));
    }

    std::add_pointer_t<std::add_const_t<T>> as_ptr() const noexcept {
        return std::launder(reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>(storage_));
    }

    // Initialize in-place with constructor arguments
    template<typename... Args>
    void write(Args&&... args) {
        new (storage_) T(std::forward<Args>(args)...);
    }

    template<typename... Args>
    void write_(Args&&... args) {
        write(std::forward<Args>(args)...);
    }

    // Initialize by moving a value in
    void write_value(T value) {
        new (storage_) T(std::move(value));
    }

    // Assume initialized and get reference (UNSAFE - caller must ensure initialized)
    T& assume_init_ref() noexcept {
        return *as_mut_ptr();
    }

    const T& assume_init_ref() const noexcept {
        return *as_ptr();
    }

    // Rust's `MaybeUninit::assume_init_mut(&mut self) -> &mut T`. Same
    // contract as the non-const `assume_init_ref` overload above, but
    // exposed under the spelling that the transpiler emits for the
    // `_mut` variant. (Rust has both methods; C++ would collapse them
    // via overloading, but the transpiler doesn't reshape callsites.)
    T& assume_init_mut() noexcept {
        return *as_mut_ptr();
    }

    // Rust's `MaybeUninit::assume_init_read(&self) -> T` — read a copy
    // of the contained value without destroying the source. Caller must
    // ensure (1) the value is initialized and (2) for non-Copy T the
    // aliasing/double-drop hazards are avoided. For trivially-copyable
    // T (the usual transpiled-btree usage: NonNull, sizes), this is a
    // bitwise copy. Gated on `is_copy_constructible_v<T>` so that
    // instantiating MaybeUninit<MoveOnlyT> does not eagerly require a
    // copy ctor that won't exist (btree_port B4 — surfaces when a map
    // is instantiated with a move-only value type like
    // `std::pair<long, rusty::Function<void()>>`).
    // @unsafe
    T assume_init_read() const noexcept(std::is_nothrow_copy_constructible_v<T>)
        requires (std::is_copy_constructible_v<T>)
    {
        return *as_ptr();
    }

    // Move-only T overload — mirrors Rust's `ptr::read`, which is a
    // bitwise copy. After this call the source storage is conceptually
    // "moved-from": the caller must not read it again and must not
    // re-invoke a destructor on the source. This matches the contract
    // of Rust's `ptr::read`. We use `__builtin_memcpy` to bypass the
    // missing copy/move ctor — equivalent to a bitwise relocation.
    // Used by btree's slice_remove and the dying-iterator paths where
    // the source storage is unconditionally overwritten or freed
    // immediately afterwards (e.g. `rusty::ptr::copy(...)` shifts the
    // rest of the slice down on top of the read slot).
    // @unsafe
    T assume_init_read() const noexcept(std::is_nothrow_move_constructible_v<T>)
        requires (!std::is_copy_constructible_v<T>
                  && std::is_move_constructible_v<T>)
    {
        alignas(T) std::byte buf[sizeof(T)];
        __builtin_memcpy(buf, storage_, sizeof(T));
        T* p = std::launder(reinterpret_cast<T*>(buf));
        return std::move(*p);
    }

    // Assume initialized and move out (UNSAFE - caller must ensure initialized)
    // After this, the storage is uninitialized again
    T assume_init() noexcept(std::is_nothrow_move_constructible_v<T>) {
        T value = std::move(*as_mut_ptr());
        if constexpr (!std::is_reference_v<T>) {
            as_mut_ptr()->~T();
        }
        return value;
    }

    // Const overload: for trivially-copyable T, dereferencing `*const T`
    // and calling `.assume_init()` is legal in Rust (the field is copied
    // out by value, then `assume_init()` consumes the copy). C++ needs
    // the receiver to be non-const for the move-out variant — provide
    // a const path that just copies, matching Rust's effective behavior
    // for Copy types.
    T assume_init() const noexcept(std::is_nothrow_copy_constructible_v<T>)
        requires (std::is_copy_constructible_v<T>)
    {
        return *as_ptr();
    }

    // Destroy the value (UNSAFE - caller must ensure initialized)
    void destroy() noexcept(std::is_nothrow_destructible_v<T>) {
        if constexpr (!std::is_reference_v<T>) {
            as_mut_ptr()->~T();
        }
    }

    // Rust's `MaybeUninit::assume_init_drop(&mut self)` — drop the inner
    // value in place. Same semantics as `destroy()`; aliased here so the
    // transpiler can emit the Rust-side spelling without rewriting.
    void assume_init_drop() noexcept(std::is_nothrow_destructible_v<T>) {
        destroy();
    }

    // Create with an initial value.
    // `new_` is the primary Rust-surface entrypoint (`MaybeUninit::new` in Rust),
    // while `new_with` is kept as a compatibility alias.
    static MaybeUninit<T> new_(T value) {
        MaybeUninit<T> mu;
        mu.write_value(std::move(value));
        return mu;
    }

    // Compatibility alias for historical runtime call-sites.
    static MaybeUninit<T> new_with(T value) {
        return MaybeUninit<T>::new_(std::move(value));
    }

    // Create uninitialized
    static MaybeUninit<T> uninit() {
        return MaybeUninit<T>();
    }

    // Create storage with all bytes set to zero.
    // Mirrors Rust's MaybeUninit::zeroed semantics.
    static MaybeUninit<T> zeroed() noexcept {
        MaybeUninit<T> mu;
        __builtin_memset(mu.storage_, 0, sizeof(T));
        return mu;
    }

    // Rust's `MaybeUninit::slice_assume_init_ref(&[MaybeUninit<T>]) -> &[T]`
    // and `slice_assume_init_mut(&mut [MaybeUninit<T>]) -> &mut [T]`. The
    // transpiler emits these as `MaybeUninit<T>::slice_assume_init_ref(s)`
    // (T inferred from the argument's slice element type), so the outer T
    // here is the same T that parameterizes the input slice.
    // @unsafe
    static std::span<const T> slice_assume_init_ref(
        std::span<const MaybeUninit<T>> slice) noexcept {
        return std::span<const T>(
            reinterpret_cast<const T*>(slice.data()), slice.size());
    }

    // @unsafe
    static std::span<T> slice_assume_init_mut(
        std::span<MaybeUninit<T>> slice) noexcept {
        return std::span<T>(
            reinterpret_cast<T*>(slice.data()), slice.size());
    }
};

// Free-function alias used by the transpiled btree_internal where the
// emit shape is `slice.assume_init_ref()` (method-style on a span) but
// the corresponding member doesn't exist on std::span. Routing this
// through rusty namespace gives the patcher a single substitution
// target: `.assume_init_ref()` → `, rusty::assume_init_ref` shape.
template<typename T>
inline std::span<const T> assume_init_ref(
    std::span<const MaybeUninit<T>> slice) noexcept {
    return std::span<const T>(
        reinterpret_cast<const T*>(slice.data()), slice.size());
}
template<typename T>
inline std::span<T> assume_init_ref(
    std::span<MaybeUninit<T>> slice) noexcept {
    return std::span<T>(
        reinterpret_cast<T*>(slice.data()), slice.size());
}

// UninitArray<T, N> - Fixed-size array of uninitialized storage
// Similar to [MaybeUninit<T>; N] in Rust
//
// This is optimized for contiguous storage and provides
// helper methods for managing partially-initialized arrays.

template<typename T, size_t N>
class UninitArray {
private:
    alignas(T) unsigned char storage_[N * sizeof(T)];

public:
    static constexpr size_t capacity = N;

    // Default constructor - no initialization
    UninitArray() noexcept = default;

    // Trivial copy/move — raw bytes, bitwise copy is safe.
    UninitArray(const UninitArray& other) noexcept {
        __builtin_memcpy(storage_, other.storage_, N * sizeof(T));
    }
    UninitArray& operator=(const UninitArray& other) noexcept {
        if (this != &other) __builtin_memcpy(storage_, other.storage_, N * sizeof(T));
        return *this;
    }
    UninitArray(UninitArray&& other) noexcept {
        __builtin_memcpy(storage_, other.storage_, N * sizeof(T));
    }
    UninitArray& operator=(UninitArray&& other) noexcept {
        if (this != &other) __builtin_memcpy(storage_, other.storage_, N * sizeof(T));
        return *this;
    }

    // Destructor does NOT destroy elements
    ~UninitArray() = default;

    // Get raw pointer to element storage
    T* data() noexcept {
        return reinterpret_cast<T*>(storage_);
    }

    const T* data() const noexcept {
        return reinterpret_cast<const T*>(storage_);
    }

    // Access element (UNSAFE - must be initialized)
    T& operator[](size_t i) noexcept {
        return data()[i];
    }

    const T& operator[](size_t i) const noexcept {
        return data()[i];
    }

    // Get pointer to element
    T* ptr_at(size_t i) noexcept {
        return &data()[i];
    }

    const T* ptr_at(size_t i) const noexcept {
        return &data()[i];
    }

    // Initialize element at position
    template<typename... Args>
    void construct_at(size_t pos, Args&&... args) {
        new (&data()[pos]) T(std::forward<Args>(args)...);
    }

    // Destroy element at position
    void destroy_at(size_t pos) noexcept(std::is_nothrow_destructible_v<T>) {
        data()[pos].~T();
    }

    // Destroy range [0, len)
    void destroy_range(size_t len) noexcept(std::is_nothrow_destructible_v<T>) {
        for (size_t i = 0; i < len; ++i) {
            data()[i].~T();
        }
    }

    // Move element from one position to another (both must be valid)
    void move_element(size_t from, size_t to) {
        new (&data()[to]) T(std::move(data()[from]));
        data()[from].~T();
    }

    // Shift elements right: [pos, pos+count) -> [pos+1, pos+count+1)
    // Caller must ensure there's room and elements are initialized
    void shift_right(size_t pos, size_t count) {
        if (count == 0) return;
        // Move from right to left to avoid overwriting
        for (size_t i = count; i > 0; --i) {
            size_t src = pos + i - 1;
            size_t dst = pos + i;
            new (&data()[dst]) T(std::move(data()[src]));
            data()[src].~T();
        }
    }

    // Shift elements left: [pos+1, pos+1+count) -> [pos, pos+count)
    void shift_left(size_t pos, size_t count) {
        if (count == 0) return;
        for (size_t i = 0; i < count; ++i) {
            size_t src = pos + 1 + i;
            size_t dst = pos + i;
            new (&data()[dst]) T(std::move(data()[src]));
            data()[src].~T();
        }
    }
};

} // namespace rusty

#endif // RUSTY_MAYBE_UNINIT_HPP
