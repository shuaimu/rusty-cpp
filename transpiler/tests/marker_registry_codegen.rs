//! End-to-end compile proof for the opt-in non-inheriting marker registry.
//!
//! String tests pin the intended spelling; this test also asks clang to
//! instantiate the positive and negative membership paths, the frozen KIND,
//! the !Send/!Sync bridge base, and its noexcept method contract.

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
fn generated_marker_registry_compiles_and_rejects_outsiders() {
    let Some(compiler) = find_clang() else {
        eprintln!("skipping marker registry compile test: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust_path = temp.path().join("registry.rs");
    let cpp_path = temp.path().join("registry.cpp");
    std::fs::write(
        &rust_path,
        r#"
pub mod rrr {
    #[cfg_attr(any(), cpp_marker_trait)]
    pub trait PayloadMember<Set> { const KIND: i32; }

    #[cfg_attr(any(), cpp_no_auto_traits)]
    pub struct Serializable<const KIND: i32> {}

    pub struct SerializableEnvelope<PayloadSet> {
        pub kind_: i32,
        pub inner_: usize,
        pub _payload_set: [core::marker::PhantomData<PayloadSet>; 0],
    }

    impl<PayloadSet> Default for SerializableEnvelope<PayloadSet> {
        #[cfg_attr(any(), cpp_ctor)]
        fn default() -> SerializableEnvelope<PayloadSet> {
            SerializableEnvelope { kind_: 0i32, inner_: 0usize, _payload_set: [] }
        }
    }

    impl<PayloadSet> SerializableEnvelope<PayloadSet> {
        pub fn empty() -> SerializableEnvelope<PayloadSet> { Default::default() }
    }

    impl<PayloadSet> Clone for SerializableEnvelope<PayloadSet> {
        fn clone(&self) -> SerializableEnvelope<PayloadSet> {
            let mut result: SerializableEnvelope<PayloadSet> = Default::default();
            result.kind_ = self.kind_;
            result.inner_ = self.inner_.clone();
            result
        }
    }

    impl<const KIND: i32> Serializable<KIND> {
        #[cfg_attr(any(), cpp_noexcept)]
        pub const fn static_kind() -> i32 {
            assert!(KIND != 0, "wire kind 0 is reserved");
            KIND
        }

        #[cfg_attr(any(), cpp_noexcept)]
        pub const fn kind(&self) -> i32 { Self::static_kind() }
    }

    pub fn pack<Set, T: PayloadMember<Set>>(_value: &T) -> i32 {
        <T as PayloadMember<Set>>::KIND
    }
}

pub mod janus {
    pub enum MakoCommandKind { Unknown = 0, LogEntry = 1 }
    pub struct MakoCommands {}
    pub struct LogEntry {}
    pub struct Outsider {}
}

#[cfg_attr(any(), cpp_marker_impl)]
impl rrr::PayloadMember<janus::MakoCommands> for janus::LogEntry {
    const KIND: i32 = janus::MakoCommandKind::LogEntry as i32;
}
"#,
    )
    .expect("write Rust source");

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let rust_metadata_path = temp.path().join("libregistry.rmeta");
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
        "marker-registry fixture is not valid warning-clean Rust\nstdout:\n{}\nstderr:\n{}",
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
    cpp.push_str(
        r#"
template<typename T>
concept Registered = requires(const T& value) {
    rrr::pack<janus::MakoCommands, T>(value);
};

static_assert(rrr::PayloadMember<janus::MakoCommands, janus::LogEntry>::value);
static_assert(rrr::PayloadMember<janus::MakoCommands, janus::LogEntry>::KIND == 1);
static_assert(!rrr::PayloadMember<janus::MakoCommands, janus::Outsider>::value);
static_assert(Registered<janus::LogEntry>);
static_assert(!Registered<janus::Outsider>);
static_assert(!rusty::is_send<rrr::Serializable<1>>::value);
static_assert(!rusty::is_sync<rrr::Serializable<1>>::value);
static_assert(noexcept(rrr::Serializable<1>::static_kind()));
static_assert(noexcept(std::declval<const rrr::Serializable<1>&>().kind()));
static_assert(rrr::Serializable<1>::static_kind() == 1);
struct LegacyEnvelopeLayout {
    int32_t kind_;
    size_t inner_;
};
static_assert(sizeof(rrr::SerializableEnvelope<janus::MakoCommands>) ==
              sizeof(LegacyEnvelopeLayout));
static_assert(alignof(rrr::SerializableEnvelope<janus::MakoCommands>) ==
              alignof(LegacyEnvelopeLayout));
static_assert(offsetof(rrr::SerializableEnvelope<janus::MakoCommands>, kind_) ==
              offsetof(LegacyEnvelopeLayout, kind_));
static_assert(offsetof(rrr::SerializableEnvelope<janus::MakoCommands>, inner_) ==
              offsetof(LegacyEnvelopeLayout, inner_));
static_assert(std::is_default_constructible_v<
              rrr::SerializableEnvelope<janus::MakoCommands>>);
static_assert(std::is_copy_constructible_v<
              rrr::SerializableEnvelope<janus::MakoCommands>>);
inline bool envelope_default_smoke() {
    auto original = rrr::SerializableEnvelope<janus::MakoCommands>::empty();
    original.kind_ = 7;
    auto cloned = original.clone();
    return cloned.kind_ == 7;
}
"#,
    );
    std::fs::write(&cpp_path, cpp).expect("append compile assertions");

    let compile = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(project_include_dir())
        .arg("-fsyntax-only")
        .arg(&cpp_path)
        .output()
        .expect("invoke clang++");
    assert!(
        compile.status.success(),
        "generated marker registry did not compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
}
