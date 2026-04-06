#ifndef RUSTY_MAYBE_UNINIT_HPP
#define RUSTY_MAYBE_UNINIT_HPP

#include <cstddef>
#include <new>
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

    // Assume initialized and move out (UNSAFE - caller must ensure initialized)
    // After this, the storage is uninitialized again
    T assume_init() noexcept(std::is_nothrow_move_constructible_v<T>) {
        T value = std::move(*as_mut_ptr());
        if constexpr (!std::is_reference_v<T>) {
            as_mut_ptr()->~T();
        }
        return value;
    }

    // Destroy the value (UNSAFE - caller must ensure initialized)
    void destroy() noexcept(std::is_nothrow_destructible_v<T>) {
        if constexpr (!std::is_reference_v<T>) {
            as_mut_ptr()->~T();
        }
    }

    // Create with an initial value
    static MaybeUninit<T> new_with(T value) {
        MaybeUninit<T> mu;
        mu.write_value(std::move(value));
        return mu;
    }

    // Create uninitialized
    static MaybeUninit<T> uninit() {
        return MaybeUninit<T>();
    }
};

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
