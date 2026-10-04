//! Lion parity (plan T6): lion-slab and lion-timer-wheel, transpiled with
//! `--crate-graph --verus-exec` from the snapshot in `tests/fixtures/lion`,
//! compiled as C++ modules, run, and compared with rustc.
//!
//! The cases are `tests/fixtures/lion/parity/src/lib.rs`: inserts, removes,
//! advances and fires over the two crates' public API, each folded into one
//! number. The Rust side is `cargo run --example run` in that crate, measured
//! by `tests/fixtures/lion/regen.sh` and recorded in `MANIFEST.toml` together
//! with a digest of every input it depends on; this test checks the digest,
//! so an edited case or crate cannot pass against a stale measurement, and it
//! never compiles vstd itself.
//!
//! Why not `parity-test`: that command transpiles `cargo expand` output, in
//! which rustc has already expanded `verus!`, so `--verus-exec` (stage 1)
//! never runs, stage 2 never sees the residue, and Lion's `vstd` dependency is
//! selected for transpilation. Measured on lion-slab at this revision, stage B
//! cannot expand vstd (its lockfile needs updating under `--locked`), stage C
//! emits `export import vstd;`, and stage D stops at "module 'vstd' not
//! found". This test therefore drives the path SRPC uses: crate mode with the
//! dependency graph, then the same compiler flags and runtime module cache as
//! `parity-test`'s stage D.
//!
//! Needs clang++ (or `$CXX`) with C++23 modules, CMake and Ninja for the
//! shared `.rusty-modules-cache` (built once, as `parity-test` builds it), and
//! the Verus git checkout that Lion's `vstd` dependency names in the local
//! Cargo cache for `--locked --offline` resolution (the verus-erase helper's
//! own git dependency on the same revision already requires it).

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

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lion")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            if entry.file_name() != "target" {
                copy_tree(&entry.path(), &target);
            }
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// sha256 over every file of `parity/`, `tree/lion-slab` and
/// `tree/lion-timer-wheel` (path, NUL, bytes, NUL, in path order); regen.sh
/// computes the same.
fn parity_inputs_digest(root: &Path) -> String {
    use sha2::{Digest, Sha256};
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                if entry.file_name() != "target" {
                    walk(root, &entry.path(), out);
                }
            } else {
                out.push(entry.path().strip_prefix(root).unwrap().to_string_lossy().into_owned());
            }
        }
    }
    let mut files = Vec::new();
    for top in ["parity", "tree/lion-slab", "tree/lion-timer-wheel"] {
        walk(root, &root.join(top), &mut files);
    }
    files.sort();
    let mut digest = Sha256::new();
    for file in files {
        digest.update(file.as_bytes());
        digest.update([0]);
        digest.update(std::fs::read(root.join(&file)).unwrap());
        digest.update([0]);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The runtime's module cache (`<repo>/.rusty-modules-cache`), built and
/// checked exactly as `parity-test` does (`ensure_rusty_modules_pcm_dir` in
/// main.rs: same directory, lock, freshness stamp and CMake target), so the
/// parity matrix and this test share one cache. Returns `(pcm dir, build dir)`.
fn rusty_module_cache() -> (PathBuf, PathBuf) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().unwrap();
    let cache = repo.join(".rusty-modules-cache");
    let (pcm, build) = (cache.join("pcm"), cache.join("build"));
    let fresh = || -> bool {
        if !pcm.join("rusty.pcm").exists() {
            return false;
        }
        let Ok(stamp) = std::fs::metadata(cache.join("freshness.stamp")).and_then(|m| m.modified()) else {
            return false;
        };
        let mut newest = std::fs::metadata(repo.join("CMakeLists.txt")).and_then(|m| m.modified()).ok();
        let mut stack = vec![repo.join("include"), repo.join("transpiled")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if matches!(path.extension().and_then(|e| e.to_str()), Some("hpp" | "h" | "cppm"))
                    && let Ok(modified) = entry.metadata().and_then(|m| m.modified())
                {
                    newest = Some(newest.map_or(modified, |n| n.max(modified)));
                }
            }
        }
        newest.is_none_or(|newest| stamp >= newest)
    };
    if fresh() {
        return (pcm, build);
    }
    std::fs::create_dir_all(&build).unwrap();
    std::fs::create_dir_all(&pcm).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(cache.join(".build.lock"))
        .unwrap();
    // SAFETY: flock on a file descriptor this function owns; released when
    // `lock` is dropped.
    assert_eq!(unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&lock), libc::LOCK_EX) }, 0);
    if fresh() {
        return (pcm, build);
    }
    if !build.join("CMakeCache.txt").is_file() {
        let configured = Command::new("cmake")
            .args(["-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_CXX_COMPILER=clang++"])
            .arg(&repo)
            .current_dir(&build)
            .output()
            .unwrap();
        assert_success(&configured, "configuring the rusty module cache");
    }
    let built = Command::new("cmake")
        .args(["--build", ".", "--target", "rusty", "--parallel", "3"])
        .current_dir(&build)
        .output()
        .unwrap();
    assert_success(&built, "building the rusty module cache");
    let mut stack = vec![build.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pcm") {
                let link = pcm.join(path.file_name().unwrap());
                let _ = std::fs::remove_file(&link);
                std::os::unix::fs::symlink(&path, &link).unwrap();
            }
        }
    }
    std::fs::write(cache.join("freshness.stamp"), b"").unwrap();
    drop(lock);
    (pcm, build)
}

