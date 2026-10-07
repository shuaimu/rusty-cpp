//! `--crate-graph` end to end on a four-crate fixture
//! (`tests/fixtures/crate_graph`):
//!
//! - `graph-root` depends on `dep-core` with `default-features = false` and
//!   on `dep-unused`, which it never names;
//! - `dep-core` depends on `dep-base`, whose top-level `reactor` module and
//!   `Reactor` type recur nested in `dep-core`'s executor-shaped `sched`
//!   module (see its doc comment for the name-resolution shapes);
//! - `dep-core` gates `mod extra;` and one of two `bonus` definitions on its
//!   default `extra` feature, keeps its executable code in a `verus!` block with a ghost
//!   field, has a ghost-only `spec` module (a spec fn and a glob re-export of
//!   `dep-spec`), and declares the dyn trait `Backend`;
//! - `dep-spec` holds nothing but spec items;
//! - the root implements `dep_core::Backend` and hands a
//!   `Box<dyn dep_core::Backend>` back to `dep-core`.
//!
//! The emitted C++ is compiled and run, and must agree with the Rust value of
//! `graph_root::run()` for the selected features: 4 + 0 + 5 * 2 from the
//! widget and the backend, 3722 from `dep_core::scan`, which reuses a
//! `Copy` `Option<u64>` after passing it by value and compares copies of a
//! `derive(Hash, Copy)` newtype, 230 from `dep_core::sched_total`
//! (rustc on `sched.rs` with `dep-base`: 230), and 1500 + 7 from std's
//! `Duration` in the root beside `dep_core::time::Duration` (rustc on
//! `time.rs` and the root's `micros`: 1507), and 3068 from
//! `dep_core::park_total`, lion-executor's Runtime and Executor shapes over
//! dep-base (rustc on `park.rs` with `dep-base`: 3068).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn helper() -> PathBuf {
    static HELPER: OnceLock<PathBuf> = OnceLock::new();
    HELPER
        .get_or_init(|| {
            if let Some(path) = std::env::var_os("RUSTY_CPP_VERUS_ERASE").filter(|v| !v.is_empty()) {
                return PathBuf::from(path);
            }
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
            let target_dir = workspace.join("target").join("verus-erase-test-helper");
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = Command::new(cargo)
                .current_dir(&workspace)
                .args([
                    "build",
                    "--release",
                    "--locked",
                    "-p",
                    "verus-erase",
                    "--bin",
                    "rusty-cpp-verus-erase",
                    "--target-dir",
                ])
                .arg(&target_dir)
                .status()
                .expect("cargo runs");
            assert!(status.success(), "building the verus-erase helper failed");
            target_dir
                .join("release")
                .join(format!("rusty-cpp-verus-erase{}", std::env::consts::EXE_SUFFIX))
        })
        .clone()
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fresh copy of the fixture with a lockfile (no registry dependencies, so
/// `--offline` resolution always succeeds).
fn fixture(features_default: bool) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crate_graph"),
        directory.path(),
    );
    let root = directory.path().join("root");
    if features_default {
        let manifest = root.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(&manifest, text.replace(", default-features = false", "")).unwrap();
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let locked = Command::new(cargo)
        .current_dir(&root)
        .args(["generate-lockfile", "--offline"])
        .output()
        .unwrap();
    assert_success(&locked, "cargo generate-lockfile");
    directory
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn transpile(root: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--crate-graph")
        .arg("--verus-exec")
        .arg("--verus-erase-helper")
        .arg(helper())
        .args(["--locked", "--offline", "--output-dir"])
        .arg(out)
        .output()
        .unwrap()
}

#[test]
fn crate_graph_emits_one_module_per_needed_crate_and_runs() {
    let directory = fixture(false);
    let out = directory.path().join("cpp");
    let transpiled = transpile(&directory.path().join("root"), &out);
    assert_success(&transpiled, "--crate-graph transpilation");

    // One module per needed dependency; nothing for the ghost-only crate or
    // the crate the root never names.
    let dep_core = out.join("dep-core/dep_core.cppm");
    assert!(dep_core.is_file());
    assert!(!out.join("dep-spec").exists(), "a ghost-only crate emitted output");
    assert!(!out.join("dep-unused").exists(), "an unused crate emitted output");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("crate-graph.json")).unwrap()).unwrap();
    assert_eq!(manifest["ghost_only"], serde_json::json!(["dep-spec"]));
    assert_eq!(manifest["unused"], serde_json::json!(["dep-unused"]));
    assert_eq!(manifest["crates"][0]["module"], "dep_base");
    assert_eq!(manifest["crates"][1]["module"], "dep_core");
    let dep_base = out.join("dep-base/dep_base.cppm");
    // dep-core's manifest carries its portable type aliases, under the
    // declaring path and the crate-root re-export.
    let core_manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("dep-core/ufcs-traits.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(core_manifest["type_aliases"]["backend::RawFd"], "i32", "{core_manifest}");
    assert_eq!(core_manifest["type_aliases"]["RawFd"], "i32", "{core_manifest}");

    let core = std::fs::read_to_string(&dep_core).unwrap();
    assert!(core.contains("export module dep_core;"), "{core}");
    assert!(core.contains("namespace dep_core {"), "{core}");
    // The feature is off exactly as in the Rust lane: no `extra` module, the
    // `not(feature)` bonus.
    assert!(!core.contains("namespace extra") && !core.contains("BONUS"), "{core}");
    assert!(core.contains("return static_cast<uint32_t>(0);"), "{core}");
    // Ghost state lowered, ghost-only module and its re-exports gone.
    assert!(core.contains("[[no_unique_address]] rusty::Ghost log;"), "{core}");
    assert!(!core.contains("namespace spec") && !core.contains(" inv("), "{core}");
    assert!(!core.contains("TODO") && !core.contains("dep_spec"), "{core}");
    // derive(Hash) hashes the fields, at global scope, fully qualified.
    assert!(
        core.contains("struct std::hash<::dep_core::Id> {")
            && core.contains("return rusty::detail::hash_fields(v._0);"),
        "{core}"
    );
    // Executor-shaped name resolution (sched.rs): a relative re-export out of
    // the child `reactor` stays the crate's, not dep-base's `reactor`; an
    // imported item that shares a name with an enum variant stays an item;
    // the glob-only nested `log` is a namespace alias only; the same-named
    // child module and function it re-exports stay apart.
    assert!(core.contains("using reactor::IDLE_MS;"), "{core}");
    assert!(!core.contains("dep_base::reactor::IDLE_MS"), "{core}");
    assert!(!core.contains("constexpr auto Reactor ="), "{core}");
    assert!(!core.contains("constexpr auto Task ="), "{core}");
    assert!(core.contains("export namespace log = ") && !core.contains("namespace log {}"), "{core}");
    assert!(core.contains("using channel_tests::channel;"), "{core}");
    assert!(core.contains("using Output = typename rusty::detail::assoc_Output<F>::type;"), "{core}");
    assert!(core.contains("rusty::pin_place::map_unchecked_mut((*this),"), "{core}");
    // A consumer crate's implementor reaches the trait through its generic
    // owning forwarder (book §3.2.10).
    assert!(
        core.contains("template <class U> using rusty_dyn_adapter = BackendAdapter<U>;"),
        "{core}"
    );
    // park.rs. A dependency's re-exported `dep_base::Reactor` stays dep-base's,
    // not the crate's same-tail `sched::types::reactor::Reactor`.
    assert!(
        core.contains("uint64_t take(std::tuple<dep_base::Reactor, rusty::Box<uint64_t>> pair) {"),
        "{core}"
    );
    assert!(!core.contains("sched::types::reactor::Reactor pair"), "{core}");
    // The crate's own `park(Option<Ticks>)` says nothing about the argument
    // of dep-base's `Reactor::park` (no `-> ..::Ticks` closure annotation).
    assert!(
        core.contains("this->inner.park(timeout.map([&](auto&& t) { return t.into_base(); }));"),
        "{core}"
    );
    // A merged `impl Executor` names `Ticks` through ITS module's import.
    assert!(
        core.contains("rusty::Option<::dep_core::park::rt::ticks::Ticks>(Ticks::from_raw("),
        "{core}"
    );
    // A dependency's generic enum: its template variant struct is deduced in
    // the visit lambda, and an owned scrutinee's move-only payload moves.
    assert!(
        core.contains("[&]<typename... __Vs>(const dep_base::Outcome_Done<__Vs...>& _v)"),
        "{core}"
    );
    assert!(core.contains("return take(std::move(pair));"), "{core}");

    let root = std::fs::read_to_string(out.join("graph_root.cppm")).unwrap();
    assert!(root.contains("export module graph_root;"), "{root}");
    assert!(root.contains("export import dep_core;"), "{root}");
    assert!(root.contains("rusty::Box<dep_core::Backend>"), "{root}");
    // A bare `Duration::` bound to std by `use std::time::Duration;` stays
    // std's, and the global module fragment's time prelude is not
    // requalified to the dependency's `time::Duration`.
    assert!(root.contains("inline const Duration Duration::ZERO{"), "{root}");
    assert!(root.contains("return static_cast<uint32_t>(Duration::from_micros("), "{root}");
    assert!(!root.contains("dep_core::time::Duration::"), "{root}");
    // EpollLike: `close(fd: i32)` and the trait's `close(fd: RawFd)` are one
    // member (dep-core's manifest carries `RawFd = i32`); the trait's
    // forwarding body adds nothing. `label`'s trait body differs, so it stays
    // as the tagged member that dep-core's generic adapter probes first.
    assert_eq!(root.matches("uint32_t close(int32_t fd);").count(), 1, "{root}");
    assert!(!root.contains("close(dep_core::backend::RawFd"), "{root}");
    assert!(root.contains("uint32_t rusty_FdBackend_label() const;"), "{root}");
    // Book §3.2.10: the dependency's generic forwarder slot tries the trait's
    // CPO first (tag-ADL at the consumer's instantiation point); a consumer
    // implementor that emitted only members is reached through the tagged
    // `rusty_FdBackend_label` member it kept beside its inherent `label`.
    assert!(
        core.contains(
            "uint32_t label() const override { if constexpr (requires { FdBackend_::label(value_); }) { return FdBackend_::label(value_); } else if constexpr (requires { value_.rusty_FdBackend_label(); }) { return value_.rusty_FdBackend_label(); } else { return value_.label(); } }"
        ),
        "{core}"
    );
    for slots in [out.join("rusty_hand_slots.md"), out.join("dep-core/rusty_hand_slots.md")] {
        let text = std::fs::read_to_string(&slots).unwrap();
        assert!(text.contains("\n0 slot(s) requiring"), "{text}");
    }

    let clang = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let main = directory.path().join("main.cpp");
    std::fs::write(
        &main,
        "#include <cstdio>\nimport graph_root;\nint main() { std::printf(\"%u\\n\", run()); return 0; }\n",
    )
    .unwrap();
    let flags = [
        "-std=c++23",
        "-DRUSTY_PORTABLE_INTRINSICS=1",
        "-pthread",
        "-Werror=return-stack-address",
    ];
    let bmi_dir = directory.path().join("bmi");
    std::fs::create_dir_all(&bmi_dir).unwrap();
    let prebuilt = format!("-fprebuilt-module-path={}", bmi_dir.display());
    let mut objects = Vec::new();
    for (module, source) in [
        ("dep_base", dep_base.clone()),
        ("dep_core", dep_core.clone()),
        ("graph_root", out.join("graph_root.cppm")),
    ] {
        let pcm = bmi_dir.join(format!("{module}.pcm"));
        let precompiled = Command::new(&clang)
            .args(flags)
            .arg("-I")
            .arg(&include)
            .arg(&prebuilt)
            .args(["-x", "c++-module", "--precompile"])
            .arg(&source)
            .arg("-o")
            .arg(&pcm)
            .output()
            .unwrap();
        assert_success(&precompiled, &format!("precompiling {module}"));
        let object = bmi_dir.join(format!("{module}.o"));
        let compiled = Command::new(&clang)
            .args(flags)
            .arg(&prebuilt)
            .arg("-c")
            .arg(&pcm)
            .arg("-o")
            .arg(&object)
            .output()
            .unwrap();
        assert_success(&compiled, &format!("compiling {module}"));
        objects.push(object);
    }
    let binary = directory.path().join("graph");
    let linked = Command::new(&clang)
        .args(flags)
        .arg("-I")
        .arg(&include)
        .arg(&prebuilt)
        .arg(&main)
        .args(&objects)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert_success(&linked, "compiling and linking the importer");
    let ran = Command::new(&binary).output().unwrap();
    assert_success(&ran, "running the graph");
    assert_eq!(String::from_utf8(ran.stdout).unwrap(), "10658\n");
}

