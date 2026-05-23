#ifndef RUSTY_SYS_ENV_HPP
#define RUSTY_SYS_ENV_HPP

// rusty::sys::env — Rust-like environment / host helpers for C++
//
// A minimal subset of Rust's std::env aimed at making @safe callers
// possible without leaking raw `char buf[N]` / `gethostname(2)` into
// safe code.
//
// All entry points are @safe; their bodies wrap the libc syscall in a
// single @unsafe block. Return types are owned values (std::string) —
// no raw `char*` escapes.

#include <climits>
#include <cstddef>
#include <cstring>
#include <string>
#include <unistd.h>

#ifndef RUSTY_SYS_ENV_HOSTNAME_BUF
#define RUSTY_SYS_ENV_HOSTNAME_BUF 256
#endif

namespace rusty {
namespace sys {
namespace env {

// @safe - gethostname(2): returns the host's nodename as an owned
// std::string. Falls back to an empty string on syscall failure
// (parity with the pre-wrapper rrr behavior).
inline std::string hostname() noexcept {
    // @unsafe { gethostname is a libc syscall + raw `char buf[]` on
    //           the stack. The function returns an owned std::string;
    //           no raw `char*` escapes. }
    {
        char buf[RUSTY_SYS_ENV_HOSTNAME_BUF];
        std::memset(buf, 0, sizeof(buf));
        const int rc = ::gethostname(buf, sizeof(buf) - 1);
        if (rc != 0) {
            return std::string{};
        }
        // Defensive: ensure NUL-termination even if the kernel returned
        // a truncated nodename without one.
        buf[sizeof(buf) - 1] = '\0';
        return std::string{buf};
    }
}

}  // namespace env
}  // namespace sys
}  // namespace rusty

#endif  // RUSTY_SYS_ENV_HPP
