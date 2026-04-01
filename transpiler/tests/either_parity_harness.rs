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
    assert!(script.exists(), "missing harness script: {}", script.display());

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
