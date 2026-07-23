#ifndef RUSTY_PANIC_HANDLER_HPP
#define RUSTY_PANIC_HANDLER_HPP

// The single point every rusty panic routes through, so panic behaviour is
// defined in ONE place. Rust panics are typeless (an opaque payload); this
// mirrors that -- a unified panic rather than a scattered set of typed C++
// throws (out_of_range / length_error / runtime_error / ...).
//
// This header is intentionally DEPENDENCY-FREE (no rusty/* includes) so the
// lowest-level headers (result.hpp, option.hpp) can route through it without
// an include cycle -- panic.hpp includes result.hpp, so the primitive cannot
// live there. panic.hpp builds catch_unwind / begin_panic on top of this.
//
// Stage 2 adds a RUSTY_PANIC_ABORT compile-time switch here (Rust's
// `panic = "abort"`): when defined, do_panic() will abort() instead of
// throwing. Keeping every panic funnelled through this one function is what
// makes that a single-line switch.

#include <cstdlib>
#include <stdexcept>
#include <string>
#include <string_view>

namespace rusty {
namespace panic {

// Rust's `panic = "unwind"` (the default): unwind the stack via a throw so
// destructors run and catch_unwind can intercept.
[[noreturn]] inline void do_panic(std::string_view message) {
    throw std::runtime_error(std::string(message));
}

[[noreturn]] inline void do_panic() {
    do_panic("panic");
}

} // namespace panic
} // namespace rusty

#endif // RUSTY_PANIC_HANDLER_HPP
