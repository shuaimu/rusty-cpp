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
    Weak() : ptr(nullptr) {}

    explicit Weak(const rusty::Rc<T>& rc)
        : ptr(rc.control_) {
        if (ptr) {
            ++ptr->weak_count;
        }
    }

    Weak(const Weak& other)
        : ptr(other.ptr) {
        if (ptr) {
            ++ptr->weak_count;
        }
    }

    Weak(Weak&& other) noexcept
        : ptr(other.ptr) {
        other.ptr = nullptr;
    }

    ~Weak() {
        reset();
    }

    Weak& operator=(const Weak& other) {
        if (this != &other) {
            reset();
            ptr = other.ptr;
            if (ptr) {
                ++ptr->weak_count;
            }
        }
        return *this;
    }

    Weak& operator=(Weak&& other) noexcept {
        if (this != &other) {
            reset();
            ptr = other.ptr;
            other.ptr = nullptr;
        }
        return *this;
    }

    Weak& operator=(const rusty::Rc<T>& rc) {
        reset();
        ptr = rc.control_;
        if (ptr) {
            ++ptr->weak_count;
        }
        return *this;
    }

    void reset() {
        if (ptr) {
            rusty::Rc<T>::release_weak(ptr);
            ptr = nullptr;
        }
    }

    Option<rusty::Rc<T>> upgrade() const {
        if (!ptr || ptr->strong_count == 0) {
            return ::rusty::None;
        }
        T* value_ptr = static_cast<T*>(ptr->get_value_ptr());
        return ::rusty::Some(rusty::Rc<T>(value_ptr, ptr, true));
    }

    bool expired() const {
        return !ptr || ptr->strong_count == 0;
    }

    size_t strong_count() const {
        return ptr ? ptr->strong_count : 0;
    }

    size_t weak_count() const {
        if (!ptr) {
            return 0;
        }
        return ptr->weak_count > 0 ? ptr->weak_count - 1 : 0;
    }

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
