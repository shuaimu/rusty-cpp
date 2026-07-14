#ifndef RUSTY_TRY_HPP
#define RUSTY_TRY_HPP

#include <type_traits>
#include <utility>

namespace rusty {
namespace detail {

template<typename TargetErr, typename SrcErr>
decltype(auto) convert_try_into_error(SrcErr&& err) {
    if constexpr (std::is_constructible_v<TargetErr, SrcErr&&>) {
        return TargetErr(std::forward<SrcErr>(err));
    } else if constexpr (requires(SrcErr&& candidate) {
        TargetErr::from(std::forward<SrcErr>(candidate));
    }) {
        return TargetErr::from(std::forward<SrcErr>(err));
    } else {
        return std::forward<SrcErr>(err);
    }
}

} // namespace detail
} // namespace rusty

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

#include <memory>
#include <utility>

// Statement expressions yield their final expression AS AN RVALUE — a
// reference-typed unwrap (Option<&T> / Result<&T, E> through `?`) DECAYS to a
// value copy at the macro boundary, and any reference formed from the binding
// afterwards dangles once the copy dies (btree Difference::next returned
// Option<const T&> into a dead stack slot — ASan stack-use-after-return).
// Carry the unwrapped value through the boundary in a trivially-movable
// wrapper instead: lvalues travel as a pointer and are re-derefed outside the
// statement expression; prvalues travel by move.
namespace rusty::detail {
template <typename T> struct try_ref_carrier { T* p; };
template <typename T> struct try_val_carrier { T v; };

template <typename T>
inline auto make_try_carrier(T&& v) {
    if constexpr (std::is_lvalue_reference_v<T&&>) {
        return try_ref_carrier<std::remove_reference_t<T>>{std::addressof(v)};
    } else {
        return try_val_carrier<std::remove_reference_t<T>>{std::forward<T>(v)};
    }
}
template <typename T> inline T& unwrap_try_carrier(try_ref_carrier<T> c) { return *c.p; }
template <typename T> inline T unwrap_try_carrier(try_val_carrier<T>&& c) { return std::move(c.v); }
} // namespace rusty::detail

// ? on Result<T, E> — unwrap Ok(T) or return Err(E)
#define RUSTY_TRY(expr) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            return std::move(_rusty_try_result); \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

// ? on Result<T, E> in a function returning a different Result<U, E>.
// `__VA_ARGS__` carries the full return Result type (may include commas).
#define RUSTY_TRY_INTO(expr, ...) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            using _rusty_target_result_t = __VA_ARGS__; \
            using _rusty_target_err_t = typename _rusty_target_result_t::err_type; \
            auto _rusty_try_err = _rusty_try_result.unwrap_err(); \
            return _rusty_target_result_t::Err( \
                ::rusty::detail::convert_try_into_error<_rusty_target_err_t>( \
                    std::forward<decltype(_rusty_try_err)>(_rusty_try_err))); \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

// ? on Result<T, E> in async context — uses co_return
#define RUSTY_CO_TRY(expr) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            co_return std::move(_rusty_try_result); \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

// ? on Result<T, E> in async context with explicit Result<U, E> return type.
#define RUSTY_CO_TRY_INTO(expr, ...) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_err()) { \
            using _rusty_target_result_t = __VA_ARGS__; \
            using _rusty_target_err_t = typename _rusty_target_result_t::err_type; \
            auto _rusty_try_err = _rusty_try_result.unwrap_err(); \
            co_return _rusty_target_result_t::Err( \
                ::rusty::detail::convert_try_into_error<_rusty_target_err_t>( \
                    std::forward<decltype(_rusty_try_err)>(_rusty_try_err))); \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

// ? on Option<T> — unwrap Some(T) or return None
//
// Returns `rusty::None` (a None_t sentinel) rather than
// `decltype(_rusty_try_result)(rusty::None)` so that the function's
// return-type Option<U> drives the conversion. This mirrors Rust's
// `?` semantics where `Option<X>::None` can short-circuit a function
// returning `Option<Y>` because both share the unit None variant.
#define RUSTY_TRY_OPT(expr) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_none()) { \
            return ::rusty::None; \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

// ? on Option<T> in async context
#define RUSTY_CO_TRY_OPT(expr) \
    (::rusty::detail::unwrap_try_carrier(({ \
        auto _rusty_try_result = (expr); \
        if (_rusty_try_result.is_none()) { \
            co_return rusty::None; \
        } \
        ::rusty::detail::make_try_carrier(_rusty_try_result.unwrap()); \
    })))

#endif // RUSTY_TRY_HPP
