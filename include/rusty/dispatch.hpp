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

#include <array>
#include <cstddef>
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
template<typename R, typename F, typename... A>
consteval bool lambda_reachable_via_deref() {
    if constexpr (requires(R&& r, F&& f, A&&... a) {
                      static_cast<F&&>(f)(r, static_cast<A&&>(a)...);
                  }) {
        return true;
    } else if constexpr (requires(R&& r) { *r; }) {
        return lambda_reachable_via_deref<
            decltype(*std::declval<R&&>()), F, A...>();
    } else {
        return false;
    }
}

/// @brief Rust `[T; 0]`: size 0, alignment of T. `std::array<T, 0>` has
/// sizeof 1 (libstdc++ keeps a dummy member, so it is NOT an empty class
/// and [[no_unique_address]] cannot collapse it) — as the leading member
/// of Rust's `_align: [Group; 0]` alignment idiom it pushed the successor
/// field to offset 1, permanently misaligning hashbrown's static-empty
/// ctrl group. Truly empty + alignas restores Rust repr(C) layout.
template<typename T>
struct alignas(T) zero_length_array {
    constexpr zero_length_array() noexcept = default;
    // Emitted initializers still spell `std::array<T, 0>{}`.
    constexpr zero_length_array(std::array<T, 0>) noexcept {}
    constexpr T* data() noexcept { return nullptr; }
    constexpr const T* data() const noexcept { return nullptr; }
    constexpr std::size_t size() const noexcept { return 0; }
    constexpr bool empty() const noexcept { return true; }
    constexpr T* begin() noexcept { return nullptr; }
    constexpr T* end() noexcept { return nullptr; }
    constexpr const T* begin() const noexcept { return nullptr; }
    constexpr const T* end() const noexcept { return nullptr; }
};

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

/// @brief Rust unary `!`: logical for bool, BITWISE for integers. Used when
/// the transpiler cannot type the operand (C++ `!` on an integer would
/// collapse it to 0/1 — hashbrown's `BitMask(!mask.0)` became an empty mask).
template<typename T>
constexpr auto rust_not(T v) {
    if constexpr (std::is_same_v<std::remove_cvref_t<T>, bool>) {
        return !v;
    } else if constexpr (std::is_integral_v<std::remove_cvref_t<T>>) {
        return ~v;
    } else {
        // User `impl Not` types emit their own operator! — dispatch to it
        // instead of applying `~` (which such class types don't define).
        return !v;
    }
}

/// @brief A mutable span from either span flavor: slice-tail view wrappers
/// hold `std::span<T>`, but the SHARED-view constructor receives
/// `std::span<const T>` (Rust's `&Slice` guarantees no mutation through it).
template<typename T>
constexpr auto despan_const(std::span<T> s) noexcept {
    if constexpr (std::is_const_v<T>) {
        using U = std::remove_const_t<T>;
        return std::span<U>(const_cast<U*>(s.data()), s.size());
    } else {
        return s;
    }
}

/// @brief Declared return type for a DIVERGING closure (`|_| unreachable!()`,
/// Rust type `!`): converts to anything so the callable satisfies slots with
/// concrete return expectations (a hasher returning uint64_t). The body's
/// [[noreturn]] panic fires first; the conversion itself is unreachable.
struct diverging_value {
    template<typename T>
    [[noreturn]] operator T() const {
        std::abort();
    }
};

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

/// @brief The `&mut` flavor: writes through the capture must reach the
/// referent, so a mutable reference binding always carries the address.
template<typename T>
constexpr T* ref_capture(T& v) {
    return std::addressof(v);
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

/// @brief Rust `*expr as *T` where `expr`'s type did not resolve at
/// transpile time. Receives the UN-derefed carrier: a blind C++-side deref
/// of a pointer carrier is ambiguous — `int*` may carry `&i32` (peel) or
/// BE the payload `&mut i32` of a `&K` accessor (identity; peeling read
/// the pointee `1` and ptr_cast's integral arm minted address 0x1 —
/// indexmap's occupied_entry_key). Rust only permits `usize` → pointer
/// casts, so a pointee that is neither a pointer-sized unsigned integer
/// nor itself a pointer CANNOT be the peel case.
template<typename Target, typename V>
Target deref_ptr_cast(V&& v) {
    using Src = std::remove_cvref_t<V>;
    if constexpr (std::is_pointer_v<Src>) {
        using Pointee = std::remove_cv_t<std::remove_pointer_t<Src>>;
        if constexpr ((std::is_integral_v<Pointee> && std::is_unsigned_v<Pointee>
                       && sizeof(Pointee) == sizeof(std::uintptr_t))
                      || std::is_pointer_v<Pointee>) {
            // `&usize`/`&*mut T` behind a pointer carrier: the Rust deref
            // yields the pointee.
            return ptr_cast<Target>(*v);
        } else {
            // The carrier itself is the reference/pointer payload.
            return reinterpret_cast<Target>(+v);
        }
    } else if constexpr (requires { *v; }) {
        // Smart-wrapper carrier (Box<*mut T>): the deref is real.
        return ptr_cast<Target>(*v);
    } else {
        return ptr_cast<Target>(static_cast<V&&>(v));
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
// Trailing args `a...` are forwarded to `f(r, a...)` at each deref step. This
// lets the transpiler pass a NAMESPACE-SCOPE method-dispatch functor (see
// RUSTY_METHOD_DISPATCH) plus the call's arguments, instead of a generic lambda
// LOCAL to the enclosing function template that captures those arguments — the
// local-lambda-across-a-module-boundary shape crashes clang's mangler (issue
// #31, "site 2"). Callers that pass a nullary functor use the empty pack.
template<typename R, typename F, typename... A>
    requires (detail::lambda_reachable_via_deref<R, F, A...>())
constexpr decltype(auto) deref_call(R&& r, F&& f, A&&... a) {
    if constexpr (requires { f(r, a...); }) {
        return f(std::forward<R>(r), std::forward<A>(a)...);
    } else {
        // Constraint guarantees one of the two branches applies; if
        // `f(r, a...)` is not callable, `*r` must be well-formed and the
        // recursion's reachability holds (consteval check threaded
        // through `lambda_reachable_via_deref`).
        return deref_call(*std::forward<R>(r), std::forward<F>(f), std::forward<A>(a)...);
    }
}

/// Generate a namespace-scope method-dispatch functor `__mdisp_<name>` whose
/// `operator()(recv, args...)` forwards to `recv.<name>(args...)` with a
/// decltype-SFINAE return (so `deref_call` can probe reachability and recurse
/// through `*recv`). The transpiler emits one of these per method name it
/// dispatches via `deref_call`, at namespace scope, so the closure is NOT local
/// to a function template — see issue #31.
#define RUSTY_METHOD_DISPATCH(name)                                            \
    struct __mdisp_##name {                                                    \
        template<typename R, typename... A>                                    \
        constexpr auto operator()(R&& r, A&&... a) const                       \
            -> decltype(static_cast<R&&>(r).name(static_cast<A&&>(a)...)) {     \
            return static_cast<R&&>(r).name(static_cast<A&&>(a)...);            \
        }                                                                      \
    };

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
