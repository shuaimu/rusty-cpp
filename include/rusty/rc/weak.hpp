#ifndef RUSTY_RC_WEAK_HPP
#define RUSTY_RC_WEAK_HPP

#include <cstddef>

#include "../option.hpp"
#include "../rc.hpp"

namespace rusty {
namespace rc {

template<typename T>
class Weak {
private:
    friend class rusty::Rc<T>;

    typename rusty::Rc<T>::ControlBlock* ptr;

    Weak(typename rusty::Rc<T>::ControlBlock* p, bool add_ref)
        : ptr(p) {
        if (ptr && add_ref) {
            ++ptr->weak_count;
        }
    }

public:
    // @safe - Default ctor; sets the raw control-block pointer to null.
    Weak() : ptr(nullptr) {}

    // @safe - Borrow control block from a strong Rc and bump weak_count.
    explicit Weak(const rusty::Rc<T>& rc)
        : ptr(rc.control_) {
        // @unsafe { raw ControlBlock* deref }
        {
            if (ptr) {
                ++ptr->weak_count;
            }
        }
    }

    // @safe - Copy ctor: share control block + bump weak_count.
    Weak(const Weak& other)
        : ptr(other.ptr) {
        // @unsafe { raw ControlBlock* deref }
        {
            if (ptr) {
                ++ptr->weak_count;
            }
        }
    }

    // @safe - Move ctor: transfer the raw pointer.
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
            // @unsafe { raw ControlBlock* deref }
            {
                if (ptr) {
                    ++ptr->weak_count;
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

    // @safe - Re-bind to a different Rc's control block.
    Weak& operator=(const rusty::Rc<T>& rc) {
        reset();
        ptr = rc.control_;
        // @unsafe { raw ControlBlock* deref }
        {
            if (ptr) {
                ++ptr->weak_count;
            }
        }
        return *this;
    }

    // @safe - Release the held weak reference.
    void reset() {
        // @unsafe { Rc<T>::release_weak does raw ControlBlock* deref + maybe-delete }
        {
            if (ptr) {
                rusty::Rc<T>::release_weak(ptr);
                ptr = nullptr;
            }
        }
    }

    // @safe - Single-threaded Weak→Rc transition; raw-ptr deref + cast
    // encapsulated in the inner @unsafe block.
    Option<rusty::Rc<T>> upgrade() const {
        // @unsafe { raw ControlBlock* deref, static_cast on raw ptr }
        {
            if (!ptr || ptr->strong_count == 0) {
                return ::rusty::None;
            }
            T* value_ptr = static_cast<T*>(ptr->get_value_ptr());
            return ::rusty::Some(rusty::Rc<T>(value_ptr, ptr, true));
        }
    }

    // @safe - One raw-ptr deref of the control block, no atomics (single-threaded).
    bool expired() const {
        // @unsafe { raw ControlBlock* deref }
        { return !ptr || ptr->strong_count == 0; }
    }

    // @safe
    size_t strong_count() const {
        // @unsafe { raw ControlBlock* deref }
        { return ptr ? ptr->strong_count : 0; }
    }

    // @safe
    size_t weak_count() const {
        if (!ptr) {
            return 0;
        }
        // @unsafe { raw ControlBlock* deref }
        { return ptr->weak_count > 0 ? ptr->weak_count - 1 : 0; }
    }

    // @safe - Delegates to the copy ctor.
    Weak clone() const {
        return Weak(*this);
    }
};

// Downgrade function - create weak from strong
// @safe
template<typename T>
Weak<T> downgrade(const rusty::Rc<T>& rc) {
    return Weak<T>(rc);
}

} // namespace rc
} // namespace rusty

#endif // RUSTY_RC_WEAK_HPP