const MAIN: &str = r#"#include <cstdio>
import lion_parity;
int main() {
    std::printf("slab_insert_get %llu\n", (unsigned long long)slab_insert_get());
    std::printf("slab_remove %llu\n", (unsigned long long)slab_remove());
    std::printf("slab_overwrite_and_get_mut %llu\n", (unsigned long long)slab_overwrite_and_get_mut());
    std::printf("slab_values %llu\n", (unsigned long long)slab_values());
    std::printf("wheel_insert_levels %llu\n", (unsigned long long)wheel_insert_levels());
    std::printf("wheel_remove %llu\n", (unsigned long long)wheel_remove());
    std::printf("wheel_fire_order %llu\n", (unsigned long long)wheel_fire_order());
    std::printf("wheel_advance_cascade %llu\n", (unsigned long long)wheel_advance_cascade());
    std::printf("wheel_reschedule %llu\n", (unsigned long long)wheel_reschedule());
    return 0;
}
"#;

#[test]
fn lion_slab_and_timer_wheel_cases_match_rustc() {
    let root = fixture_root();
    let manifest: toml::Value = toml::from_str(&std::fs::read_to_string(root.join("MANIFEST.toml")).unwrap()).unwrap();
    let parity = &manifest["parity"];
    assert_eq!(
        parity["inputs_sha256"].as_str(),
        Some(parity_inputs_digest(&root).as_str()),
        "parity/ or the two Lion crates changed since regen.sh measured the Rust output; rerun it"
    );
    let expected = parity["output"].as_str().unwrap().trim_start_matches('\n');

    // A scratch copy, so nothing is written next to the fixture.
    let work = tempfile::tempdir().unwrap();
    for dir in ["parity", "tree/lion-slab", "tree/lion-timer-wheel"] {
        copy_tree(&root.join(dir), &work.path().join(dir));
    }
    let out = work.path().join("cpp");
    let transpiled = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(work.path().join("parity/Cargo.toml"))
        .arg("--crate-graph")
        .arg("--verus-exec")
        .arg("--verus-erase-helper")
        .arg(helper())
        .args(["--locked", "--offline", "--output-dir"])
        .arg(&out)
        .output()
        .unwrap();
    assert_success(&transpiled, "--crate-graph --verus-exec transpilation");

    let graph: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("crate-graph.json")).unwrap()).unwrap();
    let mut modules = graph["crates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|krate| (krate["module"].as_str().unwrap().to_string(), out.join(krate["cppm"].as_str().unwrap())))
        .collect::<Vec<_>>();
    assert_eq!(
        modules.iter().map(|(module, _)| module.as_str()).collect::<Vec<_>>(),
        ["lion_slab", "lion_timer_wheel"]
    );
    modules.push(("lion_parity".to_string(), out.join("lion_parity.cppm")));
    for slots in [
        out.join("rusty_hand_slots.md"),
        out.join("lion-slab/rusty_hand_slots.md"),
        out.join("lion-timer-wheel/rusty_hand_slots.md"),
    ] {
        let text = std::fs::read_to_string(&slots).unwrap();
        assert!(text.contains("\n0 slot(s) requiring"), "{}:\n{text}", slots.display());
    }
    for (module, path) in &modules {
        let cpp = std::fs::read_to_string(path).unwrap();
        for residue in ["vstd", "Ghost<", "RustyVerusGhost", "TODO", "View"] {
            assert!(!cpp.contains(residue), "{module}: `{residue}` in the C++");
        }
    }

    let (rusty_pcm, rusty_build) = rusty_module_cache();
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let bmi = work.path().join("bmi");
    std::fs::create_dir_all(&bmi).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let flags = |command: &mut Command| {
        command
            .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-march=native", "-Werror=return-stack-address"])
            .arg(format!("-fprebuilt-module-path={}", rusty_pcm.display()))
            .arg("-I")
            .arg(&include)
            .arg(format!("-fprebuilt-module-path={}", bmi.display()));
    };
    let mut objects = Vec::new();
    for (module, source) in &modules {
        let pcm = bmi.join(format!("{module}.pcm"));
        let mut precompile = Command::new(&compiler);
        flags(&mut precompile);
        let precompiled = precompile
            .args(["-x", "c++-module", "--precompile"])
            .arg(source)
            .arg("-o")
            .arg(&pcm)
            .output()
            .unwrap();
        assert_success(&precompiled, &format!("precompiling {module}"));
        let object = bmi.join(format!("{module}.o"));
        let mut compile = Command::new(&compiler);
        flags(&mut compile);
        let compiled = compile.arg("-c").arg(source).arg("-o").arg(&object).output().unwrap();
        assert_success(&compiled, &format!("compiling {module}"));
        objects.push(object);
    }
    let main = work.path().join("main.cpp");
    std::fs::write(&main, MAIN).unwrap();
    let main_object = bmi.join("main.o");
    let mut compile_main = Command::new(&compiler);
    flags(&mut compile_main);
    let compiled = compile_main.arg("-c").arg(&main).arg("-o").arg(&main_object).output().unwrap();
    assert_success(&compiled, "compiling the importer");
    let binary = work.path().join("lion_parity");
    let linked = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-o")
        .arg(&binary)
        .args(&objects)
        .arg(&main_object)
        .arg(format!("-L{}", rusty_build.display()))
        .args([
            "-lrusty",
            "-lrusty_async",
            "-lvec_port",
            "-lbtree_port",
            "-lrc_port",
            "-larc_port",
            "-lbinary_heap_port",
            "-lstd_port",
            "-lstd_port_hashbrown",
            "-lvec_deque_port",
            "-llinked_list_port",
            "-lcell_port",
            "-lstring_port",
            "-pthread",
        ])
        .output()
        .unwrap();
    assert_success(&linked, "linking");
    let ran = Command::new(&binary).output().unwrap();
    assert_success(&ran, "running the C++");
    assert_eq!(String::from_utf8(ran.stdout).unwrap(), expected);
}
