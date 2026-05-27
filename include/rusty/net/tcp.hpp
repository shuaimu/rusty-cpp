#ifndef RUSTY_NET_TCP_HPP
#define RUSTY_NET_TCP_HPP

// rusty::net::{TcpListener, TcpStream} — a port of Rust's std::net::*
// TCP socket types, built on rusty::os::fd::OwnedFd.
//
// Mirrors:
//   std::net::TcpListener::bind / accept / local_addr / set_nonblocking
//   std::net::TcpStream::connect / shutdown / set_nonblocking /
//                       peer_addr / local_addr
//   std::net::Shutdown   (Read / Write / Both)
//
// Read / Write methods on TcpStream live on the type directly rather
// than via trait impls — they take `std::span<uint8_t>` and return
// `Result<size_t, io::Error>`, matching the `io::read` / `io::write`
// dispatcher in `rusty/io.hpp`.
//
// SocketAddrV4 parse / format / sockaddr_in conversion helpers live
// here too because they're tightly coupled to the socket API.
//
// All libc syscalls (`::socket`, `::bind`, `::listen`, `::accept`,
// `::connect`, `::shutdown`, `::recv`, `::send`, `::getsockname`,
// `::setsockopt`, `::fcntl`) sit in inline `// @unsafe { }` blocks so
// the public API can be called from @safe code.

#include <arpa/inet.h>
#include <cerrno>
#include <cstdint>
#include <cstring>
#include <fcntl.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <string>
#include <string_view>
#include <span>
#include <utility>

#include "rusty/io.hpp"
#include "rusty/net.hpp"
#include "rusty/os/fd.hpp"
#include "rusty/result.hpp"

namespace rusty {
namespace net {

// ── Shutdown ───────────────────────────────────────────
//
// Mirrors std::net::Shutdown.
enum class Shutdown {
    Read,
    Write,
    Both,
};

// ── sockaddr_in <-> SocketAddrV4 ────────────────────────
//
// These are internal helpers — not part of Rust's public std::net API
// (Rust hides sockaddr_in behind libc::*); exposed here so callers
// integrating with foreign C APIs (epoll loops, etc.) can convert.

// @safe - pure value extraction from a `sockaddr_in` POD.
inline SocketAddrV4 socket_addr_v4_from_sockaddr_in(const ::sockaddr_in& sa) {
    // sa.sin_addr.s_addr is in network byte order. ntohl converts to
    // host order; we then pack as octets matching Rust's
    // Ipv4Addr::new(a, b, c, d) convention.
    std::uint32_t host_order;
    // @unsafe { ntohl is a libc macro/intrinsic. }
    { host_order = ::ntohl(sa.sin_addr.s_addr); }
    std::uint8_t a = static_cast<std::uint8_t>((host_order >> 24) & 0xff);
    std::uint8_t b = static_cast<std::uint8_t>((host_order >> 16) & 0xff);
    std::uint8_t c = static_cast<std::uint8_t>((host_order >>  8) & 0xff);
    std::uint8_t d = static_cast<std::uint8_t>( host_order        & 0xff);
    std::uint16_t port;
    // @unsafe { ntohs is a libc macro/intrinsic. }
    { port = ::ntohs(sa.sin_port); }
    return SocketAddrV4::new_(Ipv4Addr::new_(a, b, c, d), port);
}

// @safe - pure value extraction into a `sockaddr_in` POD.
inline ::sockaddr_in sockaddr_in_from_socket_addr_v4(const SocketAddrV4& addr) {
    ::sockaddr_in sa;
    // @unsafe { std::memset on a libc POD. }
    { std::memset(&sa, 0, sizeof(sa)); }
    sa.sin_family = AF_INET;
    const auto& octets = addr.ip().octets();
    std::uint32_t host_order =
        (static_cast<std::uint32_t>(octets[0]) << 24) |
        (static_cast<std::uint32_t>(octets[1]) << 16) |
        (static_cast<std::uint32_t>(octets[2]) <<  8) |
        static_cast<std::uint32_t>(octets[3]);
    // @unsafe { htonl / htons are libc macros/intrinsics. }
    {
        sa.sin_addr.s_addr = ::htonl(host_order);
        sa.sin_port = ::htons(addr.port());
    }
    return sa;
}

// ── SocketAddrV4 parse / format ────────────────────────

// @safe - Parse "1.2.3.4:port" via inet_pton + strtoul. Returns
// io::Error(InvalidInput) on malformed input. Mirrors Rust's
// `impl FromStr for SocketAddrV4`.
inline rusty::Result<SocketAddrV4, rusty::io::Error>
socket_addr_v4_from_str(std::string_view s) {
    auto colon = s.find_last_of(':');
    if (colon == std::string_view::npos) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "SocketAddrV4: missing ':' separator"));
    }
    std::string host(s.substr(0, colon));
    std::string port_str(s.substr(colon + 1));
    if (host.empty() || port_str.empty()) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "SocketAddrV4: empty host or port"));
    }

    long port_long;
    // @unsafe { std::strtol is libc — sets errno on out-of-range. }
    {
        char* end = nullptr;
        port_long = std::strtol(port_str.c_str(), &end, 10);
        if (end == port_str.c_str() || *end != '\0') {
            return rusty::Err<SocketAddrV4, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "SocketAddrV4: malformed port"));
        }
    }
    if (port_long < 0 || port_long > 65535) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "SocketAddrV4: port out of range"));
    }

    ::in_addr binary;
    // @unsafe { inet_pton is libc — parses dotted-quad into a network-
    //           order ::in_addr POD. }
    int rc;
    { rc = ::inet_pton(AF_INET, host.c_str(), &binary); }
    if (rc != 1) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "SocketAddrV4: invalid IPv4 literal"));
    }
    std::uint32_t host_order;
    // @unsafe { ntohl libc. }
    { host_order = ::ntohl(binary.s_addr); }
    std::uint8_t a = static_cast<std::uint8_t>((host_order >> 24) & 0xff);
    std::uint8_t b = static_cast<std::uint8_t>((host_order >> 16) & 0xff);
    std::uint8_t c = static_cast<std::uint8_t>((host_order >>  8) & 0xff);
    std::uint8_t d = static_cast<std::uint8_t>( host_order        & 0xff);
    return rusty::Ok<SocketAddrV4, rusty::io::Error>(
        SocketAddrV4::new_(Ipv4Addr::new_(a, b, c, d),
                           static_cast<std::uint16_t>(port_long)));
}

