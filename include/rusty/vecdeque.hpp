#ifndef RUSTY_VECDEQUE_HPP
#define RUSTY_VECDEQUE_HPP

// Hand-written `rusty::VecDeque<T>` retired. `rusty::VecDeque<T, A>` is
// now the transpiled rustc `VecDeque` exported from the C++20 module
// `vec_deque_port`. Mirrors the Vec / BTreeMap retirement pattern.
//
// The alias lives in the `rusty` umbrella module (rusty.cppm), not in
// this header — C++20 headers cannot `import` modules, so consumers
// that need `rusty::VecDeque` must take it from the module:
//
//     import rusty;             // pulls in rusty::VecDeque via the umbrella
//   or
//     import vec_deque_port;    // pulls in vec_deque_port::VecDeque directly
//
// Header-only consumers that previously `#include <rusty/vecdeque.hpp>`
// no longer get the type from this header. They will see "no template
// named VecDeque in namespace rusty" at point-of-use and must convert
// to module consumption.
//
// This header is retained on the include path so callers that #include
// it continue to compile (header itself is empty); see rusty.cppm for
// the alias definition.

#include <rusty/alloc.hpp>  // rusty::alloc::Global (for the default A param)

#endif // RUSTY_VECDEQUE_HPP
