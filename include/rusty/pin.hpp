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
} // namespace rusty

#endif // RUSTY_PIN_HPP
