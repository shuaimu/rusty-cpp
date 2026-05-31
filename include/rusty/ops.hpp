#ifndef RUSTY_OPS_HPP
#define RUSTY_OPS_HPP

// Stub declarations for Rust's `core::ops` traits.
//
// These trait names appear in transpiled rustc source as `using ops::Deref;`
// etc. — they're decorative (variance markers / SFINAE pivots) in the rust
// code and never invoked at runtime. C++ has no analogue, so we represent
// each as an empty struct just so the `using` declarations resolve.
//
// If a real implementation is ever needed for a specific trait (e.g.
// SFINAE on `requires Deref<T>`), promote the relevant struct from this
// stub file into a real header.

namespace rusty {
namespace ops {

template<typename T = void> struct Deref {};
template<typename T = void> struct DerefMut {};
template<typename T = void> struct DerefPure {};
template<typename T = void, typename U = void> struct CoerceUnsized {};
template<typename T = void, typename U = void> struct DispatchFromDyn {};
template<typename T = void> struct Drop {};
template<typename T = void, typename Args = void> struct Fn {};
template<typename T = void, typename Args = void> struct FnMut {};
template<typename T = void, typename Args = void> struct FnOnce {};
template<typename T = void, typename Output = void> struct Add {};
template<typename T = void, typename Output = void> struct Sub {};
template<typename T = void, typename Output = void> struct Mul {};
template<typename T = void, typename Output = void> struct Div {};
template<typename T = void, typename Output = void> struct Neg {};
template<typename T = void, typename Output = void> struct Not {};
template<typename T = void, typename Output = void> struct BitAnd {};
template<typename T = void, typename Output = void> struct BitOr {};
template<typename T = void, typename Output = void> struct BitXor {};
template<typename T = void, typename Output = void> struct Shl {};
template<typename T = void, typename Output = void> struct Shr {};
template<typename T = void, typename Output = void> struct Rem {};
template<typename T = void> struct AddAssign {};
template<typename T = void> struct SubAssign {};
template<typename T = void> struct MulAssign {};
template<typename T = void> struct DivAssign {};
template<typename T = void> struct RemAssign {};
template<typename T = void> struct BitAndAssign {};
template<typename T = void> struct BitOrAssign {};
template<typename T = void> struct BitXorAssign {};
template<typename T = void> struct ShlAssign {};
template<typename T = void> struct ShrAssign {};
template<typename T = void, typename Output = void> struct Index {};
template<typename T = void> struct IndexMut {};
template<typename Idx = void> struct Range {};
template<typename Idx = void> struct RangeFrom {};
template<typename Idx = void> struct RangeTo {};
template<typename Idx = void> struct RangeInclusive {};
template<typename Idx = void> struct RangeToInclusive {};
struct RangeFull {};
template<typename T = void> struct Try {};
template<typename T = void> struct FromResidual {};

} // namespace ops
} // namespace rusty

#endif // RUSTY_OPS_HPP
