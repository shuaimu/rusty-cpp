#ifndef RUSTY_ARC_HPP
#define RUSTY_ARC_HPP

#include <atomic>
#include <cassert>
#include <cstddef>
#include <utility>
#include "option.hpp"  // For Option<T&> and SomeRef()

// Arc<T> - Atomically Reference Counted pointer
// Equivalent to Rust's Arc<T>
//
// Guarantees:
// - Thread-safe reference counting
// - Shared ownership across threads
// - Automatic deallocation when last Arc is dropped
// - Immutable access only (use Mutex/RwLock for mutation)

// @safe
namespace rusty {

// Forward declarations for weak references
template<typename T> class Arc;
namespace sync {
    template<typename T> class Weak;
    template<typename T> Weak<T> downgrade(const Arc<T>&);
}
template<typename T> sync::Weak<T> downgrade(const Arc<T>&);

// @unsafe - Raw pointer operations and atomic reference counting
template<typename T>
class Arc {
private:
    // Allow other Arc instantiations to access private members for conversion
    template<typename U>
    friend class Arc;
    template<typename U>
    friend class sync::Weak;
    template<typename U>
    friend sync::Weak<U> sync::downgrade(const Arc<U>&);
    template<typename U>
    friend sync::Weak<U> downgrade(const Arc<U>&);

    struct ControlBlock {
        T* value;
        std::atomic<size_t> strong_count;
        std::atomic<size_t> weak_count;

        template<typename... Args>
        ControlBlock(Args&&... args)
            : value(new T(std::forward<Args>(args)...)),
              strong_count(1),
              weak_count(1) {}

        ~ControlBlock() {
            delete value;
        }
    };

    ControlBlock* ptr;

    // @unsafe
    static void release_weak(ControlBlock* cb) {
        // @unsafe {
        if (cb && cb->weak_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
            std::atomic_thread_fence(std::memory_order_acquire);
            delete cb;
        }
        // }
    }

    // @unsafe
    void increment() {
        // @unsafe {
        if (ptr) {
            ptr->strong_count.fetch_add(1, std::memory_order_relaxed);
        }
        // }
    }

    // @unsafe
    void decrement() {
        // @unsafe {
        if (!ptr) {
            return;
        }
        ControlBlock* current = ptr;
        if (current->strong_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
            std::atomic_thread_fence(std::memory_order_acquire);
            delete current->value;
            current->value = nullptr;
            release_weak(current);
        }
        ptr = nullptr;
        // }
    }

public:
    // No default constructor - Arc must always own a value (Rust-idiomatic)
    // Use Option<Arc<T>> for nullable Arc
    Arc() = delete;

    // Rust-idiomatic factory method - Arc::new()
    // @lifetime: owned
    static Arc<T> new_(T value) {
        return Arc<T>(new ControlBlock(std::move(value)));
    }

    // Factory method for in-place construction with arguments
    // @safe - Public API is safe, internal allocation is encapsulated
    // @lifetime: owned
    template<typename... Args>
    static Arc<T> make(Args&&... args) {
        // @unsafe - new allocation
        { return Arc<T>(new ControlBlock(std::forward<Args>(args)...)); }
    }

    // Private constructor from control block
    explicit Arc(ControlBlock* p) : ptr(p) {}

    // Private constructor for weak upgrade (with increment flag)
    Arc(ControlBlock* p, bool increment_strong) : ptr(p) {
        if (increment_strong && ptr) {
            ptr->strong_count.fetch_add(1, std::memory_order_relaxed);
        }
    }

    // @safe - Copy constructor increases reference count
    Arc(const Arc& other) : ptr(other.ptr) {
        increment();
    }

    // @safe - Conversion constructor for polymorphism (Arc<Derived> → Arc<Base>)
    // Enables upcasting from derived to base types
    template<typename U, typename = typename std::enable_if<std::is_convertible<U*, T*>::value>::type>
    Arc(const Arc<U>& other) : ptr(reinterpret_cast<ControlBlock*>(other.ptr)) {
        increment();
    }

