#ifndef RUSTY_MARKER_HPP
#define RUSTY_MARKER_HPP

#include <compare>
#include <cstddef>
#include <functional>
#include <string>

namespace rusty {

// Zero-sized marker used to carry type/lifetime information in transpiled code.
template<typename T>
struct PhantomData {
    using Value = T;
    using value_type = T;

    constexpr PhantomData() noexcept = default;

    template<typename U>
    constexpr PhantomData(const PhantomData<U>&) noexcept {}
};

// Book §3.2.2 rule 7 — trailing keys where a C++ parameter type cannot carry a
// Rust distinction. `tag<A…>`: a generic trait's arguments (`impl Tr<i32> for
// T` beside `impl Tr<u8> for T` differ only in them); `self_tag<Self>`: the
// impl's self type INCLUDING reference-ness (`impl Tr for T` beside `impl Tr
// for &T` both take `const T&`). Both are defaulted on the common case, so a
// plain call stays `Tr_::m(x)`; the emitter passes them where Rust's
// resolution is lexically determined (a bound, a path, a `let` annotation, a
// receiver of reference depth ≥ 2) and the dyn forwarders pass them in every
// slot.
template<typename... A>
struct tag {};
template<typename S>
struct self_tag {};

// Erased Verus ghost state. `rusty-cpp-transpiler --verus-exec` lowers
// vstd's `Ghost<T>` and `Tracked<T>` (plain-rustc `PhantomData` wrappers whose
// `T` is a spec type such as `int` or a ghost log) and their executable
// constructors (`Ghost::assume_new()`, `Ghost::assume_new_fallback(..)`, the
// `Tracked` twins) to this one empty tag; `T` is never emitted. It keeps the
// slot, so tuple arity and patterns are as written, and fields of this type
// are emitted `[[no_unique_address]]`, so the slot takes no storage.
//
// The tag is trivially copyable and supports `==`, `<=>`, hashing and debug
// printing, so structs that hold ghost fields still derive
// Clone/Copy/PartialEq/PartialOrd/Hash/Debug through it. All tags compare
// equal (a ghost value has no executable content), hash to 0, and print as
// nothing, which is what vstd's `Tracked<T>` `Debug` impl writes.
struct Ghost {
    constexpr Ghost clone() const noexcept { return {}; }
    friend constexpr bool operator==(Ghost, Ghost) noexcept { return true; }
    friend constexpr std::strong_ordering operator<=>(Ghost, Ghost) noexcept {
        return std::strong_ordering::equal;
    }
    std::string rusty_debug_string() const { return std::string(); }
};

namespace convert {

// Stand-in for Rust's `core::convert::Infallible`.
struct Infallible {};

} // namespace convert

namespace marker {

// Stub declarations for Rust's `core::marker` traits / markers.
//
// These names appear in transpiled rustc source as `using marker::Foo;` —
// they're decorative (variance markers / SFINAE pivots) in the rust code
// and never invoked at runtime. C++ has no analogue, so each is an empty
// struct just so the `using` declarations resolve. If a real implementation
// is ever needed, promote the relevant struct out of this stub section.

template<typename T = void> struct Copy {};
template<typename T = void> struct Sized {};
template<typename T = void> struct Send {};
template<typename T = void> struct Sync {};
template<typename T = void> struct Unpin {};
template<typename T = void> struct Destruct {};
template<typename T = void, typename U = void> struct Unsize {};
struct PhantomPinned {};

// Convenience re-export so `marker::PhantomData<T>` resolves to the top-level
// PhantomData defined above.
template<typename T>
using PhantomData = ::rusty::PhantomData<T>;

} // namespace marker

} // namespace rusty

template<>
struct std::hash<rusty::Ghost> {
    std::size_t operator()(rusty::Ghost) const noexcept { return 0; }
};

#endif // RUSTY_MARKER_HPP