#[test]
fn crate_graph_evaluates_default_features_like_cargo() {
    // With dep-core's default `extra` feature on, `mod extra;` and the
    // feature's `bonus` are part of the crate, and the `not(feature)` one is not.
    let directory = fixture(true);
    let out = directory.path().join("cpp");
    let transpiled = transpile(&directory.path().join("root"), &out);
    assert_success(&transpiled, "--crate-graph transpilation with default features");
    let core = std::fs::read_to_string(out.join("dep-core/dep_core.cppm")).unwrap();
    assert!(core.contains("namespace extra") && core.contains("BONUS"), "{core}");
    assert!(core.contains("extra::BONUS"), "{core}");
    assert!(!core.contains("return static_cast<uint32_t>(0);"), "{core}");
}

/// A root that has a C++ ABI adapter, is itself a Verus crate and depends on
/// Verus crates (SRPC's shape once it depends on lion-reactor): the adapter
/// crate's opaque-surface audit runs on its ERASED source (no `verus!`, no
/// `use vstd::prelude::*`), and only on that crate — the dependencies' `verus!`
/// blocks and glob imports are not audited.
const ADAPTER_ROOT: &str = r#"//! A C++ ABI adapter crate that is also a Verus crate.
use vstd::prelude::*;

verus! {

pub fn doubled(x: u32) -> (r: u32)
    requires
        x < 1000,
    ensures
        r == 2 * x,
{
    x * 2
}

} // verus!

