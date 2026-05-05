#pragma once

// Backend selection for rusty runtime platform primitives.
//
// Default: C++ standard library backend.
// Opt-in: define RUSTY_PLATFORM_BACKEND_POSIX=1 to use pthread/pthreads-based
// synchronization primitives (on POSIX targets only).

#if defined(RUSTY_PLATFORM_BACKEND_POSIX) && defined(_WIN32)
#  error "RUSTY_PLATFORM_BACKEND_POSIX is not supported on Windows"
#endif

namespace rusty::platform {

enum class Backend {
    CppStd,
    Posix,
};

#if defined(RUSTY_PLATFORM_BACKEND_POSIX)
inline constexpr Backend kActiveBackend = Backend::Posix;
#else
inline constexpr Backend kActiveBackend = Backend::CppStd;
#endif

inline constexpr bool kUsePosixBackend = (kActiveBackend == Backend::Posix);

} // namespace rusty::platform
