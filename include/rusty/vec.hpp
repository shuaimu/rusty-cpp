#ifndef RUSTY_VEC_HPP
#define RUSTY_VEC_HPP

#include <memory>
#include <algorithm>
#include <initializer_list>
#include <cassert>
#include <utility>  // for std::move, std::forward
#include <cstddef>  // for size_t
#include <cstdint>
#include <cstring>  // for memcpy
#include <limits>
#include <span>
#include <type_traits>
#include <rusty/alloc.hpp>
#include <rusty/function.hpp>
#include <rusty/mem.hpp>
#include <rusty/option.hpp>

// Vec<T> - A growable array with owned elements
// Equivalent to Rust's Vec<T>
//
// Guarantees:
// - Single ownership of the container
// - Elements are owned by the Vec
// - Automatic memory management
// - Move semantics for the container

// @safe
namespace rusty {

template<typename T>
class Vec {
private:
    T* data_;
    size_t size_;
    size_t capacity_;

    static constexpr bool can_materialize_capacity(size_t capacity) noexcept {
        return capacity <= std::numeric_limits<size_t>::max() / sizeof(T);
    }

    static constexpr rusty::alloc::Layout storage_layout(size_t capacity) noexcept {
        return rusty::alloc::Layout::from_size_align_unchecked(
            capacity * sizeof(T), alignof(T));
    }

    static T* allocate_storage(size_t capacity) {
        if (capacity == 0) {
            return nullptr;
        }
        if (!can_materialize_capacity(capacity)) {
            throw std::bad_alloc();
        }
        const auto layout = storage_layout(capacity);
        auto* bytes = rusty::alloc::alloc(layout);
        if (bytes == nullptr) {
            rusty::alloc::handle_alloc_error(layout);
        }
        return reinterpret_cast<T*>(bytes);
    }

    static void deallocate_storage(T* ptr, size_t capacity) noexcept {
        if (ptr == nullptr || capacity == 0 || !can_materialize_capacity(capacity)) {
            return;
        }
        rusty::alloc::dealloc(
            reinterpret_cast<std::uint8_t*>(ptr),
            storage_layout(capacity));
    }

    static constexpr size_t storage_byte_count(size_t capacity) noexcept {
        if (capacity > std::numeric_limits<size_t>::max() / sizeof(T)) {
            return std::numeric_limits<size_t>::max();
        }
        return capacity * sizeof(T);
    }

    void clear_forgotten_storage_marks() noexcept {
        if (data_ == nullptr || capacity_ == 0) {
            return;
        }
        rusty::mem::clear_forgotten_address_range(
            data_, storage_byte_count(capacity_));
    }
    
    void grow() {
        size_t new_capacity = capacity_ == 0 ? 1 : capacity_ * 2;
        T* new_data = allocate_storage(new_capacity);
        
        // Move existing elements
        for (size_t i = 0; i < size_; ++i) {
            new (&new_data[i]) T(std::move(data_[i]));
            data_[i].~T();
        }
        
        clear_forgotten_storage_marks();
        deallocate_storage(data_, capacity_);
        data_ = new_data;
        capacity_ = new_capacity;
    }
    
public:
    // Default constructor - empty vec
    Vec() : data_(nullptr), size_(0), capacity_(0) {}
    
    // Factory method - Vec::new_() (Rust's Vec::new, _ suffix because `new` is C++ keyword)
    // @lifetime: owned
    static Vec<T> new_() {
        return Vec<T>();
    }

    // Unsafe constructor from raw parts.
    // Caller must guarantee `ptr` points to `cap` contiguous `T` slots with
    // the first `len` elements fully initialized and uniquely owned.
    static Vec<T> from_raw_parts(T* ptr, size_t len, size_t cap) {
        assert(len <= cap);
        assert(ptr != nullptr || cap == 0);
        assert(can_materialize_capacity(cap));
        Vec<T> v;
        v.data_ = ptr;
        v.size_ = len;
        v.capacity_ = cap;
        return v;
    }

    template<typename U>
    static Vec<U> from_raw_parts(U* ptr, size_t len, size_t cap) {
        return Vec<U>::from_raw_parts(ptr, len, cap);
    }

    template<typename Alloc>
    static Vec<T> from_raw_parts_in(T* ptr, size_t len, size_t cap, Alloc&&) {
        return from_raw_parts(ptr, len, cap);
    }

    template<typename U, typename Alloc>
    static Vec<U> from_raw_parts_in(U* ptr, size_t len, size_t cap, Alloc&&) {
        return Vec<U>::from_raw_parts(ptr, len, cap);
    }

