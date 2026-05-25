#ifndef RUSTY_SYS_TIME_HPP
#define RUSTY_SYS_TIME_HPP

// rusty::sys::time — Rust-like time helpers for C++
//
// A minimal subset of Rust's std::time aimed at making @safe callers
// possible without leaking raw `timespec` / `timeval` / clock_gettime
// / nanosleep into safe code.
//
// All entry points are @safe; their bodies wrap the libc syscall in
// a single @unsafe block so the function returns a plain integral
// type (microseconds since epoch / boot) with no raw handle escape.

#include <cstdint>
#include <ctime>
#include <sys/time.h>
#include <time.h>

namespace rusty {
namespace sys {
namespace time {

constexpr std::uint64_t kUsecPerSec = 1000000;
constexpr std::uint64_t kNsecPerUsec = 1000;
constexpr std::uint64_t kNsecPerSec  = kUsecPerSec * kNsecPerUsec;

// @safe - microseconds since the Unix epoch (CLOCK_REALTIME).
inline std::uint64_t clock_realtime_us() noexcept {
    // @unsafe { clock_gettime is a libc syscall; the timespec is a stack
    //           local. The function returns a plain uint64_t. }
    {
        struct timespec ts;
        ::clock_gettime(CLOCK_REALTIME, &ts);
        return static_cast<std::uint64_t>(ts.tv_sec) * kUsecPerSec +
               static_cast<std::uint64_t>(ts.tv_nsec) / kNsecPerUsec;
    }
}

// @safe - microseconds since the Unix epoch, coarse precision
// (CLOCK_REALTIME_COARSE on Linux, falls back to CLOCK_REALTIME on
// platforms that lack the coarse variant).
inline std::uint64_t clock_realtime_coarse_us() noexcept {
    // @unsafe { clock_gettime is a libc syscall; the timespec is a stack
    //           local. The function returns a plain uint64_t. }
    {
        struct timespec ts;
#if defined(CLOCK_REALTIME_COARSE)
        ::clock_gettime(CLOCK_REALTIME_COARSE, &ts);
#else
        ::clock_gettime(CLOCK_REALTIME, &ts);
#endif
        return static_cast<std::uint64_t>(ts.tv_sec) * kUsecPerSec +
               static_cast<std::uint64_t>(ts.tv_nsec) / kNsecPerUsec;
    }
}

// @safe - microseconds since some unspecified monotonic origin
// (CLOCK_MONOTONIC). Suitable for measuring durations; not for wall-
// clock display.
inline std::uint64_t clock_monotonic_us() noexcept {
    // @unsafe { clock_gettime is a libc syscall; the timespec is a stack
    //           local. The function returns a plain uint64_t. }
    {
        struct timespec ts;
        ::clock_gettime(CLOCK_MONOTONIC, &ts);
        return static_cast<std::uint64_t>(ts.tv_sec) * kUsecPerSec +
               static_cast<std::uint64_t>(ts.tv_nsec) / kNsecPerUsec;
    }
}

// @safe - microseconds since the Unix epoch via gettimeofday(2).
// Provided for callers that need the historical `struct timeval`
// resolution without manually invoking the syscall.
inline std::uint64_t gettimeofday_us() noexcept {
    // @unsafe { gettimeofday is a libc syscall; the timeval is a stack
    //           local. The function returns a plain uint64_t. }
    {
        struct timeval tv;
        ::gettimeofday(&tv, nullptr);
        return static_cast<std::uint64_t>(tv.tv_sec) * kUsecPerSec +
               static_cast<std::uint64_t>(tv.tv_usec);
    }
}

// @safe - sleep the current thread for `microseconds` (nanosleep-
// based). Spurious wake-ups are not retried; callers that need
// strict duration semantics should re-check elapsed time.
inline void sleep_us(std::uint64_t microseconds) noexcept {
    // @unsafe { nanosleep is a libc syscall; the timespec is a stack
    //           local. The function returns void. }
    {
        struct timespec ts;
        ts.tv_sec  = static_cast<time_t>(microseconds / kUsecPerSec);
        ts.tv_nsec = static_cast<long>((microseconds % kUsecPerSec) * kNsecPerUsec);
        ::nanosleep(&ts, nullptr);
    }
}

}  // namespace time
}  // namespace sys
}  // namespace rusty

#endif  // RUSTY_SYS_TIME_HPP
