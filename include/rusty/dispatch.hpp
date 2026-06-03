#pragma once

/// @file dispatch.hpp
/// @brief Generic auto-deref dispatcher (`rusty::deref_call`)
///
/// `rusty::deref_call(receiver, lambda)` walks the deref chain of `receiver`
/// until the lambda's body is invocable on the current step. This simulates
/// Rust's auto-deref method resolution at C++ compile time.
///
/// Typical usage by the transpiler — for a Rust call site `receiver.method(args)`
/// where the transpiler can't statically prove the method lives directly on
/// the receiver's type (e.g., generic templates, cross-port deref chains):
///
///     // Rust: vec.iter()
///     auto it = rusty::deref_call(vec, [&](auto&& r) -> decltype(auto) {
///         return r.iter();
///     });
///
/// The dispatcher tries `lambda(receiver)`; if ill-formed, it dereferences
/// (`*receiver`) and recurses. The first matching arm wins, mirroring Rust's
/// "stop at the first type in the deref chain that has the method" rule.
///
/// The transpiler should only emit this form when **uncertain** whether the
/// method lives directly on the receiver. When the receiver type is known to
/// have the method directly, emit `receiver.method(args)`. When the receiver
/// is known to be a wrapper that requires one deref step, emit
/// `(*receiver).method(args)`. The dispatcher is the fallback for the
/// "don't know" case.

#include <type_traits>
#include <utility>

namespace rusty {

/// @brief Universal deref-walking method dispatcher.
///
/// Tries to invoke `f(r)`. If ill-formed, recursively retries on `*r`,
/// until either the invocation succeeds or `r` is no longer dereferenceable.
///
/// @tparam R Receiver type (forwarding reference).
/// @tparam F Callable type — typically a generic lambda
///           `[&](auto&& r) -> decltype(auto) { return r.METHOD(ARGS); }`.
/// @return Whatever `f(r_after_derefs)` returns, with value category preserved.
///
/// @note When the receiver type is concrete and the method is statically
///       resolvable, prefer a direct call (`receiver.method(args)`) over
///       this dispatcher to save template-instantiation cost. Use
///       `deref_call` only for the "don't know if a deref is needed" case.
// @bridge - This is a safety-propagating bridge function. Its body is
// template plumbing that only exists to invoke the caller-supplied
// lambda. The actual semantic content (and any safety burden) lives in
// the lambda the caller passes, which is analyzed in the caller's own
// @safe context. Marking it @bridge lets @safe callers invoke it
// without an @unsafe block.
template<typename R, typename F>
constexpr decltype(auto) deref_call(R&& r, F&& f) {
    if constexpr (requires { f(r); }) {
        return f(std::forward<R>(r));
    } else if constexpr (requires { *r; }) {
        return deref_call(*std::forward<R>(r), std::forward<F>(f));
    } else {
        static_assert(
            sizeof(R) == 0,
            "rusty::deref_call: method not found on receiver type or its "
            "deref chain. The lambda's body cannot be invoked on the "
            "receiver, and *receiver is not well-formed."
        );
    }
}

} // namespace rusty