#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

pub fn run() -> u32 {
    doubled(dep_core::total(&dep_core::Widget::new(3)))
}
"#;

#[test]
fn crate_graph_audits_an_adapter_root_after_verus_erasure_and_per_crate() {
    let directory = fixture(false);
    let root = directory.path().join("root");
    std::fs::write(root.join("src/lib.rs"), ADAPTER_ROOT).unwrap();
    let out = directory.path().join("cpp");
    let transpiled = transpile(&root, &out);
    assert_success(&transpiled, "--crate-graph transpilation of an adapter root");
    let emitted = std::fs::read_to_string(out.join("graph_root.cppm")).unwrap();
    assert!(emitted.contains("export std::string adapted(std::string bytes);"), "{emitted}");
    assert!(emitted.contains("export uint32_t doubled(uint32_t x);"), "{emitted}");
    assert!(!emitted.contains("vstd") && !emitted.contains("verus"), "{emitted}");
    assert!(out.join("dep-core/dep_core.cppm").is_file());

    // Without --verus-exec the same root still reads `verus!` and the vstd
    // glob, and the adapter audit rejects them before any output.
    let raw_out = directory.path().join("cpp-raw");
    let raw = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .args(["--locked", "--offline", "--output-dir"])
        .arg(&raw_out)
        .output()
        .unwrap();
    assert!(!raw.status.success(), "the unerased adapter root was accepted");
    let stderr = String::from_utf8_lossy(&raw.stderr);
    assert!(stderr.contains("vstd :: prelude :: *"), "{stderr}");
    assert!(!raw_out.join("graph_root.cppm").exists());
}
