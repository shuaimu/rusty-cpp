#ifndef RUSTY_ARC_HPP
#define RUSTY_ARC_HPP

#include <atomic>
#include <cassert>
#include <cstddef>  // for size_t
#include <utility>  // for std::move, std::forward

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
template<typename T> class ArcWeak;
namespace sync { template<typename T> class Weak; }
template<typename T> ArcWeak<T> downgrade(const Arc<T>&);

template<typename T>
class Arc {
private:
    template<typename U>
    friend class ArcWeak;
    template<typename U>
    friend class sync::Weak;
    template<typename U>
    friend ArcWeak<U> downgrade(const Arc<U>&);
    struct ControlBlock {
        T value;
        std::atomic<size_t> ref_count;
        
        template<typename... Args>
        ControlBlock(Args&&... args) 
            : value(std::forward<Args>(args)...), ref_count(1) {}
    };
    
    ControlBlock* ptr;
    
    void increment() {
        if (ptr) {
            ptr->ref_count.fetch_add(1, std::memory_order_relaxed);
        }
    }
    
    void decrement() {
        if (ptr) {
            if (ptr->ref_count.fetch_sub(1, std::memory_order_acq_rel) == 1) {
                // We were the last reference
                std::atomic_thread_fence(std::memory_order_acquire);
                delete ptr;
            }
        }
    }
    
public:
    // Default constructor - creates empty Arc
    Arc() : ptr(nullptr) {}
    
    // Rust-idiomatic factory method - Arc::new()
    // @lifetime: owned
    static Arc<T> new_(T value) {
        return Arc<T>(new ControlBlock(std::move(value)));
    }
    
    // C++-friendly factory method (kept for compatibility)
    // @lifetime: owned
    static Arc<T> make(T value) {
        return Arc<T>(new ControlBlock(std::move(value)));
    }
    
    // Factory method for in-place construction with arguments
    // @lifetime: owned
    template<typename... Args>
    static Arc<T> make_in_place(Args&&... args) {
        return Arc<T>(new ControlBlock(std::forward<Args>(args)...));
    }
    
    // Private constructor from control block
    explicit Arc(ControlBlock* p) : ptr(p) {}
    
    // Private constructor for weak upgrade (with increment flag)
    Arc(ControlBlock* p, bool increment) : ptr(p) {
        if (increment && ptr) {
            ptr->ref_count.fetch_add(1, std::memory_order_relaxed);
        }
    }
    
    // Copy constructor - increases reference count
    Arc(const Arc& other) : ptr(other.ptr) {
        increment();
    }
    
    // Move constructor - no ref count change
    Arc(Arc&& other) noexcept : ptr(other.ptr) {
        other.ptr = nullptr;
    }
    
    // Copy assignment
    Arc& operator=(const Arc& other) {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            increment();
        }
        return *this;
    }
    
    // Move assignment
    Arc& operator=(Arc&& other) noexcept {
        if (this != &other) {
            decrement();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }
    
    // Destructor
    ~Arc() {
        decrement();
    }
    
    // Dereference - get immutable reference
    // @lifetime: (&'a) -> &'a
    const T& operator*() const {
        assert(ptr != nullptr);
        return ptr->value;
    }
    
    // Arrow operator - access members
    // @lifetime: (&'a) -> &'a
    const T* operator->() const {
        assert(ptr != nullptr);
        return &ptr->value;
    }
    
    // Get raw pointer
    // @lifetime: (&'a) -> &'a
    const T* get() const {
        return ptr ? &ptr->value : nullptr;
    }
    
    // Check if Arc contains a value
    bool is_valid() const {
        return ptr != nullptr;
    }
    
    // Explicit bool conversion
    explicit operator bool() const {
        return is_valid();
    }
    
    // Get current reference count
    size_t strong_count() const {
        return ptr ? ptr->ref_count.load(std::memory_order_relaxed) : 0;
    }
    
    // Clone - explicitly create a new Arc to the same value
    Arc clone() const {
        return Arc(*this);
    }
    
    // Try to get mutable reference if we're the only owner
    // Returns nullptr if there are other references
    // @lifetime: (&'a mut) -> &'a mut
    T* get_mut() {
        if (ptr && ptr->ref_count.load(std::memory_order_relaxed) == 1) {
            return &ptr->value;
        }
        return nullptr;
    }
};

// Rust-idiomatic factory function
template<typename T, typename... Args>
// @lifetime: owned
Arc<T> arc(Args&&... args) {
    return Arc<T>::new_(T(std::forward<Args>(args)...));
}

// C++-friendly factory function (kept for compatibility)
template<typename T, typename... Args>
// @lifetime: owned
Arc<T> make_arc(Args&&... args) {
    return Arc<T>::make(T(std::forward<Args>(args)...));
}

} // namespace rusty

#endif // RUSTY_ARC_HPP