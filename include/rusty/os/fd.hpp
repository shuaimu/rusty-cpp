#ifndef RUSTY_OS_FD_HPP
#define RUSTY_OS_FD_HPP

// rusty::os::fd — Rust-like owned/borrowed file descriptors for C++.
//
// Ports a minimal subset of Rust's `std::os::fd`:
//
//   OwnedFd        — owns a `int fd`, ::close()s it on drop.
//   BorrowedFd     — non-owning view over a `int fd` (does NOT close).
//
//   AsRawFd / IntoRawFd / FromRawFd          (free functions / methods)
//
//   try_clone(OwnedFd) -> Result<OwnedFd, io::Error>   — dup(2) wrapper.
//
// Naming + semantics mirror Rust's std::os::fd:
//   - `OwnedFd::from_raw_fd(int)` is `// @unsafe` — caller asserts the
//     fd is open and they're transferring ownership.
//   - `OwnedFd::into_raw_fd()` releases ownership without closing.
//   - `OwnedFd::as_raw_fd()` returns a borrowed view (the int).
//   - `BorrowedFd::borrow_raw(int)` is `// @safe` (just wraps the int;
//     the lifetime is the caller's responsibility, same as Rust where
//     BorrowedFd<'a> is parameterized by a lifetime that the borrow
//     checker enforces at the call site).
//
// All actual libc syscalls (`::close`, `::dup`) live in inline
// `// @unsafe { }` blocks so the public API can be called from @safe
// code.

#include <cerrno>
#include <cstring>
#include <unistd.h>

#include "rusty/io.hpp"     // for rusty::io::Error / Kind
#include "rusty/result.hpp"

namespace rusty {
namespace os {
namespace fd {

// Forward declaration.
class OwnedFd;

// @safe - Borrowed file descriptor — wraps a `int fd` without owning
// it. The fd's lifetime is the caller's responsibility. Mirrors Rust's
// `std::os::fd::BorrowedFd<'a>` (without the lifetime parameter, which
// in Rust is enforced by the borrow checker at use sites; in C++ the
// caller is expected to keep the source fd live for the duration of
// the borrow).
class BorrowedFd {
 public:
    // @safe - Wrap an existing fd. Caller asserts the fd is open and
    // will stay open for the duration of the borrow. Mirrors Rust's
    // `BorrowedFd::borrow_raw`.
    static BorrowedFd borrow_raw(int raw_fd) noexcept {
        return BorrowedFd(raw_fd);
    }

    // @safe - Get the underlying raw fd. No transfer of ownership.
    int as_raw_fd() const noexcept { return fd_; }

 private:
    explicit BorrowedFd(int fd) noexcept : fd_(fd) {}
    int fd_;
};

// @safe - Owned file descriptor — owns an `int fd` and ::close()s it
// on drop. Move-only (no copy). Mirrors Rust's `std::os::fd::OwnedFd`.
class OwnedFd {
 public:
    // @unsafe - Take ownership of an existing raw fd. Caller asserts
    // the fd is open and they're transferring ownership. Mirrors Rust's
    // `unsafe fn from_raw_fd(raw_fd: RawFd) -> OwnedFd`.
    static OwnedFd from_raw_fd(int raw_fd) noexcept {
        return OwnedFd(raw_fd);
    }

    // @safe - Releases ownership without closing. Returns the raw fd.
    // After this call, the OwnedFd is left in a "moved-from" state
    // (fd_ == -1) and its destructor is a no-op. Mirrors Rust's
    // `IntoRawFd::into_raw_fd`.
    int into_raw_fd() noexcept {
        int fd = fd_;
        fd_ = -1;
        return fd;
    }

    // @safe - Get the underlying raw fd. Does NOT transfer ownership;
    // the OwnedFd still closes it on drop. Mirrors Rust's
    // `AsRawFd::as_raw_fd`.
    int as_raw_fd() const noexcept { return fd_; }

    // @safe - Borrow this OwnedFd as a BorrowedFd. Mirrors Rust's
    // `OwnedFd::as_fd` (which returns `BorrowedFd<'_>`).
    BorrowedFd as_fd() const noexcept {
        return BorrowedFd::borrow_raw(fd_);
    }

    // @safe - True iff this OwnedFd actually owns an fd (fd_ >= 0).
    // Default-constructed / moved-from OwnedFds return false.
    bool is_valid() const noexcept { return fd_ >= 0; }

    // @safe - Move-only; destructor closes the fd.
    OwnedFd() noexcept : fd_(-1) {}
    OwnedFd(OwnedFd&& other) noexcept : fd_(other.fd_) { other.fd_ = -1; }
    OwnedFd& operator=(OwnedFd&& other) noexcept {
        if (this != &other) {
            close_if_valid();
            fd_ = other.fd_;
            other.fd_ = -1;
        }
        return *this;
    }

