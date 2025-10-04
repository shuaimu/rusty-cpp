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
    Weak() : ptr(nullptr) {}

    explicit Weak(const rusty::Arc<T>& arc)
        : ptr(arc.ptr) {
        if (ptr) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
        }
    }

    Weak(const Weak& other)
        : ptr(other.ptr) {
        if (ptr) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
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
                ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
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

    Weak& operator=(const rusty::Arc<T>& arc) {
        reset();
        ptr = arc.ptr;
        if (ptr) {
            ptr->weak_count.fetch_add(1, std::memory_order_relaxed);
        }
        return *this;
    }

    void reset() {
        if (ptr) {
            rusty::Arc<T>::release_weak(ptr);
            ptr = nullptr;
        }
    }

    Option<rusty::Arc<T>> upgrade() const {
        if (!ptr) {
            return ::rusty::None;
        }

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

    bool expired() const {
        return !ptr || ptr->strong_count.load(std::memory_order_acquire) == 0;
    }

    size_t strong_count() const {
        return ptr ? ptr->strong_count.load(std::memory_order_acquire) : 0;
    }

    size_t weak_count() const {
        if (!ptr) {
            return 0;
        }
        size_t count = ptr->weak_count.load(std::memory_order_acquire);
        return count > 0 ? count - 1 : 0;
    }

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
