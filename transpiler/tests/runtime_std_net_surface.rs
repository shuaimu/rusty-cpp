use std::path::{Path, PathBuf};
use std::process::Command;

fn transpile(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.rs");
    let output = dir.path().join("output.cpp");
    std::fs::write(&input, source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::read_to_string(output).unwrap()
}

fn compile_run(source: &str) {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.cpp");
    let output = dir.path().join("test");
    std::fs::write(&input, source).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let include: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let result = Command::new(compiler)
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1"])
        .arg("-I")
        .arg(include)
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(output).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn std_net_aliases_and_arc_select_socket_addr_methods() {
    let source = r#"
use std::net::TcpListener as Listener;
use std::net::TcpStream;
use std::sync::Arc;
pub type Server = Listener;
pub fn local(listener: &std::net::TcpListener) -> Result<std::net::SocketAddr, std::io::Error> {
    let listener: &Server = listener;
    listener.local_addr()
}
pub fn accept(listener: &Arc<std::net::TcpListener>) -> Result<(TcpStream, std::net::SocketAddr), std::io::Error> {
    let listener: &Arc<Server> = listener;
    listener.accept()
}
pub fn peer(stream: &TcpStream) -> Result<std::net::SocketAddr, std::io::Error> {
    stream.peer_addr()
}
pub fn kind(error: &std::io::Error) -> std::io::ErrorKind { error.kind() }
"#;
    let cpp = transpile(source);
    assert!(cpp.contains("local_socket_addr()"), "{cpp}");
    assert!(cpp.contains("accept_socket_addr()"), "{cpp}");
    assert!(cpp.contains("peer_socket_addr()"), "{cpp}");
    assert!(cpp.contains("rusty::io::Error::Kind"), "{cpp}");
    let main = r#"
int main() {
    using namespace rusty::net;
    auto listener = TcpListener::bind(SocketAddrV4::new_(Ipv4Addr::new_(127, 0, 0, 1), 0)).unwrap();
    auto address = std::get<SocketAddr_V4>(local(listener).unwrap())._0;
    auto client = TcpStream::connect(address).unwrap();
    if (std::get<SocketAddr_V4>(peer(client).unwrap())._0 != address) return 1;
    auto shared = rusty::Arc<TcpListener>::make(std::move(listener));
    auto connection = accept(shared).unwrap();
    if (!std::holds_alternative<SocketAddr_V4>(std::get<1>(connection))) return 2;
    if (kind(rusty::io::Error(rusty::io::Error::Kind::WouldBlock, "blocked")) != rusty::io::Error::Kind::WouldBlock) return 3;
}
"#;
    compile_run(&format!("{cpp}\n{main}"));
}

#[test]
fn local_net_names_keep_their_own_methods() {
    let source = r#"
struct TcpListener;
impl TcpListener { fn local_addr(&self) -> i32 { 7 } fn accept(&self) -> i32 { 9 } }
type Server = TcpListener;
pub fn local(listener: &Server) -> i32 { listener.local_addr() + listener.accept() }
"#;
    let cpp = transpile(source);
    assert!(!cpp.contains("local_socket_addr()"), "{cpp}");
    assert!(!cpp.contains("accept_socket_addr()"), "{cpp}");
    compile_run(&format!(
        "{cpp}\nint main() {{ TcpListener listener; return local(listener) == 16 ? 0 : 1; }}\n"
    ));
}

#[test]
fn runtime_imports_and_shadowed_std_do_not_acquire_standard_result_types() {
    for source in [
        "use rusty::net::TcpListener; pub fn local(s: &TcpListener) { s.local_addr(); s.accept(); }",
        "mod std { pub mod net { pub struct TcpListener; } } use std::net::TcpListener; pub fn local(s: &TcpListener) { s.local_addr(); s.accept(); }",
        "extern crate alternate as std; use std::net::TcpListener; pub fn local(s: &TcpListener) { s.local_addr(); s.accept(); }",
    ] {
        let cpp = transpile(source);
        assert!(!cpp.contains("local_socket_addr()"), "{cpp}");
        assert!(!cpp.contains("accept_socket_addr()"), "{cpp}");
    }
}

#[test]
fn standard_tcp_addresses_and_fd_transfer_preserve_socket_ownership() {
    compile_run(
        r#"
#include <rusty/net/tcp.hpp>
#include <cassert>
#include <type_traits>
using namespace rusty::net;
static_assert(std::is_same_v<decltype(std::declval<TcpListener>().local_addr()), rusty::Result<SocketAddrV4, rusty::io::Error>>);
static_assert(std::is_same_v<decltype(std::declval<TcpListener>().local_socket_addr()), rusty::Result<SocketAddr, rusty::io::Error>>);
static_assert(std::is_same_v<decltype(std::declval<TcpListener>().accept_socket_addr()), rusty::Result<std::tuple<TcpStream, SocketAddr>, rusty::io::Error>>);
int main() {
    auto listener = TcpListener::bind(SocketAddrV4::new_(Ipv4Addr::new_(127,0,0,1),0)).unwrap();
    auto local = listener.local_socket_addr().unwrap();
    assert(std::holds_alternative<SocketAddr_V4>(local));
    auto address = std::get<SocketAddr_V4>(local)._0;
    assert(address.port() != 0);
    auto client = TcpStream::connect(address).unwrap();
    auto client_local = client.local_socket_addr().unwrap();
    auto accepted = listener.accept_socket_addr().unwrap();
    auto peer = std::get<SocketAddr_V4>(std::get<1>(accepted))._0;
    assert(peer == std::get<SocketAddr_V4>(client_local)._0);
    assert(std::get<SocketAddr_V4>(client.peer_socket_addr().unwrap())._0 == address);
    assert(std::get<SocketAddr_V4>(std::get<0>(accepted).local_socket_addr().unwrap())._0 == address);
    assert(listener.set_nonblocking(true).is_ok());
    assert(listener.accept_socket_addr().unwrap_err().kind() == rusty::io::Error::Kind::WouldBlock);
    int transferred = -1;
    {
        auto stream = std::move(std::get<0>(accepted));
        transferred = stream.as_raw_fd();
        assert(stream.into_raw_fd() == transferred);
        assert(::fcntl(transferred, F_GETFD) >= 0);
    }
    assert(::fcntl(transferred, F_GETFD) >= 0);
    {
        auto owner = TcpStream::from_raw_fd(transferred);
        assert(owner.as_raw_fd() == transferred);
    }
    assert(::fcntl(transferred, F_GETFD) == -1 && errno == EBADF);
    int listener_fd;
    {
        auto owner = TcpListener::from_raw_fd(listener.into_raw_fd());
        listener_fd = owner.as_raw_fd();
        assert(owner.into_raw_fd() == listener_fd);
    }
    assert(::fcntl(listener_fd, F_GETFD) >= 0);
    assert(::close(listener_fd) == 0);

    // Sockets adopted through FromRawFd keep IPv6 addresses too.
    int v6_fd = ::socket(AF_INET6, SOCK_STREAM, 0);
    assert(v6_fd >= 0);
    sockaddr_in6 v6{};
    v6.sin6_family = AF_INET6;
    v6.sin6_addr = in6addr_loopback;
    assert(::bind(v6_fd, reinterpret_cast<sockaddr*>(&v6), sizeof(v6)) == 0);
    assert(::listen(v6_fd, 1) == 0);
    socklen_t length = sizeof(v6);
    assert(::getsockname(v6_fd, reinterpret_cast<sockaddr*>(&v6), &length) == 0);
    auto v6_listener = TcpListener::from_raw_fd(v6_fd);
    auto v6_local = v6_listener.local_socket_addr().unwrap();
    assert(std::holds_alternative<SocketAddr_V6>(v6_local));
    assert(std::get<SocketAddr_V6>(v6_local)._0.port() == ntohs(v6.sin6_port));
    int v6_client_fd = ::socket(AF_INET6, SOCK_STREAM, 0);
    assert(v6_client_fd >= 0);
    assert(::connect(v6_client_fd, reinterpret_cast<sockaddr*>(&v6), sizeof(v6)) == 0);
    auto v6_client = TcpStream::from_raw_fd(v6_client_fd);
    auto v6_accepted = v6_listener.accept_socket_addr().unwrap();
    assert(std::holds_alternative<SocketAddr_V6>(std::get<1>(v6_accepted)));
    assert(std::holds_alternative<SocketAddr_V6>(v6_client.peer_socket_addr().unwrap()));
    assert(std::holds_alternative<SocketAddr_V6>(std::get<0>(v6_accepted).local_socket_addr().unwrap()));
}
"#,
    );
}

#[test]
fn socket_addr_v4_parse_and_display_match_rust_std() {
    let cpp = transpile(
        r#"
pub fn parse(text: &str) -> Result<std::net::SocketAddrV4, std::net::AddrParseError> {
    text.parse::<std::net::SocketAddrV4>()
}
pub fn display(address: &std::net::SocketAddrV4) -> String { address.to_string() }
"#,
    );
    let mut cases = vec![
        "".to_owned(),
        "127.0.0.1".into(),
        "127.0.0.1:".into(),
        "127.0.0.1:+1".into(),
        "127.0.0.1:-0".into(),
        "127.0.0.1: 1".into(),
        "127.0.0.1:1 ".into(),
        " 127.0.0.1:1".into(),
        "127.0.0.1:1\n".into(),
        "127.0.0.1:1\0junk".into(),
        "127.0.0.1\0junk:1".into(),
        "127.0.0.1:１".into(),
        "１２７.0.0.1:1".into(),
        "[::1]:1".into(),
        "127..0.1:1".into(),
        "127.0.1:1".into(),
        "127.0.0.1.1:1".into(),
        "127.0.0.1:1:1".into(),
        "0xff.0.0.1:1".into(),
    ];
    for octet in 0..=300 {
        cases.push(format!("{octet}.0.255.1:65535"));
        cases.push(format!("127.0.0.{octet}:0"));
    }
    for octet in [
        "00",
        "01",
        "001",
        "0255",
        "256",
        "99999999999999",
        "+1",
        "-0",
    ] {
        cases.push(format!("127.0.{octet}.1:7"));
    }
    for port in [
        "0",
        "1",
        "65535",
        "65536",
        "9999999999999999999",
        "0000000000000000000000000000000000001",
        "00065535",
        "00065536",
    ] {
        cases.push(format!("1.2.3.4:{port}"));
    }
    let mut main = String::from("\n#include <cassert>\nint main() {\n");
    for case in cases {
        let bytes = case
            .bytes()
            .map(|b| format!("static_cast<char>({b})"))
            .collect::<Vec<_>>()
            .join(",");
        main.push_str(&format!("{{ const char input[] = {{{bytes}{}}}; auto parsed = parse(std::string_view(input, {}));\n", if bytes.is_empty() { "0" } else { "" }, case.len()));
        match case.parse::<std::net::SocketAddrV4>() {
            Ok(address) => main.push_str(&format!(
                "assert(parsed.is_ok()); assert(display(parsed.unwrap()) == {:?});\n",
                address.to_string()
            )),
            Err(error) => main.push_str(&format!(
                "assert(parsed.is_err()); assert(parsed.unwrap_err().to_string() == {:?});\n",
                error.to_string()
            )),
        }
        main.push_str("}\n");
    }
    main.push_str("}\n");
    compile_run(&format!("{cpp}\n{main}"));
}
