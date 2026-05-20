// Tests for rusty::net::{TcpListener, TcpStream, SocketAddrV4}.
//
// Most tests use a localhost loopback pair: bind on 127.0.0.1:0
// (kernel-assigned ephemeral port), discover the port via
// local_addr, then connect a TcpStream to it and exchange bytes.

#include "../include/rusty/net/tcp.hpp"

#include <array>
#include <cassert>
#include <cstdio>
#include <cstring>
#include <span>
#include <thread>
#include <utility>

using rusty::net::Ipv4Addr;
using rusty::net::SocketAddrV4;
using rusty::net::TcpListener;
using rusty::net::TcpStream;
using rusty::net::Shutdown;
using rusty::net::socket_addr_v4_from_str;
using rusty::net::socket_addr_v4_to_string;

void test_socket_addr_v4_from_str_roundtrip() {
    printf("test_socket_addr_v4_from_str_roundtrip: ");
    auto result = socket_addr_v4_from_str("127.0.0.1:8080");
    assert(result.is_ok());
    auto addr = result.unwrap();
    assert(addr.ip().octets() == (std::array<std::uint8_t,4>{127, 0, 0, 1}));
    assert(addr.port() == 8080);
    assert(socket_addr_v4_to_string(addr) == "127.0.0.1:8080");
    printf("PASS\n");
}

void test_socket_addr_v4_from_str_rejects_garbage() {
    printf("test_socket_addr_v4_from_str_rejects_garbage: ");
    assert(socket_addr_v4_from_str("not-an-address").is_err());
    assert(socket_addr_v4_from_str("127.0.0.1").is_err());     // no port
    assert(socket_addr_v4_from_str(":8080").is_err());         // empty host
    assert(socket_addr_v4_from_str("127.0.0.1:").is_err());    // empty port
    assert(socket_addr_v4_from_str("127.0.0.1:abc").is_err()); // bad port
    assert(socket_addr_v4_from_str("127.0.0.1:99999").is_err());  // out of range
    assert(socket_addr_v4_from_str("256.0.0.1:80").is_err());  // bad octet
    printf("PASS\n");
}

void test_bind_local_addr_returns_assigned_port() {
    printf("test_bind_local_addr_returns_assigned_port: ");
    auto bind_result = TcpListener::bind(
        SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 0));
    assert(bind_result.is_ok());
    auto listener = bind_result.unwrap();
    assert(listener.is_bound());

    auto local_result = listener.local_addr();
    assert(local_result.is_ok());
    auto local = local_result.unwrap();
    assert(local.ip().octets() == (std::array<std::uint8_t,4>{127, 0, 0, 1}));
    assert(local.port() != 0);  // kernel assigned a real port
    printf("PASS (port=%u)\n", local.port());
}

void test_connect_accept_exchange_bytes() {
    printf("test_connect_accept_exchange_bytes: ");
    auto listener_result = TcpListener::bind(
        SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 0));
    assert(listener_result.is_ok());
    auto listener = listener_result.unwrap();
    auto bound = listener.local_addr().unwrap();

    // Connect from another thread so the accept side doesn't deadlock.
    std::thread connector([bound]() {
        auto stream_result = TcpStream::connect(bound);
        assert(stream_result.is_ok());
        auto stream = stream_result.unwrap();

        const std::uint8_t msg[] = {'h', 'i', '!'};
        auto wrote = stream.write(
            std::span<const std::uint8_t>(msg, sizeof(msg)));
        assert(wrote.is_ok());
        assert(wrote.unwrap() == 3);

        // Wait briefly for the peer to read before dropping.
        std::array<std::uint8_t, 4> echo;
        auto got = stream.read(std::span<std::uint8_t>(echo));
        assert(got.is_ok());
        assert(got.unwrap() == 3);
        assert(echo[0] == 'h' && echo[1] == 'i' && echo[2] == '!');
    });

    auto accept_result = listener.accept();
    assert(accept_result.is_ok());
    auto pair = accept_result.unwrap();
    auto& conn = pair.first;
    auto& peer = pair.second;
    assert(conn.is_connected());
    assert(peer.ip().octets() == (std::array<std::uint8_t,4>{127, 0, 0, 1}));

    std::array<std::uint8_t, 8> buf{};
    auto read_result = conn.read(std::span<std::uint8_t>(buf));
    assert(read_result.is_ok());
    assert(read_result.unwrap() == 3);
    assert(buf[0] == 'h' && buf[1] == 'i' && buf[2] == '!');

    // Echo back.
    auto wrote = conn.write(std::span<const std::uint8_t>(buf.data(), 3));
    assert(wrote.is_ok());
    assert(wrote.unwrap() == 3);

    connector.join();
    printf("PASS\n");
}

