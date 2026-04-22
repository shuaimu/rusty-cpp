#ifndef RUSTY_TRY_HPP
#define RUSTY_TRY_HPP

#include <type_traits>

// Rust-like ? operator for C++ using GCC/Clang statement expressions.
//
// RUSTY_TRY(expr):
//   Evaluates `expr` which must return a Result<T, E> or Option<T>.
//   If the result is Err/None, early-returns from the current function.
//   Otherwise, evaluates to the unwrapped T value.
//
// RUSTY_CO_TRY(expr):
//   Same as RUSTY_TRY but uses co_return for async functions (coroutines).
//
// Usage:
//   rusty::Result<int, std::string> parse(std::string_view s);
//
//   rusty::Result<int, std::string> process(std::string_view input) {
//       auto value = RUSTY_TRY(parse(input));  // early-returns Err if parse fails
//       return rusty::Result<int, std::string>::ok(value * 2);
//   }
//
// Note: Requires GCC or Clang (uses statement expressions extension).
// MSVC does not support statement expressions.

// ? on Result<T, E> — unwrap Ok(T) or return Err(E)
#define RUSTY_TRY(expr) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            return std::move(_rusty_try_result); \
        } \
        _rusty_try_result.unwrap(); \
    })

// ? on Result<T, E> in a function returning a different Result<U, E>.
// `__VA_ARGS__` carries the full return Result type (may include commas).
#define RUSTY_TRY_INTO(expr, ...) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            return __VA_ARGS__::Err(_rusty_try_result.unwrap_err()); \
        } \
        _rusty_try_result.unwrap(); \
    })

// ? on Result<T, E> in async context — uses co_return
#define RUSTY_CO_TRY(expr) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            co_return std::move(_rusty_try_result); \
        } \
        _rusty_try_result.unwrap(); \
    })

// ? on Result<T, E> in async context with explicit Result<U, E> return type.
#define RUSTY_CO_TRY_INTO(expr, ...) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            co_return __VA_ARGS__::Err(_rusty_try_result.unwrap_err()); \
        } \
        _rusty_try_result.unwrap(); \
    })

// ? on Option<T> — unwrap Some(T) or return None
#define RUSTY_TRY_OPT(expr) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_none()) { \
            return rusty::None; \
        } \
        _rusty_try_result.unwrap(); \
    })

// ? on Option<T> in async context
#define RUSTY_CO_TRY_OPT(expr) \
    ({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_none()) { \
            co_return rusty::None; \
        } \
        _rusty_try_result.unwrap(); \
    })

#endif // RUSTY_TRY_HPP