// @safe - Format SocketAddrV4 as "1.2.3.4:port". Mirrors Rust's
// `impl Display for SocketAddrV4`.
inline std::string socket_addr_v4_to_string(const SocketAddrV4& addr) {
    char buf[INET_ADDRSTRLEN + 8];  // "xxx.xxx.xxx.xxx:65535\0" fits
    const auto& octets = addr.ip().octets();
    // @unsafe { std::snprintf is libc. }
    {
        std::snprintf(buf, sizeof(buf), "%u.%u.%u.%u:%u",
                      static_cast<unsigned>(octets[0]),
                      static_cast<unsigned>(octets[1]),
                      static_cast<unsigned>(octets[2]),
                      static_cast<unsigned>(octets[3]),
                      static_cast<unsigned>(addr.port()));
    }
    return std::string(buf);
}

// ── Internal: errno -> io::Error::Kind mapping ─────────

namespace detail {

// @safe - libc errno -> io::Error::Kind mapping. Mirrors Rust's
// `decode_error_kind` in libstd. Centralized so every socket call
// reports consistently.
inline rusty::io::Error errno_to_io_error(int e, const char* op) {
    rusty::io::Error::Kind kind;
    switch (e) {
        case EACCES:        kind = rusty::io::Error::Kind::PermissionDenied; break;
        case EAGAIN:        kind = rusty::io::Error::Kind::WouldBlock; break;
        case EBADF:         kind = rusty::io::Error::Kind::InvalidInput; break;
        case ECONNREFUSED:  kind = rusty::io::Error::Kind::ConnectionRefused; break;
        case ECONNRESET:    kind = rusty::io::Error::Kind::ConnectionReset; break;
        case ECONNABORTED:  kind = rusty::io::Error::Kind::ConnectionAborted; break;
        case ENOTCONN:      kind = rusty::io::Error::Kind::NotConnected; break;
        case EADDRINUSE:    kind = rusty::io::Error::Kind::AddrInUse; break;
        case EADDRNOTAVAIL: kind = rusty::io::Error::Kind::AddrNotAvailable; break;
        case EPIPE:         kind = rusty::io::Error::Kind::BrokenPipe; break;
        case EEXIST:        kind = rusty::io::Error::Kind::AlreadyExists; break;
        case EINVAL:        kind = rusty::io::Error::Kind::InvalidInput; break;
        case ETIMEDOUT:     kind = rusty::io::Error::Kind::TimedOut; break;
        case EINTR:         kind = rusty::io::Error::Kind::Interrupted; break;
        case ENOMEM:        kind = rusty::io::Error::Kind::OutOfMemory; break;
        case ENOSYS:
        case EOPNOTSUPP:    kind = rusty::io::Error::Kind::Unsupported; break;
        default:            kind = rusty::io::Error::Kind::Other; break;
    }
    std::string msg = op;
    msg += ": ";
    // @unsafe { strerror is libc — uses global thread-unsafe buffer. }
    { msg += std::strerror(e); }
    return rusty::io::Error(kind, std::move(msg));
}

// @safe - getsockname on an existing fd, returning the bound
// SocketAddrV4. Shared by TcpListener::local_addr,
// TcpStream::local_addr, and the TcpListener::bind body itself.
inline rusty::Result<SocketAddrV4, rusty::io::Error>
getsockname_v4(const rusty::os::fd::OwnedFd& fd) {
    if (!fd.is_valid()) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "getsockname: not a valid fd"));
    }
    ::sockaddr_in sa;
    ::socklen_t len = sizeof(sa);
    // @unsafe { std::memset + ::getsockname syscall. }
    int rc;
    {
        std::memset(&sa, 0, sizeof(sa));
        rc = ::getsockname(fd.as_raw_fd(),
                           reinterpret_cast<::sockaddr*>(&sa), &len);
    }
    if (rc != 0) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            errno_to_io_error(errno, "getsockname"));
    }
    if (sa.sin_family != AF_INET) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::Unsupported,
                             "getsockname: address family is not AF_INET"));
    }
    return rusty::Ok<SocketAddrV4, rusty::io::Error>(
        socket_addr_v4_from_sockaddr_in(sa));
}