void test_connect_to_nonexistent_port_returns_error() {
    printf("test_connect_to_nonexistent_port_returns_error: ");
    // Pick a high port unlikely to be in use; let the kernel tell us
    // it's refused.
    auto stream_result = TcpStream::connect(
        SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 1));  // privileged port; nobody listening
    assert(stream_result.is_err());
    auto err = stream_result.unwrap_err();
    // Expect ConnectionRefused or PermissionDenied or AddrNotAvailable
    // (some sandboxes give different mappings).
    auto kind = err.kind();
    bool ok = kind == rusty::io::Error::Kind::ConnectionRefused
           || kind == rusty::io::Error::Kind::PermissionDenied
           || kind == rusty::io::Error::Kind::AddrNotAvailable
           || kind == rusty::io::Error::Kind::Other;
    assert(ok);
    printf("PASS (kind=%d)\n", static_cast<int>(kind));
}

void test_peer_addr_local_addr_on_stream() {
    printf("test_peer_addr_local_addr_on_stream: ");
    auto listener = TcpListener::bind(
        SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 0)).unwrap();
    auto bound = listener.local_addr().unwrap();

    std::thread connector([bound]() {
        auto stream = TcpStream::connect(bound).unwrap();
        // Just hold the connection open until the parent finishes.
        std::array<std::uint8_t, 1> dummy{};
        (void)stream.read(std::span<std::uint8_t>(dummy));
    });

    auto accept_result = listener.accept().unwrap();
    auto& conn = accept_result.first;
    auto local = conn.local_addr().unwrap();
    auto peer = conn.peer_addr().unwrap();
    assert(local.port() == bound.port());
    assert(peer.port() != 0);

    conn.shutdown(Shutdown::Both).unwrap();
    connector.join();
    printf("PASS\n");
}

void test_set_nonblocking_round_trip() {
    printf("test_set_nonblocking_round_trip: ");
    auto listener = TcpListener::bind(
        SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 0)).unwrap();
    auto r = listener.set_nonblocking(true);
    assert(r.is_ok());

    // accept should return WouldBlock immediately when no peer is
    // connecting and the listener is non-blocking.
    auto accept_result = listener.accept();
    assert(accept_result.is_err());
    auto err = accept_result.unwrap_err();
    assert(err.kind() == rusty::io::Error::Kind::WouldBlock);

    // Flip back to blocking — at this point we don't test the blocking
    // path (would hang the test).
    auto r2 = listener.set_nonblocking(false);
    assert(r2.is_ok());
    printf("PASS\n");
}

void test_empty_stream_returns_invalid_input() {
    printf("test_empty_stream_returns_invalid_input: ");
    TcpStream empty;
    assert(!empty.is_connected());

    std::array<std::uint8_t, 4> buf{};
    auto r = empty.read(std::span<std::uint8_t>(buf));
    assert(r.is_err());
    assert(r.unwrap_err().kind() == rusty::io::Error::Kind::InvalidInput);

    auto w = empty.write(std::span<const std::uint8_t>(buf));
    assert(w.is_err());

    auto s = empty.shutdown(Shutdown::Both);
    assert(s.is_err());
    printf("PASS\n");
}

int main() {
    printf("=== Testing rusty::net::{TcpListener, TcpStream, SocketAddrV4} ===\n");

    test_socket_addr_v4_from_str_roundtrip();
    test_socket_addr_v4_from_str_rejects_garbage();
    test_bind_local_addr_returns_assigned_port();
    test_connect_accept_exchange_bytes();
    test_connect_to_nonexistent_port_returns_error();
    test_peer_addr_local_addr_on_stream();
    test_set_nonblocking_round_trip();
    test_empty_stream_returns_invalid_input();

    printf("\nAll TCP tests passed!\n");
    return 0;
}
