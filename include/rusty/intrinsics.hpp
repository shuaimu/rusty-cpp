#ifndef RUSTY_INTRINSICS_HPP
#define RUSTY_INTRINSICS_HPP

// Stub for Rust's `core::intrinsics::*`. Most are no-ops or trivial
// wrappers over C++ equivalents. Currently only the surface used by
// transpiled core_slice_port lives here; expand as other ports surface
// new intrinsic refs.

#include <utility>

namespace rusty {
namespace intrinsics {

// Note: `unreachable()` is emitted inline by the transpiler in each
// cppm — don't redeclare it here (would conflict on noexcept). Only
// the surface that the transpiler does NOT emit lives here.
[[noreturn]] inline void unreachable_unchecked() { __builtin_unreachable(); }

// `assume(cond)` — hint to the optimiser. C++23 has `[[assume]]`; we
// fall back to `__builtin_assume` (clang) or guarded `__builtin_unreachable`.
inline void assume(bool cond) noexcept {
#if defined(__clang__)
    __builtin_assume(cond);
#else
    if (!cond) __builtin_unreachable();
#endif
}

// `likely(b)` / `unlikely(b)` — branch hints. Pure pass-through.
inline constexpr bool likely(bool b) noexcept { return b; }
inline constexpr bool unlikely(bool b) noexcept { return b; }

} // namespace intrinsics
} // namespace rusty

#endif // RUSTY_INTRINSICS_HPP
