use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("transpiler crate should be in workspace")
        .to_path_buf()
}

fn harness_script() -> PathBuf {
    repo_root().join("tests/transpile_tests/either/run_parity_harness.sh")
}

#[test]
fn test_either_parity_harness_dry_run_lists_all_stages() {
    let script = harness_script();
    assert!(
        script.exists(),
        "missing harness script: {}",
        script.display()
    );

    let work_dir = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage 1/4: Rust baseline"));
    assert!(stdout.contains("Stage 2/4: Transpile expanded either crate"));
    assert!(stdout.contains("Stage 3/4: Build transpiled C++ module"));
    assert!(stdout.contains("Stage 4/4: Link and run C++ smoke executable"));
    assert!(stdout.contains("cargo test --manifest-path"));
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("cargo run -p rusty-cpp-transpiler -- --crate"));
    assert!(stdout.contains("g++ -std=c++23 -fmodules-ts"));
    assert!(stdout.contains("either_smoke_main.cpp"));
}

#[test]
fn test_either_parity_harness_dry_run_stop_after_transpile() {
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--stop-after")
        .arg("transpile")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage 1/4: Rust baseline"));
    assert!(stdout.contains("Stage 2/4: Transpile expanded either crate"));
    assert!(stdout.contains("Stopped after stage: transpile"));
    assert!(!stdout.contains("Stage 3/4: Build transpiled C++ module"));
}

#[test]
fn test_either_parity_harness_dry_run_stop_after_build() {
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--stop-after")
        .arg("build")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage 1/4: Rust baseline"));
    assert!(stdout.contains("Stage 2/4: Transpile expanded either crate"));
    assert!(stdout.contains("Stage 3/4: Build transpiled C++ module"));
    assert!(stdout.contains("Stopped after stage: build"));
    assert!(!stdout.contains("Stage 4/4: Link and run C++ smoke executable"));
}

#[test]
fn test_either_parity_harness_rejects_unknown_flag() {
    let script = harness_script();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--definitely-invalid-flag")
        .output()
        .expect("failed to run parity harness");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument"));
}

#[test]
fn test_either_parity_harness_baseline_stage_is_rerunnable() {
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();

    let output_first = Command::new("bash")
        .arg(&script)
        .arg("--stop-after")
        .arg("baseline")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output_first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output_first.stdout),
        String::from_utf8_lossy(&output_first.stderr)
    );
    let stdout_first = String::from_utf8_lossy(&output_first.stdout);
    assert!(stdout_first.contains("Stage 1/4: Rust baseline"));
    assert!(stdout_first.contains("Stopped after stage: baseline"));
    assert!(!stdout_first.contains("Stage 2/4: Transpile expanded either crate"));

    let rust_log_path = work_dir.path().join("rust_cargo_test.log");
    let rust_log_first = fs::read_to_string(&rust_log_path).expect("missing rust baseline log");
    assert!(rust_log_first.contains(">>> cargo test --manifest-path"));

    let output_second = Command::new("bash")
        .arg(&script)
        .arg("--stop-after")
        .arg("baseline")
        .arg("--work-dir")
        .arg(work_dir.path())
        .output()
        .expect("failed to run parity harness second time");

    assert!(
        output_second.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output_second.stdout),
        String::from_utf8_lossy(&output_second.stderr)
    );

    let rust_log_second =
        fs::read_to_string(&rust_log_path).expect("missing rust baseline log after rerun");
    let baseline_cmd_count = rust_log_second
        .matches(">>> cargo test --manifest-path")
        .count();
    assert_eq!(
        baseline_cmd_count, 1,
        "expected a fresh log for reruns, got {} baseline command entries",
        baseline_cmd_count
    );
}

#[cfg(unix)]
#[test]
fn test_either_parity_harness_reports_stage_failure() {
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();
    let shim_dir = tempfile::tempdir().unwrap();
    let cargo_shim = shim_dir.path().join("cargo");

    fs::write(&cargo_shim, "#!/usr/bin/env bash\nexit 99\n").expect("failed to write cargo shim");
    let mut perms = fs::metadata(&cargo_shim).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&cargo_shim, perms).unwrap();

    let current_path = std::env::var("PATH").unwrap_or_default();
    let shimmed_path = format!("{}:{}", shim_dir.path().display(), current_path);

    let output = Command::new("bash")
        .arg(&script)
        .arg("--stop-after")
        .arg("baseline")
        .arg("--work-dir")
        .arg(work_dir.path())
        .env("PATH", shimmed_path)
        .output()
        .expect("failed to run parity harness");

    assert!(
        !output.status.success(),
        "expected harness failure, stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rust_log_path = work_dir.path().join("rust_cargo_test.log");
    let rust_log = fs::read_to_string(rust_log_path).expect("missing rust baseline log");
    assert!(rust_log.contains(">>> cargo test --manifest-path"));
}
