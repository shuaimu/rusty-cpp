use std::path::{Path, PathBuf};
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

fn target_artifacts_root(work_dir: &Path) -> PathBuf {
    work_dir.join("targets")
}

fn target_artifact_dir(work_dir: &Path, module_name: &str) -> PathBuf {
    target_artifacts_root(work_dir).join(module_name)
}

fn expanded_artifact_path(work_dir: &Path, module_name: &str) -> PathBuf {
    target_artifact_dir(work_dir, module_name).join("expanded.rs")
}

fn cppm_artifact_path(work_dir: &Path, module_name: &str) -> PathBuf {
    target_artifact_dir(work_dir, module_name).join(format!("{}.cppm", module_name))
}

fn runner_cpp_path(work_dir: &Path) -> PathBuf {
    work_dir.join("runner.cpp")
}

fn runner_binary_path(work_dir: &Path) -> PathBuf {
    work_dir.join("runner")
}

fn build_log_path(work_dir: &Path) -> PathBuf {
    work_dir.join("build.log")
}

fn run_log_path(work_dir: &Path) -> PathBuf {
    work_dir.join("run.log")
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

/// Create a crate nested under a workspace root but not listed as a member.
fn create_workspace_mismatch_fixture(dir: &std::path::Path) -> PathBuf {
    let ws_root = dir.join("ws");
    let member_src = ws_root.join("member").join("src");
    let orphan_src = ws_root.join("orphan").join("src");
    std::fs::create_dir_all(&member_src).unwrap();
    std::fs::create_dir_all(&orphan_src).unwrap();

    std::fs::write(
        ws_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n",
    )
    .unwrap();

    std::fs::write(
        ws_root.join("member/Cargo.toml"),
        "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(member_src.join("lib.rs"), "pub fn member() {}\n").unwrap();

    std::fs::write(
        ws_root.join("orphan/Cargo.toml"),
        "[package]\nname = \"orphan\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        orphan_src.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[test]\nfn test_add() { assert_eq!(add(1, 2), 3); }\n",
    )
    .unwrap();

    ws_root.join("orphan/Cargo.toml")
}

/// Create a fixture where tests fail only because warnings are denied.
fn create_warning_as_error_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"warning_as_error_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "#![cfg_attr(test, deny(warnings))]\n\npub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nfn intentionally_unused_helper() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_add() {\n        assert_eq!(add(1, 2), 3);\n    }\n}\n",
    )
    .unwrap();

    dir.join("Cargo.toml")
}

/// Create a fixture with both lib unit tests and integration tests.
fn create_mixed_wrappers_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    let tests_dir = dir.join("tests");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&tests_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"mixed_wrappers\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn unit_add() { assert_eq!(add(1, 2), 3); }\n}\n",
    )
    .unwrap();
    std::fs::write(
        tests_dir.join("integ.rs"),
        "use mixed_wrappers::add;\n\n#[test]\nfn integ_add() { assert_eq!(add(2, 3), 5); }\n",
    )
    .unwrap();

    dir.join("Cargo.toml")
}

/// Create a fixture with only lib unit tests (`#[cfg(test)]` in lib target).
fn create_unit_only_wrappers_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"unit_only_wrappers\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn unit_add_only() { assert_eq!(add(1, 2), 3); }\n}\n",
    )
    .unwrap();

    dir.join("Cargo.toml")
}

/// Create a fixture with only integration tests (`tests/*.rs`).
fn create_integration_only_wrappers_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    let tests_dir = dir.join("tests");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&tests_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"integration_only_wrappers\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    std::fs::write(
        tests_dir.join("integ.rs"),
        "use integration_only_wrappers::add;\n\n#[test]\nfn integ_add_only() { assert_eq!(add(2, 3), 5); }\n",
    )
    .unwrap();

    dir.join("Cargo.toml")
}

