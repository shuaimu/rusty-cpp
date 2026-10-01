#pragma once

#include <functional>
#include <memory>
#include <type_traits>
#include <utility>

namespace rusty::detail {

// A copyable type-erased callable whose object representation may be moved
// BITWISE. Rust moves every value by memcpy, and so do the transpiled
// containers: hashbrown's resize copies a slot's bytes into the new table,
// btree shifts slots with memcpy. libc++'s std::function keeps a small
// callable in an inline buffer that it points to from inside itself, so a
// byte-copied one still points into the old slot, and destroying it frees a
// pointer into that slot ("free(): invalid pointer"). Here the std::function
// lives on the heap and the object is that one pointer: nothing in it
// refers to its own address. Copies are deep, as std::function's are.
template<typename Sig>
class RelocatableFunction;

template<typename R, typename... Args>
class RelocatableFunction<R(Args...)> {
    using Inner = std::function<R(Args...)>;
    std::unique_ptr<Inner> fn_;

public:
    RelocatableFunction() noexcept = default;
    RelocatableFunction(std::nullptr_t) noexcept {}

    template<typename F>
        requires (!std::is_same_v<std::remove_cvref_t<F>, RelocatableFunction>
                  && std::is_constructible_v<Inner, F &&>)
    RelocatableFunction(F&& f) {
        Inner fn(std::forward<F>(f));
        if (fn) {
            fn_ = std::make_unique<Inner>(std::move(fn));
        }
    }

    RelocatableFunction(const RelocatableFunction& other)
        : fn_(other.fn_ ? std::make_unique<Inner>(*other.fn_) : nullptr) {}
    RelocatableFunction(RelocatableFunction&&) noexcept = default;

    RelocatableFunction& operator=(const RelocatableFunction& other) {
        if (this != &other) {
            fn_ = other.fn_ ? std::make_unique<Inner>(*other.fn_) : nullptr;
        }
        return *this;
    }
    RelocatableFunction& operator=(RelocatableFunction&&) noexcept = default;
    RelocatableFunction& operator=(std::nullptr_t) noexcept {
        fn_.reset();
        return *this;
    }

    explicit operator bool() const noexcept { return fn_ != nullptr && static_cast<bool>(*fn_); }

    R operator()(Args... args) const {
        if (!fn_) {
            throw std::bad_function_call();
        }
        return (*fn_)(std::forward<Args>(args)...);
    }

    template<typename T>
    T* target() noexcept {
        return fn_ ? fn_->template target<T>() : nullptr;
    }
    template<typename T>
    const T* target() const noexcept {
        return fn_ ? std::as_const(*fn_).template target<T>() : nullptr;
    }
};

}  // namespace rusty::detail
