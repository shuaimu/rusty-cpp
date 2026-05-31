#ifndef RUSTY_MARKER_HPP
#define RUSTY_MARKER_HPP

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

#endif // RUSTY_MARKER_HPP