// @safe - getpeername on an existing fd. Mirrors getsockname_v4 above.
inline rusty::Result<SocketAddrV4, rusty::io::Error>
getpeername_v4(const rusty::os::fd::OwnedFd& fd) {
    if (!fd.is_valid()) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                             "getpeername: not a valid fd"));
    }
    ::sockaddr_in sa;
    ::socklen_t len = sizeof(sa);
    int rc;
    // @unsafe { std::memset + ::getpeername syscall. }
    {
        std::memset(&sa, 0, sizeof(sa));
        rc = ::getpeername(fd.as_raw_fd(),
                           reinterpret_cast<::sockaddr*>(&sa), &len);
    }
    if (rc != 0) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            errno_to_io_error(errno, "getpeername"));
    }
    if (sa.sin_family != AF_INET) {
        return rusty::Err<SocketAddrV4, rusty::io::Error>(
            rusty::io::Error(rusty::io::Error::Kind::Unsupported,
                             "getpeername: address family is not AF_INET"));
    }
    return rusty::Ok<SocketAddrV4, rusty::io::Error>(
        socket_addr_v4_from_sockaddr_in(sa));
}

// @safe - fcntl(F_GETFL) + fcntl(F_SETFL, O_NONBLOCK | ...).
// Shared by Tcp{Listener,Stream}::set_nonblocking.
inline rusty::Result<void, rusty::io::Error>
fd_set_nonblocking(int raw_fd, bool nonblocking) {
    int flags;
    // @unsafe { fcntl is libc. }
    { flags = ::fcntl(raw_fd, F_GETFL, 0); }
    if (flags < 0) {
        return rusty::Result<void, rusty::io::Error>::Err(
            errno_to_io_error(errno, "fcntl(F_GETFL)"));
    }
    if (nonblocking) {
        flags |= O_NONBLOCK;
    } else {
        flags &= ~O_NONBLOCK;
    }
    int rc;
    // @unsafe { fcntl is libc. }
    { rc = ::fcntl(raw_fd, F_SETFL, flags); }
    if (rc < 0) {
        return rusty::Result<void, rusty::io::Error>::Err(
            errno_to_io_error(errno, "fcntl(F_SETFL)"));
    }
    return rusty::Result<void, rusty::io::Error>::Ok();
}

}  // namespace detail

