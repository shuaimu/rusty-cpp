#ifndef RUSTY_VECDEQUE_HPP
#define RUSTY_VECDEQUE_HPP

#include <memory>
#include <algorithm>
#include <initializer_list>
#include <cassert>
#include <utility>  // for std::move, std::forward
#include <cstddef>  // for size_t
#include <rusty/function.hpp>

// VecDeque<T> - A double-ended queue implemented with a growable ring buffer
// Equivalent to Rust's VecDeque<T>
//
// This is a sequence container that allows efficient insertion and removal
// at both ends. Unlike Vec, it uses a ring buffer internally, so push_front
// is O(1) amortized instead of O(n).
//
// Guarantees:
// - Single ownership of the container
// - Elements are owned by the VecDeque
// - Automatic memory management
// - Move semantics for the container
// - O(1) amortized push/pop at both front and back

// @safe
namespace rusty {

template<typename T>
class VecDeque {
private:
    T* data_;
    size_t head_;      // Index of the first element
    size_t size_;      // Number of elements
    size_t capacity_;  // Total capacity of the ring buffer

    // Wrap index around the ring buffer
    size_t wrap_index(size_t index) const {
        if (capacity_ == 0) return 0;
        return index % capacity_;
    }

    // Get the physical index for a logical index
    size_t to_physical_index(size_t logical_index) const {
        return wrap_index(head_ + logical_index);
    }

    // Grow the ring buffer to at least new_capacity
    void grow_to(size_t new_capacity) {
        if (new_capacity <= capacity_) return;

        T* new_data = static_cast<T*>(::operator new(new_capacity * sizeof(T)));

        // Move existing elements to new buffer (linearizing them)
        for (size_t i = 0; i < size_; ++i) {
            size_t old_idx = to_physical_index(i);
            new (&new_data[i]) T(std::move(data_[old_idx]));
            data_[old_idx].~T();
        }

        ::operator delete(data_);
        data_ = new_data;
        head_ = 0;
        capacity_ = new_capacity;
    }

    // Grow by doubling capacity
    void grow() {
        size_t new_capacity = capacity_ == 0 ? 4 : capacity_ * 2;
        grow_to(new_capacity);
    }

public:
    // ========================================================================
    // Constructors and Destructors
    // ========================================================================

    // Default constructor - empty deque
    VecDeque() : data_(nullptr), head_(0), size_(0), capacity_(0) {}

    // Factory method - VecDeque::make()
    // @lifetime: owned
    static VecDeque<T> make() {
        return VecDeque<T>();
    }

    // Factory with capacity - VecDeque::with_capacity()
    // @lifetime: owned
    static VecDeque<T> with_capacity(size_t cap) {
        VecDeque<T> d;
        if (cap > 0) {
            d.data_ = static_cast<T*>(::operator new(cap * sizeof(T)));
            d.capacity_ = cap;
        }
        return d;
    }

    // Constructor with initial capacity (C++ style)
    explicit VecDeque(size_t initial_capacity)
        : data_(nullptr), head_(0), size_(0), capacity_(0) {
        if (initial_capacity > 0) {
            data_ = static_cast<T*>(::operator new(initial_capacity * sizeof(T)));
            capacity_ = initial_capacity;
        }
    }

    // Initializer list constructor
    VecDeque(std::initializer_list<T> init)
        : data_(nullptr), head_(0), size_(0), capacity_(0) {
        reserve(init.size());
        for (const T& item : init) {
            push_back(item);
        }
    }

    // No copy constructor - VecDeque cannot be copied
    VecDeque(const VecDeque&) = delete;
    VecDeque& operator=(const VecDeque&) = delete;

    // Move constructor
    VecDeque(VecDeque&& other) noexcept
        : data_(other.data_), head_(other.head_),
          size_(other.size_), capacity_(other.capacity_) {
        other.data_ = nullptr;
        other.head_ = 0;
        other.size_ = 0;
        other.capacity_ = 0;
    }

    // Move assignment
    VecDeque& operator=(VecDeque&& other) noexcept {
        if (this != &other) {
            // Clean up existing data
            clear();
            ::operator delete(data_);

            // Take ownership
            data_ = other.data_;
            head_ = other.head_;
            size_ = other.size_;
            capacity_ = other.capacity_;

            other.data_ = nullptr;
            other.head_ = 0;
            other.size_ = 0;
            other.capacity_ = 0;
        }
        return *this;
    }