/// Create a fixture with bin/test target names that collide after normalization.
fn create_module_name_collision_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    let tests_dir = dir.join("tests");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&tests_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"module_name_collision_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"cli-tool\"\npath = \"src/main.rs\"\n\n[[test]]\nname = \"cli_tool\"\npath = \"tests/cli_tool.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("main.rs"),
        "fn main() { let _ = module_name_collision_fixture::add(1, 2); }\n",
    )
    .unwrap();
    std::fs::write(
        tests_dir.join("cli_tool.rs"),
        "use module_name_collision_fixture::add;\n\n#[test]\nfn integ_collision_case() { assert_eq!(add(1, 2), 3); }\n",
    )
    .unwrap();

    dir.join("Cargo.toml")
}

/// Create a fixture that exercises std panic/cell/marker imports through parity build.
fn create_std_runtime_import_fixture(dir: &std::path::Path) -> PathBuf {
    let src_dir = dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"std_runtime_import_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "use std::cell::Cell;\nuse std::marker::PhantomData;\nuse std::panic;\n\npub fn run() -> i32 {\n    let flag = Cell::new(0i32);\n    let marker: PhantomData<&mut i32> = PhantomData;\n    let _ = marker;\n\n    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {\n        flag.set(7);\n        7i32\n    }));\n\n    match result {\n        Ok(value) => value,\n        Err(payload) => panic::resume_unwind(payload),\n    }\n}\n\n#[test]\nfn test_run() {\n    assert_eq!(run(), 7);\n}\n",
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

#[test]
fn test_parity_discovery_workspace_mismatch_fallback_passes() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_workspace_mismatch_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--no-baseline")
        .arg("--dry-run")
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Metadata retry:"));
    assert!(stdout.contains("Target: orphan (Lib)"));
}

#[test]
fn test_parity_discovery_disambiguates_normalized_module_name_collisions() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_module_name_collision_fixture(fixture_dir.path());

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
    assert!(stdout.contains("Target: cli-tool (Bin) → module cli_tool"));
    assert!(stdout.contains("Target: cli_tool (Test) → module cli_tool_test"));
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
fn test_stop_after_baseline_workspace_mismatch_fallback_passes() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_workspace_mismatch_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage A:"));
    assert!(stdout.contains("Baseline retry:"));
    assert!(work_dir.path().join("baseline.txt").exists());
}

#[test]
fn test_stop_after_baseline_workspace_mismatch_synthetic_fixture_passes() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_workspace_mismatch_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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
fn test_stop_after_baseline_warning_as_error_retry_passes() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_warning_as_error_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Baseline retry: detected warning-as-error lint failure."));
    assert!(work_dir.path().join("baseline.txt").exists());
}

#[test]
fn test_parity_test_malformed_manifest_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package\nname = \"broken\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn x() {}\n").unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(dir.path().join("Cargo.toml"))
        .arg("--stop-after")
        .arg("baseline")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse Cargo.toml"));
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
    assert!(expanded_artifact_path(work_dir.path(), "either").exists());
}

#[test]
fn test_multi_target_stop_after_expand_stops_before_transpile_and_build_outputs() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stopped after expand stage."));

    assert!(expanded_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(expanded_artifact_path(work_dir.path(), "integ").exists());
    assert!(!cppm_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(!cppm_artifact_path(work_dir.path(), "integ").exists());
    assert!(!runner_cpp_path(work_dir.path()).exists());
    assert!(!build_log_path(work_dir.path()).exists());
    assert!(!run_log_path(work_dir.path()).exists());
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
    let cppm_path = cppm_artifact_path(work_dir.path(), "either");
    assert!(cppm_path.exists());
    let cppm = std::fs::read_to_string(cppm_path).expect("failed to read transpiled cppm");
    assert!(cppm.contains("export void rusty_test_basic()"));
}

#[test]
fn test_stop_after_transpile_collects_wrappers_from_libtests_and_test_targets() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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

    let lib_cppm = std::fs::read_to_string(cppm_artifact_path(work_dir.path(), "mixed_wrappers"))
        .expect("failed to read transpiled lib target");
    assert!(lib_cppm.contains("rusty_test_tests_unit_add"));
    assert!(lib_cppm.contains("tests::unit_add();"));
    assert!(
        !lib_cppm.contains("Rust-only libtest marker without emitted function: tests::unit_add")
    );

    let integ_cppm = std::fs::read_to_string(cppm_artifact_path(work_dir.path(), "integ"))
        .expect("failed to read transpiled integration target");
    assert!(integ_cppm.contains("rusty_test_integ_add"));
}

