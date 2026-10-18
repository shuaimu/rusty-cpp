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

// ---------------------------------------------------------------------------
// --crate-graph-git: the same crates from a git repository

/// The fixture with `dep-base`, `dep-core` and `dep-spec` committed to a git
/// repository (`family`) and the root depending on `dep-core` from it, on a
/// branch whose name `cargo tree` prints form-urlencoded (`feat%2Fx%2By`);
/// `dep-unused` stays a path dependency. With `other`, `dep-base` also depends
/// on `dep-other` from a second repository and never names it. `edit_family`
/// may change the family's packages before they are committed. Cargo runs
/// with the fixture's own CARGO_HOME, so it reads the repositories through
/// `file://` URLs, needs no network and leaves `~/.cargo` alone.
struct GitFixture {
    directory: tempfile::TempDir,
    cargo_home: PathBuf,
    family: PathBuf,
    other: PathBuf,
}

impl GitFixture {
    fn root(&self) -> PathBuf {
        self.directory.path().join("root")
    }
}

fn git_commit(repository: &Path) {
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["add", "-A"],
        &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "fixture"],
        &["branch", "feat/x+y"],
    ] {
        let ran = Command::new("git")
            .current_dir(repository)
            .args(args)
            .output()
            .unwrap();
        assert_success(&ran, &format!("git {args:?}"));
    }
}