    // Destructor
    ~VecDeque() {
        clear();
        ::operator delete(data_);
    }

    // ========================================================================
    // Push operations
    // ========================================================================

    // Push element to the back
    void push_back(T value) {
        if (size_ >= capacity_) {
            grow();
        }
        size_t idx = to_physical_index(size_);
        new (&data_[idx]) T(std::move(value));
        ++size_;
    }

    // Push element to the front
    void push_front(T value) {
        if (size_ >= capacity_) {
            grow();
        }
        // Move head backwards (wrapping around)
        head_ = wrap_index(head_ + capacity_ - 1);
        new (&data_[head_]) T(std::move(value));
        ++size_;
    }

    // ========================================================================
    // Pop operations
    // ========================================================================

    // Pop element from the back
    T pop_back() {
        assert(size_ > 0 && "pop_back on empty VecDeque");
        --size_;
        size_t idx = to_physical_index(size_);
        T result = std::move(data_[idx]);
        data_[idx].~T();
        return result;
    }

    // Pop element from the front
    T pop_front() {
        assert(size_ > 0 && "pop_front on empty VecDeque");
        T result = std::move(data_[head_]);
        data_[head_].~T();
        head_ = wrap_index(head_ + 1);
        --size_;
        return result;
    }

    // ========================================================================
    // Element access
    // ========================================================================

    // Access element by index
    // @lifetime: (&'a) -> &'a
    T& operator[](size_t index) {
        assert(index < size_ && "VecDeque index out of bounds");
        return data_[to_physical_index(index)];
    }

    // @lifetime: (&'a) -> &'a
    const T& operator[](size_t index) const {
        assert(index < size_ && "VecDeque index out of bounds");
        return data_[to_physical_index(index)];
    }

    // Get element by index (same as operator[], Rust-style)
    // @lifetime: (&'a) -> &'a
    T& get(size_t index) {
        assert(index < size_ && "VecDeque::get index out of bounds");
        return data_[to_physical_index(index)];
    }

    // @lifetime: (&'a) -> &'a
    const T& get(size_t index) const {
        assert(index < size_ && "VecDeque::get index out of bounds");
        return data_[to_physical_index(index)];
    }

    // Get first element
    // @lifetime: (&'a) -> &'a
    T& front() {
        assert(size_ > 0 && "front on empty VecDeque");
        return data_[head_];
    }

    // @lifetime: (&'a) -> &'a
    const T& front() const {
        assert(size_ > 0 && "front on empty VecDeque");
        return data_[head_];
    }

    // Get last element
    // @lifetime: (&'a) -> &'a
    T& back() {
        assert(size_ > 0 && "back on empty VecDeque");
        return data_[to_physical_index(size_ - 1)];
    }

    // @lifetime: (&'a) -> &'a
    const T& back() const {
        assert(size_ > 0 && "back on empty VecDeque");
        return data_[to_physical_index(size_ - 1)];
    }

