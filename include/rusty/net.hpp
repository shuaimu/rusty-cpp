#ifndef RUSTY_NET_HPP
#define RUSTY_NET_HPP

#include <rusty/result.hpp>
#include <rusty/enum_tags.hpp>   // rusty::detail::enum_variant_tags
#include <array>
#include <cstdint>
#include <string>
#include <string_view>
#include <utility>
#include <variant>

namespace rusty::net {

// std::net compatibility surface — address types only.
//
// The address types (Ipv4Addr / Ipv6Addr / SocketAddrV4 / SocketAddrV6
// / IpAddr / SocketAddr) live in this header as pure value types — no
// kernel resources, default-constructible, copyable.
//
// The operational `TcpListener` / `TcpStream` types — which own
// `os::fd::OwnedFd` and call socket / bind / connect / accept / read /
// write / shutdown / set_nonblocking — live in `rusty/net/tcp.hpp`
// (included via `rusty/rusty.hpp` and `rusty/rusty.cppm`). Keeping the
// operational class out of this header avoids cyclic includes with
// `rusty/os/fd.hpp` and keeps this header lightweight for transpiler
// emission paths that only need the value types.

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

class AddrParseError {
    enum class Kind { SocketV4 };
    Kind kind_;
    explicit AddrParseError(Kind kind) : kind_(kind) {}
    friend struct SocketAddrV4;

public:
    std::string to_string() const { return "invalid IPv4 socket address syntax"; }
    bool operator==(const AddrParseError&) const = default;
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

    static rusty::Result<SocketAddrV4, AddrParseError> from_str(std::string_view text);

    std::string to_string() const {
        const auto& bytes = ip_.octets();
        return std::to_string(bytes[0]) + "." + std::to_string(bytes[1]) + "." +
               std::to_string(bytes[2]) + "." + std::to_string(bytes[3]) + ":" +
               std::to_string(port_);
    }

    bool operator==(const SocketAddrV4& other) const = default;
};

inline rusty::Result<SocketAddrV4, AddrParseError>
SocketAddrV4::from_str(std::string_view text) {
    auto invalid = [] {
        return rusty::Err<SocketAddrV4, AddrParseError>(
            AddrParseError(AddrParseError::Kind::SocketV4));
    };
    std::array<std::uint8_t, 4> bytes{};
    std::size_t offset = 0;
    for (std::size_t index = 0; index < bytes.size(); ++index) {
        const auto start = offset;
        unsigned value = 0;
        while (offset < text.size() && text[offset] >= '0' && text[offset] <= '9') {
            value = value * 10 + static_cast<unsigned>(text[offset++] - '0');
            if (offset - start > 3 || value > 255) return invalid();
        }
        if (offset == start || (offset - start > 1 && text[start] == '0')) return invalid();
        bytes[index] = static_cast<std::uint8_t>(value);
        const char separator = index == 3 ? ':' : '.';
        if (offset == text.size() || text[offset++] != separator) return invalid();
    }
    if (offset == text.size()) return invalid();
    unsigned port = 0;
    for (; offset < text.size(); ++offset) {
        if (text[offset] < '0' || text[offset] > '9') return invalid();
        port = port * 10 + static_cast<unsigned>(text[offset] - '0');
        if (port > 65535) return invalid();
    }
    return rusty::Ok<SocketAddrV4, AddrParseError>(
        SocketAddrV4(Ipv4Addr(bytes), static_cast<std::uint16_t>(port)));
}

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

// Enum-lowering channel (see rusty::detail::enum_variant_tags): these lower to
// std::variant, so a foreign match arm naming `V4`/`V6` cannot refute with the
// enum-class form. Declared beside the enums so the two cannot drift apart.
} // namespace rusty::net

namespace rusty { namespace detail {
template<>
struct enum_variant_tags<::rusty::net::IpAddr> {
    using V4 = ::rusty::net::IpAddr_V4;
    using V6 = ::rusty::net::IpAddr_V6;
};
template<>
struct enum_variant_tags<::rusty::net::SocketAddr> {
    using V4 = ::rusty::net::SocketAddr_V4;
    using V6 = ::rusty::net::SocketAddr_V6;
};
} } // namespace rusty::detail

#endif // RUSTY_NET_HPP