fn git_fixture(features_default: bool, other: bool, edit_family: fn(&Path)) -> GitFixture {
    let directory = tempfile::tempdir().unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/crate_graph");
    copy_tree(&fixtures.join("root"), &directory.path().join("root"));
    copy_tree(&fixtures.join("dep-unused"), &directory.path().join("dep-unused"));
    let family = directory.path().join("family");
    for package in ["dep-base", "dep-core", "dep-spec"] {
        copy_tree(&fixtures.join(package), &family.join(package));
    }
    let other_repository = directory.path().join("other");
    if other {
        std::fs::create_dir_all(other_repository.join("src")).unwrap();
        std::fs::write(
            other_repository.join("Cargo.toml"),
            "[package]\nname = \"dep-other\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(other_repository.join("src/lib.rs"), "pub fn other() -> u32 {\n    1\n}\n")
            .unwrap();
        git_commit(&other_repository);
        let manifest = family.join("dep-base/Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(
            &manifest,
            format!(
                "{text}\n[dependencies]\ndep-other = {{ git = \"file://{}\" }}\n",
                other_repository.display()
            ),
        )
        .unwrap();
    }
    edit_family(&family);
    git_commit(&family);
    let manifest = directory.path().join("root/Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    let path_dependency = "dep-core = { path = \"../dep-core\", default-features = false }";
    assert!(text.contains(path_dependency), "{text}");
    let git_dependency = format!(
        "dep-core = {{ git = \"file://{}\", branch = \"feat/x+y\"{} }}",
        family.display(),
        if features_default { "" } else { ", default-features = false" }
    );
    std::fs::write(&manifest, text.replace(path_dependency, &git_dependency)).unwrap();
    let cargo_home = directory.path().join("cargo-home");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    // Not --offline: Cargo has to fetch the file:// repositories into this
    // CARGO_HOME once; the transpiler then resolves --locked --offline.
    let locked = Command::new(cargo)
        .current_dir(directory.path().join("root"))
        .env("CARGO_HOME", &cargo_home)
        .arg("generate-lockfile")
        .output()
        .unwrap();
    assert_success(&locked, "cargo generate-lockfile with git dependencies");
    GitFixture {
        directory,
        cargo_home,
        family,
        other: other_repository,
    }
}

fn transpile_git(fixture: &GitFixture, out: &Path, extra: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .env("CARGO_HOME", &fixture.cargo_home)
        .arg("--crate")
        .arg(fixture.root().join("Cargo.toml"))
        .arg("--crate-graph")
        .arg("--verus-exec")
        .arg("--verus-erase-helper")
        .arg(helper())
        .args(extra)
        .args(["--locked", "--offline", "--output-dir"])
        .arg(out)
        .output()
        .unwrap()
}

/// Every file under `dir`, relative to it, with its bytes.
fn tree_files(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(base, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(base).unwrap().to_path_buf(),
                    std::fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    walk(dir, dir, &mut files);
    files
}

#[test]
fn crate_graph_git_repository_emits_exactly_what_path_dependencies_do() {
    // Both feature settings, so Cargo's per-crate features reach the git
    // crates exactly as they reach path crates. The two spellings of the
    // repository URL exercise Cargo's URL comparison.
    for (features_default, spelling) in [(false, "/"), (true, ".git")] {
        let path_fixture = fixture(features_default);
        let path_out = path_fixture.path().join("cpp");
        assert_success(
            &transpile(&path_fixture.path().join("root"), &path_out),
            "--crate-graph over path dependencies",
        );
        let git = git_fixture(features_default, false, |_| {});
        let git_out = git.directory.path().join("cpp");
        let url = format!("file://{}{spelling}", git.family.display());
        let transpiled = transpile_git(&git, &git_out, &["--crate-graph-git".to_string(), url]);
        assert_success(&transpiled, "--crate-graph --crate-graph-git");
        assert!(git_out.join("dep-core/dep_core.cppm").is_file());
        let path_files = tree_files(&path_out);
        let git_files = tree_files(&git_out);
        assert_eq!(
            path_files.keys().collect::<Vec<_>>(),
            git_files.keys().collect::<Vec<_>>(),
            "features_default = {features_default}"
        );
        for (file, bytes) in &path_files {
            assert!(
                &git_files[file] == bytes,
                "{} differs between the path and git graphs (features_default = {features_default})",
                file.display()
            );
        }
    }
}

#[test]
fn crate_graph_refuses_a_git_dependency_until_its_repository_is_named() {
    // Without --crate-graph-git, the root's git dependency dep-core would be
    // left out of the graph (only the path dependency dep-unused would join),
    // so --crate-graph refuses it before any output.
    let git = git_fixture(false, true, |_| {});
    let family = format!("file://{}", git.family.display());
    let other = format!("file://{}", git.other.display());
    let out = git.directory.path().join("cpp");
    let transpiled = transpile_git(&git, &out, &[]);
    assert_eq!(transpiled.status.code(), Some(1), "an unnamed git dependency was accepted");
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(
        stderr.contains(&format!("\n  graph-root -> dep-core v0.1.0 ({family})\n")),
        "{stderr}"
    );
    assert!(stderr.contains(&format!("graph crates: --crate-graph-git {family}\n")), "{stderr}");
    assert!(!stderr.contains("dep-other"), "dep-base is not a graph crate yet: {stderr}");
    assert!(!out.exists(), "output was written before the error");

    // Naming the family brings its three crates, and with them dep-base's
    // dependency on dep-other, from a second repository no one named.
    let out = git.directory.path().join("cpp-family");
    let transpiled = transpile_git(&git, &out, &["--crate-graph-git".to_string(), family.clone()]);
    assert_eq!(transpiled.status.code(), Some(1), "an unnamed git dependency was accepted");
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(stderr.contains(&format!("\n  dep-base -> dep-other v0.1.0 ({other})\n")), "{stderr}");
    assert!(stderr.contains(&format!("graph crates: --crate-graph-git {other}\n")), "{stderr}");
    assert!(!stderr.contains("dep-core v"), "{stderr}");
    assert!(!out.exists(), "output was written before the error");

    // Naming the other repository too makes dep-other a graph crate, one the
    // root never needs.
    let out = git.directory.path().join("cpp-both");
    let transpiled = transpile_git(
        &git,
        &out,
        &["--crate-graph-git".to_string(), family, "--crate-graph-git".to_string(), other],
    );
    assert_success(&transpiled, "--crate-graph-git naming both repositories");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("crate-graph.json")).unwrap()).unwrap();
    assert_eq!(manifest["crates"][0]["package"], "dep-base");
    assert_eq!(manifest["crates"][1]["package"], "dep-core");
    assert_eq!(manifest["ghost_only"], serde_json::json!(["dep-spec"]));
    assert_eq!(manifest["unused"], serde_json::json!(["dep-other", "dep-unused"]));
}

#[test]
fn crate_graph_git_rejects_a_repository_outside_the_graph_before_output() {
    let git = git_fixture(false, false, |_| {});
    let out = git.directory.path().join("cpp");
    let typo = format!("file://{}-typo", git.family.display());
    let transpiled = transpile_git(&git, &out, &["--crate-graph-git".to_string(), typo.clone()]);
    assert!(!transpiled.status.success(), "an unreached repository was accepted");
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(stderr.contains(&format!("--crate-graph-git {typo}: no package")), "{stderr}");
    assert!(
        stderr.contains(&format!("depend on: file://{}", git.family.display())),
        "{stderr}"
    );
    assert!(!out.exists(), "output was written before the error");

    // A Cargo source id or a pinned reference is not a repository URL.
    for value in [
        format!("git+file://{}", git.family.display()),
        format!("file://{}?branch=main", git.family.display()),
    ] {
        let transpiled = transpile_git(&git, &out, &["--crate-graph-git".to_string(), value]);
        assert!(!transpiled.status.success());
        assert!(!out.exists(), "output was written before the error");
    }

    // The option belongs to --crate-graph.
    let transpiled = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .env("CARGO_HOME", &git.cargo_home)
        .arg("--crate")
        .arg(git.root().join("Cargo.toml"))
        .arg("--crate-graph-git")
        .arg(format!("file://{}", git.family.display()))
        .arg("--output-dir")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(transpiled.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(stderr.contains("--crate-graph"), "{stderr}");
    assert!(!out.exists(), "output was written before the error");
}

/// A `cpp_abi` marker in a declared binary target of `dep-base`, which crate
/// mode rejects.
fn add_marked_binary(dep_base: &Path) {
    std::fs::create_dir_all(dep_base.join("src/bin")).unwrap();
    std::fs::write(
        dep_base.join("src/bin/tool.rs"),
        "#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]\n\
         pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {\n    bytes\n}\n\nfn main() {}\n",
    )
    .unwrap();
    let manifest = dep_base.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{text}\n[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\n"))
        .unwrap();
}

#[test]
fn crate_graph_git_member_source_contracts_are_preflighted_like_path_ones() {
    // The root has no source contract, so crate mode first scans declared
    // `path` dependencies for one; that scan cannot follow a git dependency.
    // A named repository must still get the exact-graph preflight a path
    // dependency gets.
    let expected = "contains source-owned C++ contracts; cross-crate adapter calls are unsupported";
    let path_fixture = fixture(false);
    add_marked_binary(&path_fixture.path().join("dep-base"));
    let transpiled = transpile(&path_fixture.path().join("root"), &path_fixture.path().join("cpp"));
    assert!(!transpiled.status.success(), "the path graph accepted the marker");
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(stderr.contains(expected), "{stderr}");

    let git = git_fixture(false, false, |family| add_marked_binary(&family.join("dep-base")));
    let out = git.directory.path().join("cpp");
    let family = format!("file://{}", git.family.display());
    let transpiled = transpile_git(&git, &out, &["--crate-graph-git".to_string(), family]);
    assert!(!transpiled.status.success(), "the git graph accepted the marker");
    let stderr = String::from_utf8_lossy(&transpiled.stderr);
    assert!(stderr.contains(expected), "{stderr}");
}