    template<typename Iter>
    static Vec<T> from_iter(Iter&& iter) {
        Vec<T> result;
        auto option_like_has_value = [](const auto& opt) {
            if constexpr (requires { opt.is_some(); }) {
                return opt.is_some();
            } else if constexpr (requires { opt.has_value(); }) {
                return opt.has_value();
            } else {
                return static_cast<bool>(opt);
            }
        };
        auto option_like_take_value = [](auto& opt) {
            if constexpr (requires { opt.unwrap(); }) {
                return opt.unwrap();
            } else if constexpr (requires { opt.has_value(); opt.reset(); }) {
                auto value = std::move(*opt);
                opt.reset();
                return value;
            } else {
                return std::move(*opt);
            }
        };
        auto normalize_item_for_vec = [](auto&& value) -> T {
            using Value = std::remove_cvref_t<decltype(value)>;
            if constexpr (std::is_pointer_v<Value> && !std::is_pointer_v<T>) {
                using Pointee = std::remove_cv_t<std::remove_pointer_t<Value>>;
                if constexpr (std::is_convertible_v<Pointee, T>) {
                    return static_cast<T>(*value);
                } else {
                    return T(*value);
                }
            } else if constexpr (requires { value.get(); } && !std::is_pointer_v<T>) {
                using GetType = std::remove_cvref_t<decltype(value.get())>;
                if constexpr (std::is_pointer_v<GetType>) {
                    using Pointee = std::remove_cv_t<std::remove_pointer_t<GetType>>;
                    if constexpr (std::is_convertible_v<Pointee, T>) {
                        return static_cast<T>(*value.get());
                    } else {
                        return T(*value.get());
                    }
                } else if constexpr (std::is_convertible_v<decltype(value.get()), T>) {
                    return static_cast<T>(value.get());
                } else {
                    return T(value.get());
                }
            } else if constexpr (std::is_convertible_v<decltype(value), T>) {
                return static_cast<T>(std::forward<decltype(value)>(value));
            } else {
                return T(std::forward<decltype(value)>(value));
            }
        };
        if constexpr (requires { std::forward<Iter>(iter).next(); }) {
            auto&& next_iter = std::forward<Iter>(iter);
            while (true) {
                auto item = next_iter.next();
                if (!option_like_has_value(item)) {
                    break;
                }
                result.push(normalize_item_for_vec(option_like_take_value(item)));
            }
        } else if constexpr (requires { std::forward<Iter>(iter).into_iter(); }) {
            return from_iter(std::forward<Iter>(iter).into_iter());
        } else {
            for (auto&& item : std::forward<Iter>(iter)) {
                result.push(normalize_item_for_vec(std::forward<decltype(item)>(item)));
            }
        }
        return result;
    }

    // Alias for backward compatibility
    static Vec<T> make() {
        return Vec<T>();
    }
    
    // Rust-idiomatic factory with capacity - Vec::with_capacity()
    // @lifetime: owned
    static Vec<T> with_capacity(size_t cap) {
        Vec<T> v;
        if (cap > 0) {
            v.data_ = allocate_storage(cap);
            v.capacity_ = cap;
        }
        return v;
    }
    
    // Constructor with initial capacity (C++ style)
    explicit Vec(size_t initial_capacity) 
        : data_(nullptr), size_(0), capacity_(0) {
        if (initial_capacity > 0) {
            data_ = allocate_storage(initial_capacity);
            capacity_ = initial_capacity;
        }
    }
    
    // Initializer list constructor
    Vec(std::initializer_list<T> init) 
        : data_(nullptr), size_(0), capacity_(0) {
        reserve(init.size());
        for (const T& item : init) {
            push(item);
        }
    }
    
    // No copy constructor - Vec cannot be copied
    Vec(const Vec&) = delete;
    Vec& operator=(const Vec&) = delete;
    
    // Move constructor
    Vec(Vec&& other) noexcept 
        : data_(other.data_), size_(other.size_), capacity_(other.capacity_) {
        other.data_ = nullptr;
        other.size_ = 0;
        other.capacity_ = 0;
    }
    
    // Move assignment
    Vec& operator=(Vec&& other) {
        if (this != &other) {
            // Clean up existing data
            clear();
            clear_forgotten_storage_marks();
            deallocate_storage(data_, capacity_);
            
            // Take ownership
            data_ = other.data_;
            size_ = other.size_;
            capacity_ = other.capacity_;
            
            other.data_ = nullptr;
            other.size_ = 0;
            other.capacity_ = 0;
        }
        return *this;
    }
    
    // Destructor
    ~Vec() noexcept(false) {
        clear();
        clear_forgotten_storage_marks();
        deallocate_storage(data_, capacity_);
    }
    
    // Push element to the back
    void push(T value) {
        if (size_ >= capacity_) {
            grow();
        }
        new (&data_[size_]) T(std::move(value));
        ++size_;
    }
    
    // Pop element from the back
    // Returns empty Option-like type if vec is empty
    T pop() {
        assert(size_ > 0);
        --size_;
        T result = std::move(data_[size_]);
        data_[size_].~T();
        return result;
    }
    