// ── TcpStream ──────────────────────────────────────────
//
// Mirrors std::net::TcpStream. Owns an OwnedFd; closes on drop.
//
// Default-constructible to an "empty" state (no fd) — methods on an
// empty TcpStream return io::Error::Kind::InvalidInput. This keeps
// compatibility with the original stub (which is default-constructed
// by the transpiler's generic VecLegacy instantiation).

class TcpStream {
 public:
    // @safe - Default-construct an empty (unconnected) stream.
    TcpStream() = default;

    // @safe - Take ownership of a connected fd. Internal; production
    // callers use `connect()`.
    explicit TcpStream(rusty::os::fd::OwnedFd fd) : fd_(std::move(fd)) {}

    // Move-only.
    TcpStream(TcpStream&&) noexcept = default;
    TcpStream& operator=(TcpStream&&) noexcept = default;
    TcpStream(const TcpStream&) = delete;
    TcpStream& operator=(const TcpStream&) = delete;
    ~TcpStream() = default;

    // @safe - Connect to a SocketAddrV4. Calls socket(2) + connect(2).
    // Mirrors `std::net::TcpStream::connect`. Returns the connected
    // stream on success, or an io::Error mapped from libc errno.
    static rusty::Result<TcpStream, rusty::io::Error>
    connect(const SocketAddrV4& addr) {
        int raw_fd;
        // @unsafe { ::socket libc syscall. }
        { raw_fd = ::socket(AF_INET, SOCK_STREAM, 0); }
        if (raw_fd < 0) {
            return rusty::Err<TcpStream, rusty::io::Error>(
                detail::errno_to_io_error(errno, "socket"));
        }
        rusty::os::fd::OwnedFd fd =
            rusty::os::fd::OwnedFd::from_raw_fd(raw_fd);
        ::sockaddr_in sa = sockaddr_in_from_socket_addr_v4(addr);
        int rc;
        // @unsafe { ::connect libc syscall. }
        {
            rc = ::connect(fd.as_raw_fd(),
                           reinterpret_cast<const ::sockaddr*>(&sa),
                           sizeof(sa));
        }
        if (rc != 0) {
            return rusty::Err<TcpStream, rusty::io::Error>(
                detail::errno_to_io_error(errno, "connect"));
        }
        return rusty::Ok<TcpStream, rusty::io::Error>(TcpStream(std::move(fd)));
    }

