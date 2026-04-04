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
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The thin wrapper forwards to parity-test which uses Stage A/B/C/D/E labels
    assert!(stdout.contains("Parity Test: either"));
    assert!(stdout.contains("Stage A"));
    assert!(stdout.contains("Stage B"));
    assert!(stdout.contains("Stage C"));
    assert!(stdout.contains("Stage D"));
    assert!(stdout.contains("Stage E"));
    assert!(stdout.contains("[dry-run]"));
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
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage A"));
    assert!(stdout.contains("Stopped after transpile stage"));
    assert!(!stdout.contains("Stage D"));
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
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Stage D"));
    assert!(stdout.contains("Stopped after build stage"));
    assert!(!stdout.contains("Stage E"));
}

#[test]
fn test_either_parity_harness_rejects_unknown_flag() {
    let script = harness_script();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--definitely-invalid-flag")
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    // clap rejects unknown flags with non-zero exit
    assert!(!output.status.success());
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
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output_first.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output_first.stdout),
        String::from_utf8_lossy(&output_first.stderr)
    );
    let stdout_first = String::from_utf8_lossy(&output_first.stdout);
    assert!(stdout_first.contains("Stage A"));
    assert!(stdout_first.contains("Stopped after baseline stage"));

    let baseline_path = work_dir.path().join("baseline.txt");
    assert!(baseline_path.exists(), "baseline.txt should be created");

    // Run a second time — should succeed and overwrite
    let output_second = Command::new("bash")
        .arg(&script)
        .arg("--stop-after")
        .arg("baseline")
        .arg("--work-dir")
        .arg(work_dir.path())
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness second time");

    assert!(
        output_second.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output_second.stdout),
        String::from_utf8_lossy(&output_second.stderr)
    );
}

#[test]
fn test_either_parity_harness_stop_after_run_passes_as_control_crate() {
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();

    let output = Command::new("bash")
        .arg(&script)
        .arg("--stop-after")
        .arg("run")
        .arg("--work-dir")
        .arg(work_dir.path())
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parity Test: either"));
    assert!(stdout.contains("Stage E: Running transpiled tests..."));
    assert!(stdout.contains("Run: PASS"));

    assert!(work_dir.path().join("baseline.txt").exists());
    assert!(work_dir.path().join("build.log").exists());
    assert!(work_dir.path().join("run.log").exists());

    let run_log = std::fs::read_to_string(work_dir.path().join("run.log")).expect("read run.log");
    assert!(run_log.contains("Results:"));
}

#[cfg(unix)]
#[test]
fn test_either_parity_harness_reports_stage_failure() {
    // When cargo is unavailable, the harness should fail with an error
    let script = harness_script();
    let work_dir = tempfile::tempdir().unwrap();
    let shim_dir = tempfile::tempdir().unwrap();

    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

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
        .current_dir(repo_root())
        .output()
        .expect("failed to run parity harness");

    // Should fail since cargo shim exits 99
    assert!(!output.status.success());
}
