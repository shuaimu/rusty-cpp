#ifndef RUSTY_SYS_PROCESS_HPP
#define RUSTY_SYS_PROCESS_HPP

// rusty::sys::process — Rust-like process / system-info helpers for C++
//
// A minimal subset of Rust's std::process / std::env aimed at making
// @safe callers possible without leaking raw `struct sysinfo` / `struct
// tms` / `pid_t` into safe code.
//
// All entry points are @safe; their bodies wrap the libc syscall in a
// single @unsafe block so the function returns a plain integral type
// (or a small POD result struct) with no raw handle escape.

#include <cstdint>
#include <sys/times.h>
#include <unistd.h>

#if defined(__linux__)
#include <sys/sysinfo.h>
#endif

namespace rusty {
namespace sys {
namespace process {

// @safe - getpid(2): returns the calling process's PID.
inline int getpid() noexcept {
    // @unsafe { libc getpid syscall — pure, no escape. }
    {
        return static_cast<int>(::getpid());
    }
}

// @safe - sysconf(3): query a runtime system limit / parameter by
// canonical name (e.g. _SC_NPROCESSORS_ONLN, _SC_PAGE_SIZE).
inline long sysconf(int name) noexcept {
    // @unsafe { libc sysconf — pure, no escape. }
    {
        return ::sysconf(name);
    }
}

// Aggregate of process CPU-time counters as returned by times(2).
// All four fields are clock-tick counters (clock_t) cast to int64_t so
// callers can compute deltas without worrying about platform width.
struct ProcessTimes {
    std::int64_t wall_ticks;     // return value of times()
    std::int64_t user_ticks;     // tms_utime
    std::int64_t system_ticks;   // tms_stime
    std::int64_t cuser_ticks;    // tms_cutime  (children, terminated)
    std::int64_t csystem_ticks;  // tms_cstime  (children, terminated)
};

// @safe - times(2): sample the current process's CPU-time counters.
inline ProcessTimes process_times() noexcept {
    // @unsafe { libc times syscall + raw `struct tms*` argument; the
    //           struct lives on this stack frame. }
    {
        struct tms tms_buf;
        const std::int64_t wall =
            static_cast<std::int64_t>(::times(&tms_buf));
        return ProcessTimes{
            wall,
            static_cast<std::int64_t>(tms_buf.tms_utime),
            static_cast<std::int64_t>(tms_buf.tms_stime),
            static_cast<std::int64_t>(tms_buf.tms_cutime),
            static_cast<std::int64_t>(tms_buf.tms_cstime),
        };
    }
}

#if defined(__linux__)
// Aggregate of system-wide memory + uptime info as returned by sysinfo(2).
// Only the subset of `struct sysinfo` fields actually used in rrr is
// surfaced here; extend as new callers appear.
struct SysInfo {
    std::uint64_t uptime_sec;
    std::uint64_t total_ram_bytes;
    std::uint64_t free_ram_bytes;
    std::uint64_t shared_ram_bytes;
    std::uint64_t buffer_ram_bytes;
    std::uint64_t total_swap_bytes;
    std::uint64_t free_swap_bytes;
};

// @safe - sysinfo(2): sample system-wide memory + uptime info.
// Linux-only — guarded behind __linux__ at the header level. The
// returned values are scaled by `mem_unit` so callers see bytes.
inline SysInfo sysinfo() noexcept {
    // @unsafe { libc sysinfo syscall + raw `struct sysinfo*` argument;
    //           the struct lives on this stack frame. }
    {
        struct ::sysinfo info;
        ::sysinfo(&info);
        const std::uint64_t unit =
            static_cast<std::uint64_t>(info.mem_unit ? info.mem_unit : 1);
        return SysInfo{
            static_cast<std::uint64_t>(info.uptime),
            static_cast<std::uint64_t>(info.totalram)  * unit,
            static_cast<std::uint64_t>(info.freeram)   * unit,
            static_cast<std::uint64_t>(info.sharedram) * unit,
            static_cast<std::uint64_t>(info.bufferram) * unit,
            static_cast<std::uint64_t>(info.totalswap) * unit,
            static_cast<std::uint64_t>(info.freeswap)  * unit,
        };
    }
}
#endif  // __linux__

}  // namespace process
}  // namespace sys
}  // namespace rusty

#endif  // RUSTY_SYS_PROCESS_HPP
