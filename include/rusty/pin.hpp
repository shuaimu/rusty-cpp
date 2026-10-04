#ifndef RUSTY_PIN_HPP
#define RUSTY_PIN_HPP

// Stub declarations for Rust's `core::pin` traits.
//
// These names appear in transpiled rustc source as
// `using rusty::pin::PinCoerceUnsized;` etc. — they're decorative (variance
// markers / SFINAE pivots) in the rust code and never invoked at runtime.
// C++ has no analogue, so each is an empty struct just so the `using`
// declarations resolve. If a real `Pin<T>` implementation is ever needed,
// promote the relevant struct out of this stub section.

#include <type_traits>
#include <utility>

namespace rusty {
namespace pin {

// NOTE: deliberately do NOT define `Pin<T>` here — transpiled rustc
// code emits its own `template<typename T> using Pin = T;` alias
// inside its `pin` namespace (which auto-namespace mode lands in
// `rusty::pin` when the surrounding file already opened
// `namespace rusty`). Defining a struct template here would clash
// with the transpiled alias. If a port needs `Pin<T>` outside of
// the transpiled code, it can declare its own local alias.

template<typename T = void> struct PinCoerceUnsized {};
template<typename T = void, typename U = void> struct PinDerefMut {};

} // namespace pin

// `Pin<&mut T>` / `Pin<&T>` in transpiled code is the pinned place itself (a
// `T&`, or the `T*` a `self` receiver lowers to), so Pin's projections are
// identities over that place: a pointer argument is dereferenced and the
// result is an lvalue reference to the pinned value. (A separate namespace:
// transpiled ports declare their own `rusty::pin` members.)
namespace pin_place {
namespace detail {
template<typename P>
decltype(auto) place(P&& p) {
    if constexpr (std::is_pointer_v<std::remove_cvref_t<P>>) {
        return *p;
    } else {
        return static_cast<std::remove_reference_t<P>&>(p);
    }
}
} // namespace detail

// `pin.get_unchecked_mut()` / `get_mut()` / `get_ref()` / `into_ref()`.
template<typename P>
decltype(auto) get_unchecked_mut(P&& p) { return detail::place(std::forward<P>(p)); }
template<typename P>
decltype(auto) get_mut(P&& p) { return detail::place(std::forward<P>(p)); }
template<typename P>
decltype(auto) get_ref(P&& p) { return detail::place(std::forward<P>(p)); }
template<typename P>
decltype(auto) into_ref(P&& p) { return detail::place(std::forward<P>(p)); }
// `pin.as_mut()` / `as_ref()` reborrow the pin, and `pin!(e)` in expression
// position is `e`'s place: all the same place.
template<typename P>
decltype(auto) as_mut(P&& p) { return detail::place(std::forward<P>(p)); }
template<typename P>
decltype(auto) as_ref(P&& p) { return detail::place(std::forward<P>(p)); }

// `pin.map_unchecked_mut(|s| &mut s.field)`: the projected field, pinned.
template<typename P, typename F>
decltype(auto) map_unchecked_mut(P&& p, F&& f) {
    return detail::place(std::forward<F>(f)(detail::place(std::forward<P>(p))));
}
template<typename P, typename F>
decltype(auto) map_unchecked(P&& p, F&& f) {
    return detail::place(std::forward<F>(f)(detail::place(std::forward<P>(p))));
}
} // namespace pin_place
} // namespace rusty

#endif // RUSTY_PIN_HPP
