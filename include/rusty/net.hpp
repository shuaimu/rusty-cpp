#ifndef RUSTY_NET_HPP
#define RUSTY_NET_HPP

namespace rusty::net {

// Minimal std::net compatibility surface for transpiled type paths.
// This can be expanded incrementally as parity work requires behavior.
struct TcpStream {
    TcpStream() = default;
    TcpStream(const TcpStream&) = default;
    TcpStream(TcpStream&&) noexcept = default;
    TcpStream& operator=(const TcpStream&) = default;
    TcpStream& operator=(TcpStream&&) noexcept = default;
    ~TcpStream() = default;
};

struct IpAddr {
    IpAddr() = default;
};

struct Ipv4Addr {
    Ipv4Addr() = default;
};

struct Ipv6Addr {
    Ipv6Addr() = default;
};

struct SocketAddr {
    SocketAddr() = default;
};

struct SocketAddrV4 {
    SocketAddrV4() = default;
};

struct SocketAddrV6 {
    SocketAddrV6() = default;
};

} // namespace rusty::net

#endif // RUSTY_NET_HPP