    // @safe - Move constructor with no ref count change
    Arc(Arc&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // @unsafe - Copy assignment with proper ref counting
    Arc& operator=(const Arc& other) {
        // @unsafe {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            increment();
        }
        return *this;
        // }
    }

    // @unsafe - Move assignment with proper cleanup
    Arc& operator=(Arc&& other) noexcept {
        // @unsafe {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
        // }
    }

    // Destructor
    ~Arc() {
        decrement();
    }

    // Dereference - get immutable reference
    // @safe - safe to call, internal unsafe operations are encapsulated
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        // @unsafe
        { assert(ptr != nullptr && ptr->value != nullptr); }
        // @unsafe
        { return *ptr->value; }
    }

    // Arrow operator - access members
    // @safe - safe to call, internal unsafe operations are encapsulated
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        // @unsafe
        { assert(ptr != nullptr && ptr->value != nullptr); }
        // @unsafe
        { return ptr->value; }
    }

    // Get raw pointer
    // @safe - safe to call, returns const pointer
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        // @unsafe
        { return (ptr && ptr->value) ? ptr->value : nullptr; }
    }

    // @safe - Check if Arc contains a value
    bool is_valid() const {
        // @unsafe
        { return ptr != nullptr && ptr->value != nullptr; }
    }

    // @safe - Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }

    // @safe - Get current reference count
    size_t strong_count() const {
        // @unsafe
        { return ptr ? ptr->strong_count.load(std::memory_order_relaxed) : 0; }
    }

    // @safe - Get weak count excluding implicit strong-held weak
    size_t weak_count() const {
        // @unsafe
        {
            if (!ptr) {
                return 0;
            }
            size_t count = ptr->weak_count.load(std::memory_order_relaxed);
            return count > 0 ? count - 1 : 0;
        }
    }

    // @safe - Clone - explicitly create a new Arc to the same value
    Arc clone() const {
        return Arc(*this);
    }

    // Try to get mutable reference if we're the only owner
    // Returns None if there are other references (shared state)
    // @safe
    // @lifetime: (&'a mut self) -> Option<&'a mut T>
    Option<T&> get_mut() {
        if (ptr && ptr->value && ptr->strong_count.load(std::memory_order_relaxed) == 1) {
            return SomeRef(*ptr->value);
        }
        return None;
    }

    // Get raw pointer to the value (for legacy code)
    // Unlike get_mut(), this works even with multiple references
    // @unsafe - Returns mutable pointer to potentially shared data
    // @lifetime: (&'a self) -> *'a T
    T* as_ptr() const {
        return ptr ? ptr->value : nullptr;
    }
};

// @safe - Rust-idiomatic factory function
template<typename T, typename... Args>
// @lifetime: owned
Arc<T> arc(Args&&... args) {
    return Arc<T>::new_(T(std::forward<Args>(args)...));
}

// @safe - C++-friendly factory function (kept for compatibility)
template<typename T, typename... Args>
// @lifetime: owned
Arc<T> make_arc(Args&&... args) {
    return Arc<T>::make(T(std::forward<Args>(args)...));
}

// @safe - Comparison operators for Arc<T> (needed for std::set and std::map)
template<typename T>
bool operator<(const Arc<T>& lhs, const Arc<T>& rhs) {
    return lhs.get() < rhs.get();
}

// @safe
template<typename T>
bool operator==(const Arc<T>& lhs, const Arc<T>& rhs) {
    return lhs.get() == rhs.get();
}

// @safe
template<typename T>
bool operator!=(const Arc<T>& lhs, const Arc<T>& rhs) {
    return !(lhs == rhs);
}

// @safe
template<typename T>
bool operator<=(const Arc<T>& lhs, const Arc<T>& rhs) {
    return !(rhs < lhs);
}

// @safe
template<typename T>
bool operator>(const Arc<T>& lhs, const Arc<T>& rhs) {
    return rhs < lhs;
}

// @safe
template<typename T>
bool operator>=(const Arc<T>& lhs, const Arc<T>& rhs) {
    return !(lhs < rhs);
}

} // namespace rusty

#endif // RUSTY_ARC_HPP
