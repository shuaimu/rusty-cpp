#ifndef RUSTY_PANIC_HPP
#define RUSTY_PANIC_HPP

#include <cstdlib>
#include <exception>
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

[[noreturn]] inline void resume_unwind(std::exception_ptr payload) {
    if (payload) {
        std::rethrow_exception(payload);
    }
    std::abort();
}

} // namespace panic
} // namespace rusty

#endif // RUSTY_PANIC_HPP