    // @safe - Read up to buf.size() bytes from the stream. Mirrors
    // `std::net::TcpStream::read`. Returns the actual byte count read
    // (0 on EOF), or io::Error on failure.
    rusty::Result<std::size_t, rusty::io::Error> read(std::span<std::uint8_t> buf) {
        if (!fd_.is_valid()) {
            return rusty::Err<std::size_t, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "read: stream not connected"));
        }
        ssize_t n;
        // @unsafe { ::recv libc syscall. }
        { n = ::recv(fd_.as_raw_fd(), buf.data(), buf.size(), 0); }
        if (n < 0) {
            return rusty::Err<std::size_t, rusty::io::Error>(
                detail::errno_to_io_error(errno, "recv"));
        }
        return rusty::Ok<std::size_t, rusty::io::Error>(static_cast<std::size_t>(n));
    }

    // @safe - Write up to buf.size() bytes to the stream. Mirrors
    // `std::net::TcpStream::write`. Uses MSG_NOSIGNAL to avoid
    // SIGPIPE on broken-pipe writes (the error surfaces as
    // io::Error::Kind::BrokenPipe instead).
    rusty::Result<std::size_t, rusty::io::Error> write(std::span<const std::uint8_t> buf) {
        if (!fd_.is_valid()) {
            return rusty::Err<std::size_t, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "write: stream not connected"));
        }
        ssize_t n;
        // @unsafe { ::send libc syscall. }
        {
#ifdef MSG_NOSIGNAL
            n = ::send(fd_.as_raw_fd(), buf.data(), buf.size(), MSG_NOSIGNAL);
#else
            n = ::send(fd_.as_raw_fd(), buf.data(), buf.size(), 0);
#endif
        }
        if (n < 0) {
            return rusty::Err<std::size_t, rusty::io::Error>(
                detail::errno_to_io_error(errno, "send"));
        }
        return rusty::Ok<std::size_t, rusty::io::Error>(static_cast<std::size_t>(n));
    }

    // @safe - Shut down the read, write, or both halves of the
    // connection. Mirrors `std::net::TcpStream::shutdown`.
    rusty::Result<void, rusty::io::Error> shutdown(Shutdown how) {
        if (!fd_.is_valid()) {
            return rusty::Result<void, rusty::io::Error>::Err(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "shutdown: stream not connected"));
        }
        int how_flag;
        switch (how) {
            case Shutdown::Read:  how_flag = SHUT_RD;   break;
            case Shutdown::Write: how_flag = SHUT_WR;   break;
            case Shutdown::Both:  how_flag = SHUT_RDWR; break;
        }
        int rc;
        // @unsafe { ::shutdown libc syscall. }
        { rc = ::shutdown(fd_.as_raw_fd(), how_flag); }
        if (rc != 0) {
            return rusty::Result<void, rusty::io::Error>::Err(
                detail::errno_to_io_error(errno, "shutdown"));
        }
        return rusty::Result<void, rusty::io::Error>::Ok();
    }

    // @safe - Set the socket's non-blocking mode via
    // fcntl(F_GETFL/F_SETFL). Mirrors `std::net::TcpStream::set_nonblocking`.
    rusty::Result<void, rusty::io::Error> set_nonblocking(bool nonblocking) {
        if (!fd_.is_valid()) {
            return rusty::Result<void, rusty::io::Error>::Err(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "set_nonblocking: stream not connected"));
        }
        return detail::fd_set_nonblocking(fd_.as_raw_fd(), nonblocking);
    }

    // @safe - Return the local endpoint via ::getsockname. Mirrors
    // `std::net::TcpStream::local_addr`.
    rusty::Result<SocketAddrV4, rusty::io::Error> local_addr() const {
        return detail::getsockname_v4(fd_);
    }

    // @safe - Return the peer endpoint via ::getpeername. Mirrors
    // `std::net::TcpStream::peer_addr`.
    rusty::Result<SocketAddrV4, rusty::io::Error> peer_addr() const {
        return detail::getpeername_v4(fd_);
    }

    // @safe - True iff the stream owns a valid fd.
    bool is_connected() const noexcept { return fd_.is_valid(); }

    // @safe - Borrow the inner fd. Used by foreign event loops
    // (epoll / kqueue / io_uring) that want to register the socket
    // without taking ownership. Mirrors Rust's `AsRawFd for TcpStream`.
    const rusty::os::fd::OwnedFd& as_owned_fd() const noexcept { return fd_; }

    // @unsafe - Release the inner fd, leaving the TcpStream empty.
    // Mirrors Rust's `IntoRawFd for TcpStream`.
    rusty::os::fd::OwnedFd into_owned_fd() noexcept {
        return std::move(fd_);
    }

 private:
    rusty::os::fd::OwnedFd fd_;
};

// ── TcpListener ────────────────────────────────────────
//
// Mirrors std::net::TcpListener. Owns an OwnedFd; closes on drop.

class TcpListener {
 public:
    // @safe - Default-construct an empty listener (no fd).
    TcpListener() = default;

    // Move-only.
    TcpListener(TcpListener&&) noexcept = default;
    TcpListener& operator=(TcpListener&&) noexcept = default;
    TcpListener(const TcpListener&) = delete;
    TcpListener& operator=(const TcpListener&) = delete;
    ~TcpListener() = default;

