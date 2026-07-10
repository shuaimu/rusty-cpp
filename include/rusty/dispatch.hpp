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

#include <cstdint>
#include <memory>
#include <type_traits>
#include <utility>

namespace rusty {

namespace detail {

/// @brief Check whether `f` is invocable somewhere along `r`'s deref chain.
///
/// Recursive `consteval` check used to make `deref_call` itself
/// SFINAE-friendly: when neither `f(r)` nor any step of `*r` is
/// invocable, `deref_call`'s constraint fails — letting callers wrap
/// the call in `requires { rusty::deref_call(r, f); }` to probe
/// reachability without triggering a hard static_assert in the body.
template<typename R, typename F>
consteval bool lambda_reachable_via_deref() {
    if constexpr (requires(R&& r, F&& f) { static_cast<F&&>(f)(r); }) {
        return true;
    } else if constexpr (requires(R&& r) { *r; }) {
        return lambda_reachable_via_deref<
            decltype(*std::declval<R&&>()), F>();
    } else {
        return false;
    }
}

/// @brief A pointer from a Rust-reference-typed binding, whatever its C++
/// carrier shape: identity (decayed to prvalue) when the binding already
/// lowered to a raw pointer, address-of when it lowered to an lvalue.
template<typename T>
constexpr decltype(auto) ptr_or_addr(T&& v) {
    if constexpr (std::is_pointer_v<std::remove_cvref_t<T>>) {
        return +v;
    } else {
        return std::addressof(v);
    }
}

/// @brief Move-capture surrogate for a reference-typed binding in a `move`
/// closure: copyable payloads capture by value (what `std::move` of a const
/// ref did anyway), move-only payloads carry the referent's ADDRESS — the
/// deref-dispatch at use sites tolerates either carrier.
template<typename T>
constexpr decltype(auto) ref_capture(const T& v) {
    if constexpr (std::is_copy_constructible_v<T>) {
        return T(v);
    } else {
        return std::addressof(v);
    }
}

/// @brief `expr as *T` for a source whose C++ carrier shape the transpiler
/// could not resolve: pointers reinterpret directly, integers round-trip
/// through uintptr_t (usize-as-pointer), lvalues decay to their address.
template<typename Target, typename V>
Target ptr_cast(V&& v) {
    using Src = std::remove_cvref_t<V>;
    if constexpr (std::is_pointer_v<Src>) {
        return reinterpret_cast<Target>(+v);
    } else if constexpr (std::is_integral_v<Src> || std::is_enum_v<Src>) {
        return reinterpret_cast<Target>(static_cast<std::uintptr_t>(v));
    } else {
        return reinterpret_cast<Target>(std::addressof(v));
    }
}

} // namespace detail

/// @brief Universal deref-walking method dispatcher.
///
/// Tries to invoke `f(r)`. If ill-formed, recursively retries on `*r`,
/// until either the invocation succeeds or `r` is no longer dereferenceable.
///
/// Constrained on `detail::lambda_reachable_via_deref<R, F>()` so the
/// call is SFINAE'd out — rather than hard-erroring — when no step of
/// the chain accepts the lambda. This lets the outer `requires {
/// rusty::deref_call(r, f); }` cleanly evaluate to false for receivers
/// that have no matching method anywhere in their deref chain.
///
/// @tparam R Receiver type (forwarding reference).
/// @tparam F Callable type — typically a generic lambda
///           `[&](auto&& r) -> decltype(r.METHOD(ARGS)) { return r.METHOD(ARGS); }`.
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
    requires (detail::lambda_reachable_via_deref<R, F>())
constexpr decltype(auto) deref_call(R&& r, F&& f) {
    if constexpr (requires { f(r); }) {
        return f(std::forward<R>(r));
    } else {
        // Constraint guarantees one of the two branches applies; if
        // `f(r)` is not callable, `*r` must be well-formed and the
        // recursion's reachability holds (consteval check threaded
        // through `lambda_reachable_via_deref`).
        return deref_call(*std::forward<R>(r), std::forward<F>(f));
    }
}

/// Rust `V::default()` for a TYPE-PARAM owner V. Struct emissions carry a
/// `default_()` static member; a C-like enum-class cannot hold members, so
/// its Default impl emits an ADL marker next to the enum
/// (`E __rusty_default(std::type_identity<E>)`, carrying the impl's
/// variant). Tiers: member → ADL marker → value-init.
template<typename V>
constexpr V default_like() {
    if constexpr (requires { V::default_(); }) {
        return V::default_();
    } else if constexpr (requires { __rusty_default(std::type_identity<V>{}); }) {
        return __rusty_default(std::type_identity<V>{});
    } else {
        return V{};
    }
}

} // namespace rusty