    // Access element by index
    // @lifetime: (&'a) -> &'a
    T& operator[](size_t index) {
        assert(index < size_);
        return data_[index];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& operator[](size_t index) const {
        assert(index < size_);
        return data_[index];
    }
    
    // Get first element
    // @lifetime: (&'a) -> &'a
    T& front() {
        assert(size_ > 0);
        return data_[0];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& front() const {
        assert(size_ > 0);
        return data_[0];
    }
    
    // Get last element
    // @lifetime: (&'a) -> &'a
    T& back() {
        assert(size_ > 0);
        return data_[size_ - 1];
    }
    
    // @lifetime: (&'a) -> &'a
    const T& back() const {
        assert(size_ > 0);
        return data_[size_ - 1];
    }
    
    // Get size
    size_t len() const { return size_; }
    size_t size() const { return size_; }

    // Unsafe-style length override (Rust Vec::set_len semantics).
    // Caller is responsible for initialization/drop invariants.
    void set_len(size_t new_len) {
        assert(new_len <= capacity_);
        size_ = new_len;
    }
    
    // Check if empty
    bool is_empty() const { return size_ == 0; }
    
    // Get capacity
    size_t capacity() const { return capacity_; }
    
    // Reserve capacity
    void reserve(size_t new_capacity) {
        if (new_capacity > capacity_) {
            T* new_data = allocate_storage(new_capacity);

            // Move existing elements
            for (size_t i = 0; i < size_; ++i) {
                new (&new_data[i]) T(std::move(data_[i]));
                data_[i].~T();
            }

            clear_forgotten_storage_marks();
            deallocate_storage(data_, capacity_);
            data_ = new_data;
            capacity_ = new_capacity;
        }
    }

    void reserve_exact(size_t additional) {
        reserve(size_ + additional);
    }
    
    // Clear all elements
    void clear() {
        for (size_t i = 0; i < size_; ++i) {
            data_[i].~T();
        }
        size_ = 0;
    }
    
    // Iterator support
    // @lifetime: (&'a) -> &'a
    T* begin() { return data_; }
    const T* begin() const { return data_; }

    // Slice-style data access used by shared iterator helpers.
    T* data() { return data_; }
    const T* data() const { return data_; }
    
    // @lifetime: (&'a) -> &'a
    T* end() { return data_ + size_; }
    const T* end() const { return data_ + size_; }
    
    // Clone the Vec (explicit deep copy)
    // @lifetime: owned
    Vec clone() const {
        Vec result = Vec::with_capacity(capacity_);
        for (size_t i = 0; i < size_; ++i) {
            result.push(data_[i]);  // Requires T to be copyable
        }
        return result;
    }
    
    // Equality comparison
    bool operator==(const Vec& other) const {
        if (size_ != other.size_) return false;
        for (size_t i = 0; i < size_; ++i) {
            if (!(data_[i] == other.data_[i])) return false;
        }
        return true;
    }

    template<typename U>
    bool operator==(const Vec<U>& other) const {
        if (size_ != other.len()) return false;
        for (size_t i = 0; i < size_; ++i) {
            if constexpr (requires(const T& lhs, const U& rhs) { lhs == rhs; }) {
                if (!(data_[i] == other[i])) return false;
            } else if constexpr (requires(const T& lhs, const U& rhs) { rhs == lhs; }) {
                if (!(other[i] == data_[i])) return false;
            } else if constexpr (std::is_empty_v<std::remove_cv_t<T>>
                                 && std::is_empty_v<std::remove_cv_t<U>>) {
                continue;
            } else {
                return false;
            }
        }
        return true;
    }

    bool operator!=(const Vec& other) const {
        return !(*this == other);
    }

    template<typename U>
    bool operator!=(const Vec<U>& other) const {
        return !(*this == other);
    }

    // Retain only elements where predicate returns true
    // Similar to Rust's Vec::retain
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    void retain(Function<bool(const T&)> predicate) {
        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            if (predicate(static_cast<const T&>(data_[read]))) {
                if (write != read) {
                    // Move element to new position
                    new (&data_[write]) T(std::move(data_[read]));
                    data_[read].~T();
                }
                ++write;
            } else {
                // Destroy element that doesn't match predicate
                data_[read].~T();
            }
        }
        size_ = write;
    }

    // Extract elements where predicate returns true, removing them from this Vec
    // Similar to Rust's Vec::extract_if (formerly drain_filter)
    // Returns a new Vec containing the extracted elements
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    Vec<T> extract_if(Function<bool(const T&)> predicate) {
        Vec<T> extracted = Vec<T>::with_capacity(size_ / 2);  // Reasonable initial guess
        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            if (predicate(static_cast<const T&>(data_[read]))) {
                // Move to extracted Vec
                extracted.push(std::move(data_[read]));
                data_[read].~T();
            } else {
                // Keep in this Vec
                if (write != read) {
                    new (&data_[write]) T(std::move(data_[read]));
                    data_[read].~T();
                }
                ++write;
            }
        }
        size_ = write;
        return extracted;
    }
};

// Helper function to create a Vec
template<typename T>
// @lifetime: owned
Vec<T> vec_of(std::initializer_list<T> init) {
    return Vec<T>(init);
}

template<typename T>
void vec_extend_from_slice(Vec<T>& self, std::span<const T> other) {
    self.reserve(self.size() + other.size());
    for (const auto& item : other) {
        if constexpr (requires { item.clone(); }) {
            self.push(item.clone());
        } else {
            self.push(item);
        }
    }
}

} // namespace rusty

#endif // RUSTY_VEC_HPP
