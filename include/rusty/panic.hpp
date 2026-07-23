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
#include "rusty/panic_handler.hpp"  // rusty::panic::do_panic — the unified panic primitive

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
    do_panic();
}

template<typename Message, typename... Args>
[[noreturn]] inline void begin_panic(Message&& message, Args&&...) {
    if constexpr (std::is_convertible_v<Message, std::string_view>) {
        do_panic(std::string_view(std::forward<Message>(message)));
    } else {
        do_panic();
    }
}

// Stand-in for Rust's `core::panic::Location` (the `&'static panic::Location`
// pointer carried in BorrowError / BorrowMutError for diagnostics). The
// transpiled cell port references `rusty::panic::Location` by reference and
// reads only `.caller()` for formatting; we surface that as a no-op that
// returns the same Location, which is enough to satisfy method-lookup at
// instantiation time.
struct Location {
    constexpr const char* file() const noexcept { return ""; }
    constexpr unsigned line() const noexcept { return 0; }
    constexpr unsigned column() const noexcept { return 0; }
    // `core::panic::Location::caller()` is a static intrinsic in Rust
    // (it returns the &'static Location pointing at the caller). We
    // return a reference to a global zero-Location — accurate enough
    // for an empty stub, since cell_port only stores+forwards it.
    static const Location& caller() noexcept {
        static const Location _;
        return _;
    }
};

// `const_panic` is the Rust-side const-eval-friendly panic shim. The
// transpiled code uses it through `using panic::const_panic;`, but the
// expanded calls land in unreachable! / panic! branches. Map it onto
// `begin_panic` so any residual call site still aborts.
template<typename... Args>
[[noreturn]] inline void const_panic(Args&&... args) {
    begin_panic(std::forward<Args>(args)...);
}

} // namespace panic
} // namespace rusty

#endif // RUSTY_PANIC_HPP