#[test]
fn test_multi_target_stop_after_transpile_stops_before_build_and_run_outputs() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stopped after transpile stage."));

    assert!(expanded_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(expanded_artifact_path(work_dir.path(), "integ").exists());
    assert!(cppm_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(cppm_artifact_path(work_dir.path(), "integ").exists());
    assert!(!runner_cpp_path(work_dir.path()).exists());
    assert!(!build_log_path(work_dir.path()).exists());
    assert!(!run_log_path(work_dir.path()).exists());
}

#[test]
fn test_stop_after_transpile_collects_wrappers_for_unit_only_crate() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_unit_only_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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

    let lib_cppm =
        std::fs::read_to_string(cppm_artifact_path(work_dir.path(), "unit_only_wrappers"))
            .expect("failed to read transpiled lib target");
    assert!(lib_cppm.contains("rusty_test_tests_unit_add_only"));
    assert!(lib_cppm.contains("tests::unit_add_only();"));
}

#[test]
fn test_stop_after_transpile_collects_wrappers_for_integration_only_crate() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_integration_only_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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

    let lib_cppm = std::fs::read_to_string(cppm_artifact_path(
        work_dir.path(),
        "integration_only_wrappers",
    ))
    .expect("failed to read transpiled lib target");
    assert!(
        !lib_cppm.contains("rusty_test_"),
        "lib target should not contribute wrappers for integration-only fixture"
    );

    let integ_cppm = std::fs::read_to_string(cppm_artifact_path(work_dir.path(), "integ"))
        .expect("failed to read transpiled integration target");
    assert!(integ_cppm.contains("rusty_test_integ_add_only"));
}

#[test]
fn test_stop_after_build_succeeds_for_integration_only_crate() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_integration_only_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("build")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stopped after build stage."));

    assert!(runner_cpp_path(work_dir.path()).exists());
    assert!(runner_binary_path(work_dir.path()).exists());
    assert!(build_log_path(work_dir.path()).exists());
    assert!(!run_log_path(work_dir.path()).exists());

    let runner_cpp =
        std::fs::read_to_string(runner_cpp_path(work_dir.path())).expect("failed to read runner");
    assert!(runner_cpp.contains("rusty_test_integ_add_only();"));
}

#[test]
fn test_stop_after_transpile_rewrites_std_runtime_import_fixture() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_std_runtime_import_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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
    let cppm = std::fs::read_to_string(cppm_artifact_path(
        work_dir.path(),
        "std_runtime_import_fixture",
    ))
    .expect("failed to read transpiled std runtime fixture");
    assert!(cppm.contains("namespace panic = rusty::panic;"));
    assert!(cppm.contains("using rusty::Cell;"));
    assert!(cppm.contains("using rusty::PhantomData;"));
    assert!(!cppm.contains("using std::panic;"));
    assert!(!cppm.contains("using std::cell::Cell;"));
    assert!(!cppm.contains("using std::marker::PhantomData;"));
}

#[test]
fn test_stop_after_transpile_persists_unique_artifacts_for_normalized_collisions() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_module_name_collision_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
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
    assert!(cppm_artifact_path(work_dir.path(), "module_name_collision_fixture").exists());
    assert!(cppm_artifact_path(work_dir.path(), "cli_tool").exists());
    assert!(cppm_artifact_path(work_dir.path(), "cli_tool_test").exists());
    assert!(expanded_artifact_path(work_dir.path(), "module_name_collision_fixture").exists());
    assert!(expanded_artifact_path(work_dir.path(), "cli_tool").exists());
    assert!(expanded_artifact_path(work_dir.path(), "cli_tool_test").exists());
}

