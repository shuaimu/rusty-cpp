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
#include "rusty/any.hpp"
#include "rusty/string.hpp"
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

// A standard Rust panic keeps its original Any payload. Deriving from
// runtime_error preserves ordinary C++ exception diagnostics at mixed callers.
class AnyException : public std::runtime_error {
    std::shared_ptr<rusty::any_types::BoxAny> payload_;
public:
    AnyException(rusty::any_types::BoxAny payload, std::string message)
        : std::runtime_error(std::move(message)),
          payload_(std::make_shared<rusty::any_types::BoxAny>(std::move(payload))) {}
    rusty::any_types::BoxAny take_payload() {
        if (payload_->has_value()) return std::move(*payload_);
        // C++ can copy an exception_ptr and rethrow it after Rust consumed
        // its move-only payload. Preserve its diagnostic and exception
        // identity on later catches; the original value cannot be cloned.
        auto diagnostic = rusty::any_types::BoxAny::new_(rusty::String::from(what()));
        diagnostic.retain_foreign_exception(std::current_exception());
        return diagnostic;
    }
};

inline std::string any_message(const rusty::any_types::BoxAny& payload) {
    if (auto text = payload.downcast_ref<rusty::String>(); text.is_some()) {
        const auto& value = text.unwrap();
        return std::string(value.as_str());
    }
    if (auto text = payload.downcast_ref<std::string_view>(); text.is_some()) {
        return std::string(text.unwrap());
    }
    return "Box<dyn Any>";
}

template<typename T>
[[noreturn]] void panic_any(T payload) {
    auto erased = rusty::any_types::BoxAny::new_(std::move(payload));
    auto message = any_message(erased);
#ifdef RUSTY_PANIC_ABORT
    do_panic(message);
#else
    throw AnyException(std::move(erased), std::move(message));
#endif
}

// std::panic uses a boxed Any error, while the historical C++ entry above
// continues returning exception_ptr for existing C++ consumers.
template<typename F>
auto catch_unwind_std(AssertUnwindSafe<F> wrapped) {
    using Return = decltype(wrapped.callable());
    using Result = rusty::Result<Return, rusty::any_types::BoxAny>;
    try {
        if constexpr (std::is_void_v<Return>) {
            wrapped.callable();
            return Result::Ok();
        } else {
            return Result::Ok(wrapped.callable());
        }
    } catch (AnyException& exception) {
        return Result::Err(exception.take_payload());
    } catch (const std::exception& exception) {
        auto payload = rusty::any_types::BoxAny::new_(rusty::String::from(exception.what()));
        payload.retain_foreign_exception(std::current_exception());
        return Result::Err(std::move(payload));
    } catch (...) {
        auto original = std::current_exception();
        auto payload = rusty::any_types::BoxAny::new_(original);
        payload.retain_foreign_exception(std::move(original));
        return Result::Err(std::move(payload));
    }
}

template<typename F>
    requires(!is_assert_unwind_safe<std::remove_cvref_t<F>>::value)
auto catch_unwind_std(F&& callable) {
    return catch_unwind_std(AssertUnwindSafe<std::remove_cvref_t<F>>(
        std::forward<F>(callable)));
}

[[noreturn]] inline void resume_unwind_std(rusty::any_types::BoxAny payload) {
#ifdef RUSTY_PANIC_ABORT
    std::abort();
#else
    if (auto exception = payload.foreign_exception()) std::rethrow_exception(exception);
    auto message = any_message(payload);
    throw AnyException(std::move(payload), std::move(message));
#endif
}

// Recover the diagnostic carried by a caught C++ exception. Rust panic
// payloads are opaque, so callers must handle the None case just as they
// would for a non-string Rust panic payload.
inline rusty::Option<std::string> payload_message(std::exception_ptr payload) {
    if (!payload) {
        return rusty::None;
    }
    try {
        std::rethrow_exception(payload);
    } catch (const std::exception& e) {
        return rusty::Some(std::string(e.what()));
    } catch (...) {
        return rusty::None;
    }
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
