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

    // No copy/move - manage manually
    MaybeUninit(const MaybeUninit&) = delete;
    MaybeUninit& operator=(const MaybeUninit&) = delete;
    MaybeUninit(MaybeUninit&&) = delete;
    MaybeUninit& operator=(MaybeUninit&&) = delete;

    // Destructor does NOT destroy the value - caller must do it
    ~MaybeUninit() = default;

    // Get pointer to the storage (for placement new)
    T* as_mut_ptr() noexcept {
        return reinterpret_cast<T*>(storage_);
    }

    const T* as_ptr() const noexcept {
        return reinterpret_cast<const T*>(storage_);
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
        return *reinterpret_cast<T*>(storage_);
    }

    const T& assume_init_ref() const noexcept {
        return *reinterpret_cast<const T*>(storage_);
    }

    // Assume initialized and move out (UNSAFE - caller must ensure initialized)
    // After this, the storage is uninitialized again
    T assume_init() noexcept(std::is_nothrow_move_constructible_v<T>) {
        T value = std::move(*reinterpret_cast<T*>(storage_));
        reinterpret_cast<T*>(storage_)->~T();
        return value;
    }

    // Destroy the value (UNSAFE - caller must ensure initialized)
    void destroy() noexcept(std::is_nothrow_destructible_v<T>) {
        reinterpret_cast<T*>(storage_)->~T();
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

    // No copy/move - manage manually
    UninitArray(const UninitArray&) = delete;
    UninitArray& operator=(const UninitArray&) = delete;
    UninitArray(UninitArray&&) = delete;
    UninitArray& operator=(UninitArray&&) = delete;

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