    // ========================================================================
    // Capacity operations
    // ========================================================================

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
            grow_to(new_capacity);
        }
    }

    // Shrink to fit - reduce capacity to match size
    void shrink_to_fit() {
        if (size_ < capacity_) {
            if (size_ == 0) {
                ::operator delete(data_);
                data_ = nullptr;
                capacity_ = 0;
                head_ = 0;
            } else {
                // Allocate new smaller buffer and move elements
                T* new_data = static_cast<T*>(::operator new(size_ * sizeof(T)));
                for (size_t i = 0; i < size_; ++i) {
                    size_t old_idx = to_physical_index(i);
                    new (&new_data[i]) T(std::move(data_[old_idx]));
                    data_[old_idx].~T();
                }
                ::operator delete(data_);
                data_ = new_data;
                head_ = 0;
                capacity_ = size_;
            }
        }
    }

    // ========================================================================
    // Modification operations
    // ========================================================================

    // Clear all elements
    void clear() {
        for (size_t i = 0; i < size_; ++i) {
            size_t idx = to_physical_index(i);
            data_[idx].~T();
        }
        size_ = 0;
        head_ = 0;
    }

    // Swap two elements
    void swap(size_t i, size_t j) {
        assert(i < size_ && j < size_ && "VecDeque::swap index out of bounds");
        if (i != j) {
            std::swap(data_[to_physical_index(i)], data_[to_physical_index(j)]);
        }
    }

    // Rotate left by mid positions (elements 0..mid go to end)
    void rotate_left(size_t mid) {
        if (mid == 0 || mid >= size_) return;
        // Simple implementation: adjust head
        head_ = wrap_index(head_ + mid);
    }

    // Rotate right by k positions (elements from end go to front)
    void rotate_right(size_t k) {
        if (k == 0 || k >= size_) return;
        head_ = wrap_index(head_ + capacity_ - k);
    }

    // Make contiguous - ensure all elements are in a contiguous slice
    // Returns pointer to the first element
    T* make_contiguous() {
        if (size_ == 0) return nullptr;

        // Check if already contiguous
        if (head_ + size_ <= capacity_) {
            return &data_[head_];
        }

        // Need to rearrange - allocate new buffer with same capacity and linearize
        T* new_data = static_cast<T*>(::operator new(capacity_ * sizeof(T)));
        for (size_t i = 0; i < size_; ++i) {
            size_t old_idx = to_physical_index(i);
            new (&new_data[i]) T(std::move(data_[old_idx]));
            data_[old_idx].~T();
        }
        ::operator delete(data_);
        data_ = new_data;
        head_ = 0;
        return data_;
    }

    // ========================================================================
    // Iterator support
    // ========================================================================

    // Simple iterator class for VecDeque
    class iterator {
    private:
        VecDeque<T>* deque_;
        size_t index_;

    public:
        using iterator_category = std::random_access_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = T*;
        using reference = T&;

        iterator(VecDeque<T>* d, size_t i) : deque_(d), index_(i) {}

        reference operator*() { return (*deque_)[index_]; }
        pointer operator->() { return &(*deque_)[index_]; }

        iterator& operator++() { ++index_; return *this; }
        iterator operator++(int) { iterator tmp = *this; ++index_; return tmp; }
        iterator& operator--() { --index_; return *this; }
        iterator operator--(int) { iterator tmp = *this; --index_; return tmp; }

        iterator& operator+=(difference_type n) { index_ += n; return *this; }
        iterator& operator-=(difference_type n) { index_ -= n; return *this; }

        iterator operator+(difference_type n) const { return iterator(deque_, index_ + n); }
        iterator operator-(difference_type n) const { return iterator(deque_, index_ - n); }
        difference_type operator-(const iterator& other) const {
            return static_cast<difference_type>(index_) - static_cast<difference_type>(other.index_);
        }

        reference operator[](difference_type n) { return (*deque_)[index_ + n]; }

        bool operator==(const iterator& other) const { return index_ == other.index_; }
        bool operator!=(const iterator& other) const { return index_ != other.index_; }
        bool operator<(const iterator& other) const { return index_ < other.index_; }
        bool operator>(const iterator& other) const { return index_ > other.index_; }
        bool operator<=(const iterator& other) const { return index_ <= other.index_; }
        bool operator>=(const iterator& other) const { return index_ >= other.index_; }
    };

    class const_iterator {
    private:
        const VecDeque<T>* deque_;
        size_t index_;

    public:
        using iterator_category = std::random_access_iterator_tag;
        using value_type = T;
        using difference_type = std::ptrdiff_t;
        using pointer = const T*;
        using reference = const T&;

        const_iterator(const VecDeque<T>* d, size_t i) : deque_(d), index_(i) {}

        reference operator*() const { return (*deque_)[index_]; }
        pointer operator->() const { return &(*deque_)[index_]; }

        const_iterator& operator++() { ++index_; return *this; }
        const_iterator operator++(int) { const_iterator tmp = *this; ++index_; return tmp; }
        const_iterator& operator--() { --index_; return *this; }
        const_iterator operator--(int) { const_iterator tmp = *this; --index_; return tmp; }

        const_iterator& operator+=(difference_type n) { index_ += n; return *this; }
        const_iterator& operator-=(difference_type n) { index_ -= n; return *this; }

        const_iterator operator+(difference_type n) const { return const_iterator(deque_, index_ + n); }
        const_iterator operator-(difference_type n) const { return const_iterator(deque_, index_ - n); }
        difference_type operator-(const const_iterator& other) const {
            return static_cast<difference_type>(index_) - static_cast<difference_type>(other.index_);
        }

        reference operator[](difference_type n) const { return (*deque_)[index_ + n]; }

        bool operator==(const const_iterator& other) const { return index_ == other.index_; }
        bool operator!=(const const_iterator& other) const { return index_ != other.index_; }
        bool operator<(const const_iterator& other) const { return index_ < other.index_; }
        bool operator>(const const_iterator& other) const { return index_ > other.index_; }
        bool operator<=(const const_iterator& other) const { return index_ <= other.index_; }
        bool operator>=(const const_iterator& other) const { return index_ >= other.index_; }
    };

    // @lifetime: (&'a) -> &'a
    iterator begin() { return iterator(this, 0); }
    const_iterator begin() const { return const_iterator(this, 0); }
    const_iterator cbegin() const { return const_iterator(this, 0); }

    // @lifetime: (&'a) -> &'a
    iterator end() { return iterator(this, size_); }
    const_iterator end() const { return const_iterator(this, size_); }
    const_iterator cend() const { return const_iterator(this, size_); }

    // ========================================================================
    // Utility operations
    // ========================================================================

    // Clone the VecDeque (explicit deep copy)
    // @lifetime: owned
    VecDeque clone() const {
        VecDeque result = VecDeque::with_capacity(capacity_);
        for (size_t i = 0; i < size_; ++i) {
            result.push_back(data_[to_physical_index(i)]);  // Requires T to be copyable
        }
        return result;
    }

    // Append all elements from another VecDeque (consuming it)
    void append(VecDeque&& other) {
        reserve(size_ + other.size_);
        for (size_t i = 0; i < other.size_; ++i) {
            push_back(std::move(other[i]));
        }
        other.clear();
    }

    // Equality comparison
    bool operator==(const VecDeque& other) const {
        if (size_ != other.size_) return false;
        for (size_t i = 0; i < size_; ++i) {
            if (!((*this)[i] == other[i])) return false;
        }
        return true;
    }

    bool operator!=(const VecDeque& other) const {
        return !(*this == other);
    }

    // Retain only elements where predicate returns true
    // Similar to Rust's VecDeque::retain
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    void retain(Function<bool(const T&)> predicate) {
        // Make contiguous first for simpler logic
        make_contiguous();

        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            size_t phys = to_physical_index(read);
            if (predicate(static_cast<const T&>(data_[phys]))) {
                if (write != read) {
                    size_t write_phys = to_physical_index(write);
                    new (&data_[write_phys]) T(std::move(data_[phys]));
                    data_[phys].~T();
                }
                ++write;
            } else {
                data_[phys].~T();
            }
        }
        size_ = write;
    }

    // Extract elements where predicate returns true, removing them from this VecDeque
    // Similar to Rust's VecDeque::extract_if
    // Returns a new VecDeque containing the extracted elements
    // Uses rusty::Function for type-erased, move-only callable (no ref captures)
    // Predicate signature: bool(const T&) - takes immutable borrow of element
    VecDeque<T> extract_if(Function<bool(const T&)> predicate) {
        VecDeque<T> extracted = VecDeque<T>::with_capacity(size_ / 2);

        // Make contiguous for simpler logic
        make_contiguous();

        size_t write = 0;
        for (size_t read = 0; read < size_; ++read) {
            size_t phys = to_physical_index(read);
            if (predicate(static_cast<const T&>(data_[phys]))) {
                extracted.push_back(std::move(data_[phys]));
                data_[phys].~T();
            } else {
                if (write != read) {
                    size_t write_phys = to_physical_index(write);
                    new (&data_[write_phys]) T(std::move(data_[phys]));
                    data_[phys].~T();
                }
                ++write;
            }
        }
        size_ = write;
        return extracted;
    }

    // ========================================================================
    // Conversion operations
    // ========================================================================

    // Convert to Vec (consumes the VecDeque)
    // Note: This is a method, not a conversion operator, to be explicit
    // @lifetime: owned
    // Vec<T> into_vec() && {
    //     make_contiguous();
    //     // ... would need Vec header
    // }

    // Check if the internal buffer is contiguous
    bool is_contiguous() const {
        return size_ == 0 || head_ + size_ <= capacity_;
    }
};

// Helper function to create a VecDeque
template<typename T>
// @lifetime: owned
VecDeque<T> vecdeque_of(std::initializer_list<T> init) {
    return VecDeque<T>(init);
}

} // namespace rusty

#endif // RUSTY_VECDEQUE_HPP