    OwnedFd(const OwnedFd&) = delete;
    OwnedFd& operator=(const OwnedFd&) = delete;

    // @safe - Destructor — ::close()s the fd if owned. The actual libc
    // call is wrapped in an inline `// @unsafe { }` block.
    ~OwnedFd() { close_if_valid(); }

    // @safe - Duplicate this fd via dup(2). Returns an Err with the
    // libc errno on failure (mapped to io::Error::Kind). Mirrors Rust's
    // `OwnedFd::try_clone`.
    rusty::Result<OwnedFd, rusty::io::Error> try_clone() const {
        if (fd_ < 0) {
            return rusty::Err<OwnedFd, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "try_clone: not a valid fd"));
        }
        int new_fd = -1;
        // @unsafe { ::dup is libc — duplicates the descriptor. }
        { new_fd = ::dup(fd_); }
        if (new_fd < 0) {
            return rusty::Err<OwnedFd, rusty::io::Error>(
                errno_to_io_error(errno, "dup"));
        }
        return rusty::Ok<OwnedFd, rusty::io::Error>(OwnedFd(new_fd));
    }

 private:
    explicit OwnedFd(int fd) noexcept : fd_(fd) {}

    // @safe - close() the fd if it's valid (>= 0). Retries on EINTR
    // since `close` can be interrupted on Linux. The actual libc call
    // is in an inline `// @unsafe { }` block.
    void close_if_valid() noexcept {
        if (fd_ < 0) return;
        // @unsafe { ::close is libc — releases the kernel fd. Loop on
        //           EINTR per POSIX; ignore any other error since
        //           there's nothing the dropper can do about it (and
        //           leaking a closed fd would also be wrong). }
        {
            while (::close(fd_) < 0) {
                if (errno != EINTR) break;
            }
        }
        fd_ = -1;
    }

    // @safe - Map a libc errno to an rusty::io::Error with the right
    // Kind. Defensive against unknown values — falls back to
    // io::Error::Kind::Other.
    static rusty::io::Error errno_to_io_error(int e, const char* op) {
        rusty::io::Error::Kind kind;
        switch (e) {
            case EACCES:       kind = rusty::io::Error::Kind::PermissionDenied; break;
            case EAGAIN:       kind = rusty::io::Error::Kind::WouldBlock; break;
            case EBADF:        kind = rusty::io::Error::Kind::InvalidInput; break;
            case ECONNREFUSED: kind = rusty::io::Error::Kind::ConnectionRefused; break;
            case ECONNRESET:   kind = rusty::io::Error::Kind::ConnectionReset; break;
            case ECONNABORTED: kind = rusty::io::Error::Kind::ConnectionAborted; break;
            case ENOENT:       kind = rusty::io::Error::Kind::NotFound; break;
            case ENOTCONN:     kind = rusty::io::Error::Kind::NotConnected; break;
            case EADDRINUSE:   kind = rusty::io::Error::Kind::AddrInUse; break;
            case EADDRNOTAVAIL:kind = rusty::io::Error::Kind::AddrNotAvailable; break;
            case EPIPE:        kind = rusty::io::Error::Kind::BrokenPipe; break;
            case EEXIST:       kind = rusty::io::Error::Kind::AlreadyExists; break;
            case EINVAL:       kind = rusty::io::Error::Kind::InvalidInput; break;
            case ETIMEDOUT:    kind = rusty::io::Error::Kind::TimedOut; break;
            case EINTR:        kind = rusty::io::Error::Kind::Interrupted; break;
            case ENOMEM:       kind = rusty::io::Error::Kind::OutOfMemory; break;
            case ENOSYS:
            case EOPNOTSUPP:   kind = rusty::io::Error::Kind::Unsupported; break;
            default:           kind = rusty::io::Error::Kind::Other; break;
        }
        std::string msg = op;
        msg += ": ";
        // @unsafe { strerror is libc — global thread-unsafe buffer. }
        { msg += std::strerror(e); }
        return rusty::io::Error(kind, std::move(msg));
    }

    int fd_;
};

// Free-function helpers matching Rust's AsRawFd / IntoRawFd / FromRawFd
// traits. In Rust these are trait methods; in C++ they sit at namespace
// scope so generic code can call them uniformly on OwnedFd / BorrowedFd.

// @safe - AsRawFd::as_raw_fd equivalent for OwnedFd.
inline int as_raw_fd(const OwnedFd& fd) noexcept { return fd.as_raw_fd(); }
// @safe - AsRawFd::as_raw_fd equivalent for BorrowedFd.
inline int as_raw_fd(const BorrowedFd& fd) noexcept { return fd.as_raw_fd(); }

}  // namespace fd
}  // namespace os
}  // namespace rusty

#endif  // RUSTY_OS_FD_HPP
