use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("transpiler crate should be in workspace")
        .to_path_buf()
}

fn matrix_script() -> PathBuf {
    repo_root().join("tests/transpile_tests/run_parity_matrix.sh")
}

#[test]
fn test_parity_matrix_dry_run_lists_all_crates_and_run_stage() {
    let script = matrix_script();
    assert!(
        script.exists(),
        "missing parity matrix script: {}",
        script.display()
    );

    let work_root = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--work-root")
        .arg(work_root.path())
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity matrix script");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for crate_name in [
        "either", "tap", "cfg-if", "take_mut", "arrayvec", "semver", "bitflags",
    ] {
        assert!(
            stdout.contains(&format!("crate: {}", crate_name)),
            "missing dry-run entry for crate '{}'\nstdout:\n{}",
            crate_name,
            stdout
        );
    }
    assert!(stdout.contains("parity-test"));
    assert!(stdout.contains("--stop-after run"));
}

#[test]
fn test_parity_matrix_unknown_crate_filter_fails() {
    let script = matrix_script();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--crate")
        .arg("definitely-not-a-matrix-crate")
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity matrix script");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown matrix crate"));
}

#[test]
fn test_parity_matrix_single_crate_run_passes_for_either_control() {
    let script = matrix_script();
    let work_root = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--crate")
        .arg("either")
        .arg("--work-root")
        .arg(work_root.path())
        .arg("--keep-work-dirs")
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity matrix script");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS: either"), "stdout:\n{}", stdout);
    assert!(work_root.path().join("either/baseline.txt").exists());
    assert!(work_root.path().join("either/build.log").exists());
    assert!(work_root.path().join("either/run.log").exists());
}