    // @safe - Bind to an address. Calls socket(2) + setsockopt(2)
    // (SO_REUSEADDR) + bind(2) + listen(2). Mirrors
    // `std::net::TcpListener::bind`. Returns the listening socket on
    // success, or an io::Error mapped from libc errno.
    //
    // backlog defaults to 128 — matches Rust's std::net default
    // (which delegates to libc's listen(fd, 128)).
    static rusty::Result<TcpListener, rusty::io::Error>
    bind(const SocketAddrV4& addr, int backlog = 128) {
        int raw_fd;
        // @unsafe { ::socket libc syscall. }
        { raw_fd = ::socket(AF_INET, SOCK_STREAM, 0); }
        if (raw_fd < 0) {
            return rusty::Err<TcpListener, rusty::io::Error>(
                detail::errno_to_io_error(errno, "socket"));
        }
        rusty::os::fd::OwnedFd fd =
            rusty::os::fd::OwnedFd::from_raw_fd(raw_fd);

        // SO_REUSEADDR — match Rust's default. Without this, restarting
        // a server quickly fails with EADDRINUSE due to TIME_WAIT.
        int reuse = 1;
        int rc;
        // @unsafe { ::setsockopt libc syscall. }
        {
            rc = ::setsockopt(fd.as_raw_fd(), SOL_SOCKET, SO_REUSEADDR,
                              &reuse, sizeof(reuse));
        }
        if (rc != 0) {
            return rusty::Err<TcpListener, rusty::io::Error>(
                detail::errno_to_io_error(errno, "setsockopt(SO_REUSEADDR)"));
        }

        ::sockaddr_in sa = sockaddr_in_from_socket_addr_v4(addr);
        // @unsafe { ::bind libc syscall. }
        {
            rc = ::bind(fd.as_raw_fd(),
                        reinterpret_cast<const ::sockaddr*>(&sa),
                        sizeof(sa));
        }
        if (rc != 0) {
            return rusty::Err<TcpListener, rusty::io::Error>(
                detail::errno_to_io_error(errno, "bind"));
        }

        // @unsafe { ::listen libc syscall. }
        { rc = ::listen(fd.as_raw_fd(), backlog); }
        if (rc != 0) {
            return rusty::Err<TcpListener, rusty::io::Error>(
                detail::errno_to_io_error(errno, "listen"));
        }

        TcpListener result;
        result.fd_ = std::move(fd);
        return rusty::Ok<TcpListener, rusty::io::Error>(std::move(result));
    }

    // @safe - Accept a single incoming connection. Mirrors
    // `std::net::TcpListener::accept` — returns
    // `(TcpStream, SocketAddrV4)` on success.
    rusty::Result<std::pair<TcpStream, SocketAddrV4>, rusty::io::Error>
    accept() {
        if (!fd_.is_valid()) {
            return rusty::Err<std::pair<TcpStream, SocketAddrV4>, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "accept: listener not bound"));
        }
        ::sockaddr_in peer;
        ::socklen_t peer_len = sizeof(peer);
        int conn_fd;
        // @unsafe { ::memset + ::accept libc syscall. }
        {
            std::memset(&peer, 0, sizeof(peer));
            conn_fd = ::accept(fd_.as_raw_fd(),
                               reinterpret_cast<::sockaddr*>(&peer),
                               &peer_len);
        }
        if (conn_fd < 0) {
            return rusty::Err<std::pair<TcpStream, SocketAddrV4>, rusty::io::Error>(
                detail::errno_to_io_error(errno, "accept"));
        }
        TcpStream stream(rusty::os::fd::OwnedFd::from_raw_fd(conn_fd));
        SocketAddrV4 peer_addr = socket_addr_v4_from_sockaddr_in(peer);
        return rusty::Ok<std::pair<TcpStream, SocketAddrV4>, rusty::io::Error>(
            std::make_pair(std::move(stream), peer_addr));
    }

    // @safe - Return the local endpoint via ::getsockname. Mirrors
    // `std::net::TcpListener::local_addr`. Useful when binding to
    // port 0 to discover the kernel-assigned ephemeral port.
    rusty::Result<SocketAddrV4, rusty::io::Error> local_addr() const {
        return detail::getsockname_v4(fd_);
    }

    // @safe - Set the listener's non-blocking mode. Mirrors
    // `std::net::TcpListener::set_nonblocking`.
    rusty::Result<void, rusty::io::Error> set_nonblocking(bool nonblocking) {
        if (!fd_.is_valid()) {
            return rusty::Result<void, rusty::io::Error>::Err(
                rusty::io::Error(rusty::io::Error::Kind::InvalidInput,
                                 "set_nonblocking: listener not bound"));
        }
        return detail::fd_set_nonblocking(fd_.as_raw_fd(), nonblocking);
    }

    // @safe - True iff the listener owns a valid fd.
    bool is_bound() const noexcept { return fd_.is_valid(); }

    // @safe - Borrow the inner fd for foreign event loops.
    const rusty::os::fd::OwnedFd& as_owned_fd() const noexcept { return fd_; }

    // @unsafe - Release the inner fd.
    rusty::os::fd::OwnedFd into_owned_fd() noexcept {
        return std::move(fd_);
    }

 private:
    rusty::os::fd::OwnedFd fd_;
};

}  // namespace net
}  // namespace rusty

#endif  // RUSTY_NET_TCP_HPP
