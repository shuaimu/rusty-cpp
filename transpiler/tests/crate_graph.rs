//! `--crate-graph` end to end on a four-crate fixture
//! (`tests/fixtures/crate_graph`):
//!
//! - `graph-root` depends on `dep-core` with `default-features = false` and
//!   on `dep-unused`, which it never names;
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
//! widget and the backend, plus 3722 from `dep_core::scan`, which reuses a
//! `Copy` `Option<u64>` after passing it by value and compares copies of a
//! `derive(Hash, Copy)` newtype.

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
    assert_eq!(manifest["crates"][0]["module"], "dep_core");

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
    // A consumer crate's implementor reaches the trait through its generic adapter.
    assert!(
        core.contains("template <class U> using rusty_dyn_adapter = BackendDynAdapter<U>;"),
        "{core}"
    );

    let root = std::fs::read_to_string(out.join("graph_root.cppm")).unwrap();
    assert!(root.contains("export module graph_root;"), "{root}");
    assert!(root.contains("export import dep_core;"), "{root}");
    assert!(root.contains("rusty::Box<dep_core::Backend>"), "{root}");
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
    let flags = ["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-pthread"];
    let bmi_dir = directory.path().join("bmi");
    std::fs::create_dir_all(&bmi_dir).unwrap();
    let prebuilt = format!("-fprebuilt-module-path={}", bmi_dir.display());
    let mut objects = Vec::new();
    for (module, source) in [("dep_core", dep_core.clone()), ("graph_root", out.join("graph_root.cppm"))] {
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
    assert_eq!(String::from_utf8(ran.stdout).unwrap(), "3736\n");
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
