#ifndef RUSTY_SYNC_WEAK_HPP
#define RUSTY_SYNC_WEAK_HPP

#include <atomic>

#include "../option.hpp"
#include "../arc.hpp"

namespace rusty {
namespace sync {

template<typename T>
class Weak {
private:
    friend class rusty::Arc<T>;

    typename rusty::Arc<T>::ControlBlock* ptr;

    Weak(typename rusty::Arc<T>::ControlBlock* p, bool add_ref)
        : ptr(p) {
        if (ptr && add_ref) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
        }
    }

public:
    // @safe - Default ctor; sets the raw control-block pointer to null.
    Weak() : ptr(nullptr) {}

    // @safe - Borrow control block from a strong Arc and bump weak_count.
    // The raw ControlBlock* deref + atomic increment are quarantined in
    // the inner @unsafe block.
    explicit Weak(const rusty::Arc<T>& arc)
        : ptr(arc.ptr) {
        // @unsafe { raw ControlBlock* deref + std::atomic::fetch_add }
        {
            if (ptr) {
                ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
            }
        }
    }

    // @safe - Copy ctor: share control block + bump weak_count.
    Weak(const Weak& other)
        : ptr(other.ptr) {
        // @unsafe { raw ControlBlock* deref + std::atomic::fetch_add }
        {
            if (ptr) {
                ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
            }
        }
    }

    // @safe - Move ctor: transfer the raw pointer; no atomic work.
    Weak(Weak&& other) noexcept
        : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    // @safe - Trivial dtor; reset() carries its own quarantine.
    ~Weak() {
        reset();
    }

    // @safe - Copy assignment: reset + share control block + bump.
    Weak& operator=(const Weak& other) {
        if (this != &other) {
            reset();
            ptr = other.ptr;
            // @unsafe { raw ControlBlock* deref + std::atomic::fetch_add }
            {
                if (ptr) {
                    ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
                }
            }
        }
        return *this;
    }

    // @safe - Move assignment: pure pointer transfer.
    Weak& operator=(Weak&& other) noexcept {
        if (this != &other) {
            reset();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }

    // @safe - Re-bind to a different Arc's control block.
    Weak& operator=(const rusty::Arc<T>& arc) {
        reset();
        ptr = arc.ptr;
        // @unsafe { raw ControlBlock* deref + std::atomic::fetch_add }
        {
            if (ptr) {
                ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
            }
        }
        return *this;
    }

    // @safe - Release the held weak reference (drops weak_count by one).
    void reset() {
        // @unsafe { Arc<T>::release_weak does atomic dec + maybe-delete }
        {
            if (ptr) {
                rusty::Arc<T>::release_weak(ptr);
                ptr = nullptr;
            }
        }
    }

    // @safe - Atomic Weak→Arc transition; atomic ops + raw-ptr deref
    // encapsulated in the inner @unsafe block.
    Option<rusty::Arc<T>> upgrade() const {
        if (!ptr) {
            return ::rusty::None;
        }
        // @unsafe { std::atomic load + CAS, raw ControlBlock* deref }
        {
            size_t count = ptr->strong_count.load(std::memory_order_acquire);
            while (count != 0) {
                if (ptr->strong_count.compare_exchange_weak(
                        count,
                        count + 1,
                        std::memory_order_acquire,
                        std::memory_order_relaxed)) {
                    return ::rusty::Some(rusty::Arc<T>(ptr, false));
                }
            }
            return ::rusty::None;
        }
    }

    // @safe - One atomic load + null check; load wrapped in @unsafe block.
    bool expired() const {
        // @unsafe { raw ControlBlock* deref + std::atomic::load }
        {
            return !ptr ||
                   ptr->strong_count.load(std::memory_order_acquire) == 0;
        }
    }

    // @safe - One atomic load.
    size_t strong_count() const {
        // @unsafe { raw ControlBlock* deref + std::atomic::load }
        {
            return ptr ? ptr->strong_count.load(std::memory_order_acquire) : 0;
        }
    }

    // @safe - One atomic load + count correction (self doesn't count).
    size_t weak_count() const {
        if (!ptr) {
            return 0;
        }
        // @unsafe { raw ControlBlock* deref + std::atomic::load }
        {
            size_t count = ptr->weak_count.load(std::memory_order_acquire);
            return count > 0 ? count - 1 : 0;
        }
    }

    // @safe - Delegates to the copy ctor.
    Weak clone() const {
        return Weak(*this);
    }
};

// Downgrade function - create weak from strong
// @safe
template<typename T>
Weak<T> downgrade(const rusty::Arc<T>& arc) {
    return Weak<T>(arc);
}

} // namespace sync
} // namespace rusty

#endif // RUSTY_SYNC_WEAK_HPP
