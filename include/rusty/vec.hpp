#ifndef RUSTY_VEC_HPP
#define RUSTY_VEC_HPP

#include <memory>
#include <algorithm>
#include <initializer_list>
#include <cassert>
#include <utility>  // for std::move, std::forward
#include <cstddef>  // for size_t
#include <cstring>  // for memcpy
#include <rusty/function.hpp>

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
    
    void grow() {
        size_t new_capacity = capacity_ == 0 ? 1 : capacity_ * 2;
        T* new_data = static_cast<T*>(::operator new(new_capacity * sizeof(T)));
        
        // Move existing elements
        for (size_t i = 0; i < size_; ++i) {
            new (&new_data[i]) T(std::move(data_[i]));
            data_[i].~T();
        }
        
        ::operator delete(data_);
        data_ = new_data;
        capacity_ = new_capacity;
    }
    
public:
    // Default constructor - empty vec
    Vec() : data_(nullptr), size_(0), capacity_(0) {}
    
    // Factory method - Vec::make()
    // @lifetime: owned
    static Vec<T> make() {
        return Vec<T>();
    }
    
    // Rust-idiomatic factory with capacity - Vec::with_capacity()
    // @lifetime: owned
    static Vec<T> with_capacity(size_t cap) {
        Vec<T> v;
        if (cap > 0) {
            v.data_ = static_cast<T*>(::operator new(cap * sizeof(T)));
            v.capacity_ = cap;
        }
        return v;
    }
    
    // Constructor with initial capacity (C++ style)
    explicit Vec(size_t initial_capacity) 
        : data_(nullptr), size_(0), capacity_(0) {
        if (initial_capacity > 0) {
            data_ = static_cast<T*>(::operator new(initial_capacity * sizeof(T)));
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
    Vec& operator=(Vec&& other) noexcept {
        if (this != &other) {
            // Clean up existing data
            clear();
            ::operator delete(data_);
            
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
    ~Vec() {
        clear();
        ::operator delete(data_);
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
    
    // Check if empty
    bool is_empty() const { return size_ == 0; }
    
    // Get capacity
    size_t capacity() const { return capacity_; }
    
    // Reserve capacity
    void reserve(size_t new_capacity) {
        if (new_capacity > capacity_) {
            T* new_data = static_cast<T*>(::operator new(new_capacity * sizeof(T)));
            
            // Move existing elements
            for (size_t i = 0; i < size_; ++i) {
                new (&new_data[i]) T(std::move(data_[i]));
                data_[i].~T();
            }
            
            ::operator delete(data_);
            data_ = new_data;
            capacity_ = new_capacity;
        }
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

    bool operator!=(const Vec& other) const {
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

} // namespace rusty

#endif // RUSTY_VEC_HPP