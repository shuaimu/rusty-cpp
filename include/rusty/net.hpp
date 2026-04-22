#ifndef RUSTY_NET_HPP
#define RUSTY_NET_HPP

#include <array>
#include <cstdint>
#include <utility>
#include <variant>

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

struct Ipv4Addr {
    std::array<std::uint8_t, 4> bytes_{};

    Ipv4Addr() = default;
    explicit Ipv4Addr(std::array<std::uint8_t, 4> bytes) : bytes_(std::move(bytes)) {}

    static Ipv4Addr new_(
        std::uint8_t a,
        std::uint8_t b,
        std::uint8_t c,
        std::uint8_t d
    ) {
        return Ipv4Addr(std::array<std::uint8_t, 4>{a, b, c, d});
    }

    const std::array<std::uint8_t, 4>& octets() const {
        return bytes_;
    }

    bool operator==(const Ipv4Addr& other) const = default;
};

struct Ipv6Addr {
    std::array<std::uint8_t, 16> bytes_{};

    Ipv6Addr() = default;
    explicit Ipv6Addr(std::array<std::uint8_t, 16> bytes) : bytes_(std::move(bytes)) {}

    const std::array<std::uint8_t, 16>& octets() const {
        return bytes_;
    }

    bool operator==(const Ipv6Addr& other) const = default;
};

struct SocketAddrV4 {
    Ipv4Addr ip_{};
    std::uint16_t port_{0};

    SocketAddrV4() = default;
    SocketAddrV4(Ipv4Addr ip, std::uint16_t port) : ip_(std::move(ip)), port_(port) {}

    static SocketAddrV4 new_(Ipv4Addr ip, std::uint16_t port) {
        return SocketAddrV4(std::move(ip), port);
    }

    const Ipv4Addr& ip() const {
        return ip_;
    }

    std::uint16_t port() const {
        return port_;
    }

    bool operator==(const SocketAddrV4& other) const = default;
};

struct SocketAddrV6 {
    Ipv6Addr ip_{};
    std::uint16_t port_{0};
    std::uint32_t flowinfo_{0};
    std::uint32_t scope_id_{0};

    SocketAddrV6() = default;
    SocketAddrV6(
        Ipv6Addr ip,
        std::uint16_t port,
        std::uint32_t flowinfo = 0,
        std::uint32_t scope_id = 0
    )
        : ip_(std::move(ip)), port_(port), flowinfo_(flowinfo), scope_id_(scope_id) {}

    static SocketAddrV6 new_(
        Ipv6Addr ip,
        std::uint16_t port,
        std::uint32_t flowinfo,
        std::uint32_t scope_id
    ) {
        return SocketAddrV6(std::move(ip), port, flowinfo, scope_id);
    }

    const Ipv6Addr& ip() const {
        return ip_;
    }

    std::uint16_t port() const {
        return port_;
    }

    bool operator==(const SocketAddrV6& other) const = default;
};

struct IpAddr_V4 {
    Ipv4Addr _0;
    explicit IpAddr_V4(Ipv4Addr value) : _0(std::move(value)) {}
    bool operator==(const IpAddr_V4& other) const = default;
};

struct IpAddr_V6 {
    Ipv6Addr _0;
    explicit IpAddr_V6(Ipv6Addr value) : _0(std::move(value)) {}
    bool operator==(const IpAddr_V6& other) const = default;
};

using IpAddr = std::variant<IpAddr_V4, IpAddr_V6>;

struct SocketAddr_V4 {
    SocketAddrV4 _0;
    explicit SocketAddr_V4(SocketAddrV4 value) : _0(std::move(value)) {}
    bool operator==(const SocketAddr_V4& other) const = default;
};

struct SocketAddr_V6 {
    SocketAddrV6 _0;
    explicit SocketAddr_V6(SocketAddrV6 value) : _0(std::move(value)) {}
    bool operator==(const SocketAddr_V6& other) const = default;
};

using SocketAddr = std::variant<SocketAddr_V4, SocketAddr_V6>;

} // namespace rusty::net

#endif // RUSTY_NET_HPP
