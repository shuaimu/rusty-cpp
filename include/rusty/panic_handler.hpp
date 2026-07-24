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
// The panic strategy is chosen at compile time via RUSTY_PANIC_ABORT, exactly
// like Rust's `panic = "unwind"` (default) vs `panic = "abort"`. Because every
// panic funnels through do_panic(), this is a single-point switch:
//
//   default            -> throw (unwind the stack, run destructors, catchable
//                          via rusty::panic::catch_unwind)
//   -DRUSTY_PANIC_ABORT -> print to stderr + std::abort() (no unwinding, no
//                          cleanup, not catchable) — smaller binaries, mirrors
//                          Rust's `-C panic=abort`.

#include <cstdlib>
#include <string_view>
#ifdef RUSTY_PANIC_ABORT
#include <cstdio>
#else
#include <stdexcept>
#include <string>
#endif

namespace rusty {
namespace panic {

[[noreturn]] inline void do_panic(std::string_view message) {
#ifdef RUSTY_PANIC_ABORT
    // Rust's `panic = "abort"`: no unwinding, no cleanup — report and die.
    std::fputs("thread panicked: ", stderr);
    std::fwrite(message.data(), 1, message.size(), stderr);
    std::fputc('\n', stderr);
    std::abort();
#else
    // Rust's `panic = "unwind"` (default): unwind the stack via a throw so
    // destructors run and catch_unwind can intercept.
    throw std::runtime_error(std::string(message));
#endif
}

[[noreturn]] inline void do_panic() {
    do_panic("panic");
}

} // namespace panic
} // namespace rusty

#endif // RUSTY_PANIC_HANDLER_HPP
