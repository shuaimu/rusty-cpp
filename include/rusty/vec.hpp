#ifndef RUSTY_VEC_HPP
#define RUSTY_VEC_HPP

// VecLegacy retired. `rusty::Vec<T, A>` is now `::Vec<T, A>` — the
// transpiled rustc `Vec` exported from the C++20 module `vec_port.vec`.
//
// The alias lives in the `rusty` umbrella module (rusty.cppm), not in this
// header — C++20 headers cannot `import` modules, so consumers that need
// `rusty::Vec` must take it from the module:
//
//     import rusty;             // pulls in rusty::Vec via the umbrella
//   or
//     import vec_port.vec;      // pulls in ::Vec directly
//
// Header-only consumers that previously included `<rusty/vec.hpp>` no
// longer get rusty::Vec from this header. They will see "no template named
// Vec in namespace rusty" at point-of-use and must convert to module
// consumption. Most code already #includes <rusty/rusty.hpp> + uses
// `rusty::Vec` — switch those callers to `import rusty;`.
//
// This header is retained on the include path so callers that #include it
// continue to compile (header itself is empty); see rusty.cppm for the
// alias definition.

#include <rusty/alloc.hpp>  // rusty::alloc::Global (for the default A param)

#endif // RUSTY_VEC_HPP
