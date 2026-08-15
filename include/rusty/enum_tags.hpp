#ifndef RUSTY_ENUM_TAGS_HPP
#define RUSTY_ENUM_TAGS_HPP

// Enum-lowering channel.
//
// A Rust enum reaches C++ in one of two shapes:
//   * unit-only enums  -> a C++ `enum class E { A, B };`
//   * data enums       -> `using E = std::variant<E_A, E_B>;` plus tag structs
//
// A transpiled `match` arm naming a variant of an enum the crate does NOT
// declare cannot tell which shape it got: there is no registry entry for a
// foreign enum, and that missing knowledge is exactly what makes the arm
// ambiguous. The two shapes need different refutations —
//   enum class:  `m == E::V`
//   data enum:   `rusty::detail::variant_holds<E_V>(m)`
// — so an emitter that picks one form breaks the other.
//
// The emitted arm therefore asks the TYPE. For the enum-class shape nothing is
// needed: `E::V` names the enumerator and `requires { E::V; }` detects it. The
// variant shape has no such name, so every data enum publishes one
// specialisation mapping its variant NAMES to their tag TYPES:
//
//     template<> struct enum_variant_tags<Bound<T>> {
//         using Unbounded = Bound_Unbounded<T>;
//         ...
//     };
//
// Declaring it beside the enum keeps the two from drifting apart, and it
// travels with the header or module, so a consumer needs no side table and no
// cross-crate manifest entry.
//
// A type with no specialisation is not an error: the arm falls back to the
// conservative always-match behaviour it had before this channel existed.
//
// SCOPE TODAY: the specialisations below cover the data enums the RUNTIME
// declares. The transpiler does not yet emit one for a data enum it lowers
// itself, so a match arm naming a variant of a DEPENDENCY crate's data enum
// still takes the always-match fallback. Emitting them from the data-enum
// lowering (both the `using E = std::variant<...>` and the
// `struct E : std::variant<...>` wrapper shapes, threading template
// parameters) is the remaining half of this channel.

namespace rusty {
namespace detail {

/// Primary: no variant tags known for `E`. Data enums specialise this.
template<typename E>
struct enum_variant_tags {};

} // namespace detail
} // namespace rusty

#endif // RUSTY_ENUM_TAGS_HPP
