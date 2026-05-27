#ifndef RUSTY_STD_MINIMAL_HPP
#define RUSTY_STD_MINIMAL_HPP

// Minimal std:: declarations for rusty types
// This allows our types to compile without full std library

namespace std {
    
    // Forward declarations for move semantics
    template<typename T>
    T&& move(T& t) noexcept {
        return static_cast<T&&>(t);
    }
    
    template<typename T>
    T&& forward(T& t) noexcept {
        return static_cast<T&&>(t);
    }
    
    // Memory fence for atomic operations
    enum memory_order {
        memory_order_relaxed,
        memory_order_acquire,
        memory_order_release,
        memory_order_acq_rel,
        memory_order_seq_cst
    };
    
    inline void atomic_thread_fence(memory_order) {}
    
    // Minimal atomic for Arc
    template<typename T>
    class atomic {
        T value;
    public:
        atomic(T v = T()) : value(v) {}
        
        T load(memory_order = memory_order_seq_cst) const {
            return value;
        }
        
        void store(T v, memory_order = memory_order_seq_cst) {
            value = v;
        }
        
        T fetch_add(T v, memory_order = memory_order_seq_cst) {
            T old = value;
            value += v;
            return old;
        }
        
        T fetch_sub(T v, memory_order = memory_order_seq_cst) {
            T old = value;
            value -= v;
            return old;
        }
    };
    
    // Minimal variant for Result (simplified)
    template<typename T, typename E>
    class variant {
        union {
            T t_val;
            E e_val;
        };
        bool is_t;
        
    public:
        variant() : is_t(true) {}
        variant(T t) : t_val(t), is_t(true) {}
        variant(E e) : e_val(e), is_t(false) {}
        
        variant& operator=(T t) {
            if (!is_t && sizeof(E) > 0) e_val.~E();
            t_val = t;
            is_t = true;
            return *this;
        }
        
        variant& operator=(E e) {
            if (is_t && sizeof(T) > 0) t_val.~T();
            e_val = e;
            is_t = false;
            return *this;
        }
    };
    
    template<typename T>
    T& get(variant<T, auto>& v) {
        return v.t_val;  // Simplified, no checking
    }
    
    template<typename E>
    E& get(variant<auto, E>& v) {
        return v.e_val;  // Simplified, no checking
    }
    
    // initializer_list for VecLegacy
    template<typename T>
    class initializer_list {
        const T* ptr;
        size_t len;
        
    public:
        initializer_list() : ptr(nullptr), len(0) {}
        initializer_list(const T* p, size_t l) : ptr(p), len(l) {}
        
        const T* begin() const { return ptr; }
        const T* end() const { return ptr + len; }
        size_t size() const { return len; }
    };
}

#endif // RUSTY_STD_MINIMAL_HPP