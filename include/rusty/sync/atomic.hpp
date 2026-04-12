#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <type_traits>

#include "../result.hpp"

namespace rusty::sync::atomic {

enum class Ordering {
    Relaxed,
    Release,
    Acquire,
    AcqRel,
    SeqCst,
};

inline constexpr std::memory_order to_std_memory_order(Ordering order) noexcept {
    switch (order) {
        case Ordering::Relaxed:
            return std::memory_order_relaxed;
        case Ordering::Release:
            return std::memory_order_release;
        case Ordering::Acquire:
            return std::memory_order_acquire;
        case Ordering::AcqRel:
            return std::memory_order_acq_rel;
        case Ordering::SeqCst:
        default:
            return std::memory_order_seq_cst;
    }
}

template<typename T>
class Atomic {
public:
    using value_type = T;

    Atomic() noexcept : inner_(T{}) {}
    explicit Atomic(T value) noexcept : inner_(value) {}

    static Atomic new_(T value) noexcept {
        return Atomic(value);
    }

    Atomic(const Atomic& other) noexcept
        : inner_(other.inner_.load(std::memory_order_relaxed)) {}

    Atomic& operator=(const Atomic& other) noexcept {
        store(other.load(Ordering::Relaxed), Ordering::Relaxed);
        return *this;
    }

    Atomic(Atomic&& other) noexcept : Atomic(other.load(Ordering::Relaxed)) {}

    Atomic& operator=(Atomic&& other) noexcept {
        store(other.load(Ordering::Relaxed), Ordering::Relaxed);
        return *this;
    }

    T load(Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.load(to_std_memory_order(order));
    }

    void store(T value, Ordering order = Ordering::SeqCst) const noexcept {
        inner_.store(value, to_std_memory_order(order));
    }

    T swap(T value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.exchange(value, to_std_memory_order(order));
    }

    rusty::Result<T, T> compare_exchange(
        T current,
        T new_value,
        Ordering success,
        Ordering failure) const noexcept {
        T expected = current;
        if (inner_.compare_exchange_strong(
                expected,
                new_value,
                to_std_memory_order(success),
                to_std_memory_order(failure))) {
            return rusty::Result<T, T>::Ok(current);
        }
        return rusty::Result<T, T>::Err(expected);
    }

    rusty::Result<T, T> compare_exchange_weak(
        T current,
        T new_value,
        Ordering success,
        Ordering failure) const noexcept {
        T expected = current;
        if (inner_.compare_exchange_weak(
                expected,
                new_value,
                to_std_memory_order(success),
                to_std_memory_order(failure))) {
            return rusty::Result<T, T>::Ok(current);
        }
        return rusty::Result<T, T>::Err(expected);
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_add(U value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_add(value, to_std_memory_order(order));
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_sub(U value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_sub(value, to_std_memory_order(order));
    }

    template<typename U = T>
    requires std::is_pointer_v<U>
    U fetch_add(std::ptrdiff_t value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_add(value, to_std_memory_order(order));
    }

    template<typename U = T>
    requires std::is_pointer_v<U>
    U fetch_sub(std::ptrdiff_t value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_sub(value, to_std_memory_order(order));
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_and(U value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_and(value, to_std_memory_order(order));
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_or(U value, Ordering order = Ordering::SeqCst) const noexcept {
        return inner_.fetch_or(value, to_std_memory_order(order));
    }

    operator const Atomic*() const noexcept {
        return this;
    }

    operator Atomic*() noexcept {
        return this;
    }

private:
    mutable std::atomic<T> inner_;
};

using AtomicBool = Atomic<bool>;
using AtomicUsize = Atomic<std::size_t>;

template<typename T>
using AtomicPtr = Atomic<T*>;

inline void fence(Ordering order = Ordering::SeqCst) noexcept {
    std::atomic_thread_fence(to_std_memory_order(order));
}

} // namespace rusty::sync::atomic
