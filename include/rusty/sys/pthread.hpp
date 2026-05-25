#ifndef RUSTY_SYS_PTHREAD_HPP
#define RUSTY_SYS_PTHREAD_HPP

// rusty::sys::pthread — Rust-like thread-identity helpers for C++
//
// A minimal subset focused on making @safe callers possible without
// leaking raw `pthread_t` values into safe code. The wrapper returns a
// stable hash (uint64_t) of the current thread's pthread_t — the
// concrete bit-pattern of pthread_t is unspecified by POSIX, so a
// hash is the only portable identity value.
//
// rusty::sync::mutex / rusty::sync::condvar wrap pthread mutexes /
// condvars in their own @safe APIs; this header focuses on the
// thread-identity surface only.

#include <cstdint>
#include <functional>
#include <pthread.h>

namespace rusty {
namespace sys {
namespace pthread {

// @safe - pthread_self(3) + std::hash<pthread_t>: returns a stable
// hash of the calling thread's pthread_t. The concrete pthread_t
// value never escapes the function body.
inline std::uint64_t current_id_hash() noexcept {
    // @unsafe { pthread_self libc call + std::hash<pthread_t>; both
    //           return by value, no raw handle escape. }
    {
        return static_cast<std::uint64_t>(
            std::hash<pthread_t>{}(::pthread_self()));
    }
}

}  // namespace pthread
}  // namespace sys
}  // namespace rusty

#endif  // RUSTY_SYS_PTHREAD_HPP
