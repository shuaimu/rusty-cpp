use std::path::PathBuf;
use std::process::Command;

fn transpiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("transpiler crate should be in workspace")
        .to_path_buf()
}

fn either_manifest() -> PathBuf {
    repo_root().join("tests/transpile_tests/either/Cargo.toml")
}

/// Create a minimal fixture crate for testing (not either).
fn create_fixture_crate(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fixture_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[test]\nfn test_add() { assert_eq!(add(1, 2), 3); }\n",
    )
    .unwrap();
    dir.join("Cargo.toml")
}

// ── CLI parse tests ────────────────────────────────────

#[test]
fn test_parity_test_no_args_uses_defaults() {
    // Without --manifest-path, defaults to Cargo.toml in current dir
    // In a temp dir with no Cargo.toml, should fail gracefully
    let dir = tempfile::tempdir().unwrap();
    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--dry-run")
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    // Should fail: no Cargo.toml in temp dir
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("Manifest"));
}

#[test]
fn test_parity_test_all_valid_stop_after_values() {
    for stage in &["baseline", "expand", "transpile", "build", "run"] {
        let output = transpiler_bin()
            .arg("parity-test")
            .arg("--manifest-path")
            .arg(either_manifest())
            .arg("--dry-run")
            .arg("--stop-after")
            .arg(stage)
            .output()
            .expect("failed to run");

        assert!(
            output.status.success(),
            "stage '{}' should be valid, stderr: {}",
            stage,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_parity_test_no_baseline_flag() {
    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(either_manifest())
        .arg("--dry-run")
        .arg("--no-baseline")
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Stage A should be skipped
    assert!(!stdout.contains("Stage A:"));
    assert!(stdout.contains("Stage B"));
}

// ── Discovery tests ────────────────────────────────────

#[test]
fn test_parity_discovers_either_lib_target() {
    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(either_manifest())
        .arg("--dry-run")
        .arg("--stop-after")
        .arg("expand")
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Target: either (Lib)"));
    assert!(stdout.contains("module either"));
    assert!(stdout.contains("cargo expand --lib --tests"));
}

#[test]
fn test_parity_discovers_fixture_crate() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = create_fixture_crate(dir.path());

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--dry-run")
        .arg("--stop-after")
        .arg("expand")
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parity Test: fixture_crate"));
    assert!(stdout.contains("Target: fixture_crate (Lib)"));
}

// ── Stop-after integration tests ───────────────────────

#[test]
fn test_stop_after_baseline_creates_baseline_log() {
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(either_manifest())
        .arg("--stop-after")
        .arg("baseline")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(work_dir.path().join("baseline.txt").exists());
}

#[test]
fn test_stop_after_expand_creates_expanded_source() {
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(either_manifest())
        .arg("--stop-after")
        .arg("expand")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(work_dir.path().join("expanded_either.rs").exists());
}

#[test]
fn test_stop_after_transpile_creates_cppm() {
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(either_manifest())
        .arg("--stop-after")
        .arg("transpile")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cppm_path = work_dir.path().join("either.cppm");
    assert!(cppm_path.exists());
    let cppm = std::fs::read_to_string(cppm_path).expect("failed to read transpiled cppm");
    assert!(cppm.contains("export void rusty_test_basic()"));
}

// ── Rerun determinism ──────────────────────────────────

#[test]
fn test_rerun_same_workdir_does_not_append_stale_artifacts() {
    let work_dir = tempfile::tempdir().unwrap();

    // Run twice with same work dir
    for _ in 0..2 {
        let output = transpiler_bin()
            .arg("parity-test")
            .arg("--manifest-path")
            .arg(either_manifest())
            .arg("--stop-after")
            .arg("transpile")
            .arg("--work-dir")
            .arg(work_dir.path())
            .output()
            .expect("failed to run");

        assert!(output.status.success());
    }

    // Should have exactly one baseline.txt, one expanded_either.rs, one either.cppm
    let files: Vec<String> = std::fs::read_dir(work_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let baseline_count = files.iter().filter(|f| f.starts_with("baseline")).count();
    let expanded_count = files.iter().filter(|f| f.starts_with("expanded_")).count();
    let cppm_count = files.iter().filter(|f| f.ends_with(".cppm")).count();

    assert_eq!(
        baseline_count, 1,
        "expected 1 baseline file, got {}",
        baseline_count
    );
    assert_eq!(
        expanded_count, 1,
        "expected 1 expanded file, got {}",
        expanded_count
    );
    assert_eq!(cppm_count, 1, "expected 1 cppm file, got {}", cppm_count);
}

// ── Non-either fixture crate ───────────────────────────

#[test]
fn test_dry_run_on_non_either_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = create_fixture_crate(dir.path());

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--dry-run")
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parity Test: fixture_crate"));
    assert!(stdout.contains("[dry-run]"));
    assert!(!stdout.contains("either"));
}
