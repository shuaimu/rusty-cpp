#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <type_traits>

#include "../result.hpp"

// @safe - thread-safe primitives that wrap std::atomic<T>. Each
// method's body funnels into a `// @unsafe { ... }` block around the
// underlying `std::atomic<T>::*` call (STL, not borrow-checked).
//
// Rust std exposes only the concrete typed aliases (`AtomicBool`,
// `AtomicI32`, `AtomicU64`, …) — there is no generic `std::atomic<T>`
// equivalent. To keep the rusty surface parallel, the underlying
// generic template lives in `detail::` and is **not** part of the
// public API. New user code MUST use one of the concrete aliases
// below; the templated form is only an implementation detail kept to
// avoid duplicating the same method bodies thirteen times.
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

namespace detail {

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
        // @unsafe { std::atomic<T>::load }
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
        // @unsafe { std::atomic<T>::load is STL, not borrow-checked }
        { return inner_.load(to_std_memory_order(order)); }
    }

    void store(T value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::store is STL, not borrow-checked }
        { inner_.store(value, to_std_memory_order(order)); }
    }

    T swap(T value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::exchange is STL, not borrow-checked }
        { return inner_.exchange(value, to_std_memory_order(order)); }
    }

    T* get_mut() noexcept {
        return reinterpret_cast<T*>(&inner_);
    }

    const T* get_mut() const noexcept {
        return reinterpret_cast<const T*>(&inner_);
    }

    rusty::Result<T, T> compare_exchange(
        T current,
        T new_value,
        Ordering success,
        Ordering failure) const noexcept {
        T expected = current;
        // @unsafe { std::atomic<T>::compare_exchange_strong is STL, not borrow-checked }
        bool ok = false;
        {
            ok = inner_.compare_exchange_strong(
                expected,
                new_value,
                to_std_memory_order(success),
                to_std_memory_order(failure));
        }
        if (ok) {
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
        // @unsafe { std::atomic<T>::compare_exchange_weak is STL, not borrow-checked }
        bool ok = false;
        {
            ok = inner_.compare_exchange_weak(
                expected,
                new_value,
                to_std_memory_order(success),
                to_std_memory_order(failure));
        }
        if (ok) {
            return rusty::Result<T, T>::Ok(current);
        }
        return rusty::Result<T, T>::Err(expected);
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_add(U value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::fetch_add is STL, not borrow-checked }
        { return inner_.fetch_add(value, to_std_memory_order(order)); }
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_sub(U value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::fetch_sub is STL, not borrow-checked }
        { return inner_.fetch_sub(value, to_std_memory_order(order)); }
    }

    template<typename U = T>
    requires std::is_pointer_v<U>
    U fetch_add(std::ptrdiff_t value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T*>::fetch_add is STL, not borrow-checked }
        { return inner_.fetch_add(value, to_std_memory_order(order)); }
    }

    template<typename U = T>
    requires std::is_pointer_v<U>
    U fetch_sub(std::ptrdiff_t value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T*>::fetch_sub is STL, not borrow-checked }
        { return inner_.fetch_sub(value, to_std_memory_order(order)); }
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_and(U value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::fetch_and is STL, not borrow-checked }
        { return inner_.fetch_and(value, to_std_memory_order(order)); }
    }

    template<typename U = T>
    requires (std::is_integral_v<U> && !std::is_same_v<U, bool>)
    U fetch_or(U value, Ordering order = Ordering::SeqCst) const noexcept {
        // @unsafe { std::atomic<T>::fetch_or is STL, not borrow-checked }
        { return inner_.fetch_or(value, to_std_memory_order(order)); }
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

} // namespace detail

using AtomicBool = detail::Atomic<bool>;
using AtomicI8 = detail::Atomic<std::int8_t>;
using AtomicI16 = detail::Atomic<std::int16_t>;
using AtomicI32 = detail::Atomic<std::int32_t>;
using AtomicI64 = detail::Atomic<std::int64_t>;
using AtomicIsize = detail::Atomic<std::ptrdiff_t>;
using AtomicU8 = detail::Atomic<std::uint8_t>;
using AtomicU16 = detail::Atomic<std::uint16_t>;
using AtomicU32 = detail::Atomic<std::uint32_t>;
using AtomicU64 = detail::Atomic<std::uint64_t>;
using AtomicUsize = detail::Atomic<std::size_t>;

template<typename T>
using AtomicPtr = detail::Atomic<T*>;

inline void fence(Ordering order = Ordering::SeqCst) noexcept {
    // @unsafe { std::atomic_thread_fence is STL, not borrow-checked }
    { std::atomic_thread_fence(to_std_memory_order(order)); }
}

} // namespace rusty::sync::atomic
