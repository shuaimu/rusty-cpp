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

// `likely(b)` / `unlikely(b)` — branch hints.
//
// These names are commonly function-like macros (`__builtin_expect(!!(x), ...)`)
// in lower-level headers that may be active in the same TU (eRPC's common.h,
// Masstree's compiler.hh, Linux-kernel-style code). Such a macro textually
// rewrites the definitions below into `__builtin_expect(...)` and breaks the
// build ("expected ')'"). Fence the definitions: save and #undef any such
// macro, define, then restore it so downstream macro users are unaffected.
// push_macro/pop_macro are no-ops when no such macro is present. (Same guard
// libc++ uses for std::min/std::max via _LIBCPP_PUSH_MACROS.)
//
// NOTE: since the macro is restored afterward, do not add a *qualified*
// `rusty::intrinsics::likely(x)` call in a TU where such a macro is live.
#pragma push_macro("likely")
#pragma push_macro("unlikely")
#undef likely
#undef unlikely

inline constexpr bool likely(bool b)   noexcept { return __builtin_expect(b, 1); }
inline constexpr bool unlikely(bool b) noexcept { return __builtin_expect(b, 0); }

#pragma pop_macro("unlikely")
#pragma pop_macro("likely")

} // namespace intrinsics
} // namespace rusty

#endif // RUSTY_INTRINSICS_HPP