#[test]
fn test_stop_after_build_generates_runner_entries_from_discovered_wrappers() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("build")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Generated runner:"),
        "expected runner generation in stdout, got:\n{}",
        stdout
    );
    assert!(stdout.contains("Stopped after build stage."));
    assert!(runner_binary_path(work_dir.path()).exists());
    assert!(build_log_path(work_dir.path()).exists());
    assert!(!run_log_path(work_dir.path()).exists());

    let runner_cpp =
        std::fs::read_to_string(work_dir.path().join("runner.cpp")).expect("failed to read runner");
    let integ_pos = runner_cpp
        .find("rusty_test_integ_add();")
        .expect("runner should invoke integration wrapper");
    let unit_pos = runner_cpp
        .find("rusty_test_tests_unit_add();")
        .expect("runner should invoke unit-test wrapper");

    assert!(
        integ_pos < unit_pos,
        "wrapper invocation order should be deterministic by wrapper name"
    );
    assert!(!runner_cpp.contains("TEST_CASE(\""));
}

#[test]
fn test_multi_target_stop_after_run_executes_and_persists_run_log() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("run")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage E: Running transpiled tests..."));
    assert!(stdout.contains("Run: PASS"));

    assert!(expanded_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(expanded_artifact_path(work_dir.path(), "integ").exists());
    assert!(cppm_artifact_path(work_dir.path(), "mixed_wrappers").exists());
    assert!(cppm_artifact_path(work_dir.path(), "integ").exists());
    assert!(runner_cpp_path(work_dir.path()).exists());
    assert!(runner_binary_path(work_dir.path()).exists());
    assert!(build_log_path(work_dir.path()).exists());
    assert!(run_log_path(work_dir.path()).exists());

    let run_log = std::fs::read_to_string(run_log_path(work_dir.path())).expect("read run.log");
    assert!(run_log.contains("Results:"));
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

    assert!(work_dir.path().join("baseline.txt").exists());
    assert!(target_artifacts_root(work_dir.path()).exists());
    assert!(expanded_artifact_path(work_dir.path(), "either").exists());
    assert!(cppm_artifact_path(work_dir.path(), "either").exists());
}

#[test]
fn test_keep_work_dir_prunes_stale_target_dirs_between_reruns() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let first_output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("transpile")
        .arg("--work-dir")
        .arg(work_dir.path())
        .arg("--keep-work-dir")
        .output()
        .expect("failed to run");
    assert!(
        first_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );
    assert!(target_artifact_dir(work_dir.path(), "mixed_wrappers").exists());
    assert!(target_artifact_dir(work_dir.path(), "integ").exists());

    std::fs::remove_file(fixture_dir.path().join("tests/integ.rs")).unwrap();

    let second_output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("transpile")
        .arg("--work-dir")
        .arg(work_dir.path())
        .arg("--keep-work-dir")
        .output()
        .expect("failed to run");
    assert!(
        second_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );
    assert!(target_artifact_dir(work_dir.path(), "mixed_wrappers").exists());
    assert!(
        !target_artifact_dir(work_dir.path(), "integ").exists(),
        "stale integration-target directory should be pruned on rerun"
    );
}

#[test]
fn test_build_stage_ignores_stale_root_cppm_when_reusing_work_dir() {
    let fixture_dir = tempfile::tempdir().unwrap();
    let manifest = create_mixed_wrappers_fixture(fixture_dir.path());
    let work_dir = tempfile::tempdir().unwrap();

    let first_output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("transpile")
        .arg("--work-dir")
        .arg(work_dir.path())
        .arg("--keep-work-dir")
        .output()
        .expect("failed to run");
    assert!(
        first_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );

    std::fs::write(
        work_dir.path().join("stale.cppm"),
        "this is not valid c++\n",
    )
    .unwrap();

    let build_output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--stop-after")
        .arg("build")
        .arg("--work-dir")
        .arg(work_dir.path())
        .arg("--keep-work-dir")
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(
        stdout.contains("Generated runner:"),
        "expected runner generation, stdout:\n{}",
        stdout
    );

    let runner_cpp =
        std::fs::read_to_string(work_dir.path().join("runner.cpp")).expect("failed to read runner");
    assert!(
        !runner_cpp.contains("stale.cppm"),
        "runner should ignore stale root-level .cppm files, runner:\n{}",
        runner_cpp
    );
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
