#ifndef RUSTY_PANIC_HPP
#define RUSTY_PANIC_HPP

#include <cstdlib>
#include <exception>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>

#include "rusty/result.hpp"

namespace rusty {
namespace panic {

template<typename F>
struct AssertUnwindSafe {
    F callable;

    explicit AssertUnwindSafe(F f) : callable(std::move(f)) {}
};

template<typename F>
AssertUnwindSafe(F) -> AssertUnwindSafe<F>;

template<typename T>
struct is_assert_unwind_safe : std::false_type {};

template<typename F>
struct is_assert_unwind_safe<AssertUnwindSafe<F>> : std::true_type {};

template<typename F>
auto catch_unwind(AssertUnwindSafe<F> wrapped) {
    using Return = decltype(wrapped.callable());
    try {
        if constexpr (std::is_void_v<Return>) {
            wrapped.callable();
            return rusty::Result<void, std::exception_ptr>::Ok();
        } else {
            return rusty::Result<Return, std::exception_ptr>::Ok(wrapped.callable());
        }
    } catch (...) {
        if constexpr (std::is_void_v<Return>) {
            return rusty::Result<void, std::exception_ptr>::Err(std::current_exception());
        } else {
            return rusty::Result<Return, std::exception_ptr>::Err(std::current_exception());
        }
    }
}

template<typename F>
    requires(!is_assert_unwind_safe<std::remove_cvref_t<F>>::value)
auto catch_unwind(F&& callable) {
    return catch_unwind(AssertUnwindSafe<std::remove_cvref_t<F>>(
        std::forward<F>(callable)));
}

[[noreturn]] inline void resume_unwind(std::exception_ptr payload) {
    if (payload) {
        std::rethrow_exception(payload);
    }
    std::abort();
}

template<typename... Args>
[[noreturn]] inline void begin_panic(Args&&...) {
    throw std::runtime_error("panic");
}

template<typename Message, typename... Args>
[[noreturn]] inline void begin_panic(Message&& message, Args&&...) {
    if constexpr (std::is_convertible_v<Message, std::string_view>) {
        throw std::runtime_error(
            std::string(std::string_view(std::forward<Message>(message))));
    } else {
        throw std::runtime_error("panic");
    }
}

} // namespace panic
} // namespace rusty

#endif // RUSTY_PANIC_HPP
