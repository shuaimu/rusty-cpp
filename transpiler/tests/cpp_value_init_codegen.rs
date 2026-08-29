//! Compile-and-run proof for `#[cfg_attr(any(), cpp_value_init)]`.
//!
//! The string assertions pin the narrow generated spelling.  The C++ checks
//! prove that this spelling preserves the downstream ABI traits it exists for:
//! aggregate and positional initialization, layout, trivial copying, and
//! scalar zeroing under plain default construction.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["clang++", "clang++-22", "clang++-21"] {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

#[test]
fn cpp_value_init_generated_scalars_preserve_aggregate_layout_and_defaults() {
    let Some(compiler) = find_clang() else {
        eprintln!("skipping cpp_value_init compile test: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust_path = temp.path().join("message.rs");
    let cpp_path = temp.path().join("message.cpp");
    let bin_path = temp.path().join("message.bin");
    std::fs::write(
        &rust_path,
        r#"
pub struct Message {
    #[cfg_attr(any(), cpp_value_init)]
    pub term: u64,
    #[cfg_attr(any(), cpp_value_init)]
    pub voter: u16,
    #[cfg_attr(any(), cpp_value_init)]
    pub acknowledged: bool,
    pub untouched: i32,
}
"#,
    )
    .expect("write Rust source");

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let rust_metadata_path = temp.path().join("libmessage.rmeta");
    let rust_compile = Command::new(rustc)
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg("-Dwarnings")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(&rust_metadata_path)
        .arg(&rust_path)
        .output()
        .expect("invoke rustc");
    assert!(
        rust_compile.status.success(),
        "cpp_value_init fixture is not warning-clean Rust\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rust_compile.stdout),
        String::from_utf8_lossy(&rust_compile.stderr)
    );

    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust_path)
        .arg("-o")
        .arg(&cpp_path)
        .output()
        .expect("invoke transpiler");
    assert!(
        transpile.status.success(),
        "transpile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );

    let mut cpp = std::fs::read_to_string(&cpp_path).expect("read generated C++");
    assert!(cpp.contains("uint64_t term{};"), "{cpp}");
    assert!(cpp.contains("uint16_t voter{};"), "{cpp}");
    assert!(cpp.contains("bool acknowledged{};"), "{cpp}");
    assert!(cpp.contains("int32_t untouched;"), "{cpp}");
    assert!(!cpp.contains("int32_t untouched{};"), "{cpp}");
    cpp.push_str(
        r#"
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <type_traits>

struct LegacyMessage {
    uint64_t term{};
    uint16_t voter{};
    bool acknowledged{};
    int32_t untouched;
};

static_assert(std::is_aggregate_v<Message>);
static_assert(std::is_standard_layout_v<Message>);
static_assert(std::is_trivially_copyable_v<Message>);
static_assert(!std::is_trivially_default_constructible_v<Message>);
static_assert(sizeof(Message) == sizeof(LegacyMessage));
static_assert(alignof(Message) == alignof(LegacyMessage));
static_assert(offsetof(Message, term) == offsetof(LegacyMessage, term));
static_assert(offsetof(Message, voter) == offsetof(LegacyMessage, voter));
static_assert(offsetof(Message, acknowledged) == offsetof(LegacyMessage, acknowledged));
static_assert(offsetof(Message, untouched) == offsetof(LegacyMessage, untouched));

int main() {
    Message plain;
    assert(plain.term == 0);
    assert(plain.voter == 0);
    assert(!plain.acknowledged);

    Message value{};
    assert(value.term == 0);
    assert(value.voter == 0);
    assert(!value.acknowledged);
    assert(value.untouched == 0);

    Message positional{11, 12, true, 13};
    assert(positional.term == 11);
    assert(positional.voter == 12);
    assert(positional.acknowledged);
    assert(positional.untouched == 13);
    return 0;
}
"#,
    );
    std::fs::write(&cpp_path, cpp).expect("append C++ assertions");

    let compile = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(project_include_dir())
        .arg(&cpp_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke clang++");
    assert!(
        compile.status.success(),
        "generated cpp_value_init fixture did not compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin_path)
        .output()
        .expect("run compiled fixture");
    assert!(
        run.status.success(),
        "generated cpp_value_init fixture failed at runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
