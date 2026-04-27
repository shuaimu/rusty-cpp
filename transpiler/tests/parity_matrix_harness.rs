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

fn cpp_module_interop_compile_script() -> PathBuf {
    repo_root().join("tests/transpile_tests/run_cpp_module_interop_compile.sh")
}

fn cpp_std_complex_compile_script() -> PathBuf {
    repo_root().join("tests/transpile_tests/run_cpp_std_complex_compile.sh")
}

fn ci_workflow_file() -> PathBuf {
    repo_root().join(".github/workflows/ci.yml")
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
        "either",
        "tap",
        "cfg-if",
        "take_mut",
        "arrayvec",
        "semver",
        "bitflags",
        "smallvec",
        "itertools",
        "once_cell",
        "serde_bytes",
        "serde_repr",
        "pollster",
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
fn test_cpp_module_interop_compile_script_dry_run_reports_expected_commands() {
    let script = cpp_module_interop_compile_script();
    assert!(
        script.exists(),
        "missing cpp-module interop compile script: {}",
        script.display()
    );

    let work_root = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--work-dir")
        .arg(work_root.path())
        .current_dir(repo_root())
        .output()
        .expect("failed to run cpp-module interop compile script");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("parity-test"), "stdout:\n{}", stdout);
    assert!(
        stdout.contains("--stop-after transpile"),
        "stdout:\n{}",
        stdout
    );
    assert!(stdout.contains("--cpp-module-index"), "stdout:\n{}", stdout);
    assert!(stdout.contains("custom.math.cppm"), "stdout:\n{}", stdout);
    assert!(
        stdout.contains("cpp_module_interop.cppm"),
        "stdout:\n{}",
        stdout
    );
}

#[test]
fn test_cpp_std_complex_compile_script_dry_run_reports_expected_commands() {
    let script = cpp_std_complex_compile_script();
    assert!(
        script.exists(),
        "missing cpp-std-complex compile script: {}",
        script.display()
    );

    let work_root = tempfile::tempdir().unwrap();
    let output = Command::new("bash")
        .arg(&script)
        .arg("--dry-run")
        .arg("--work-dir")
        .arg(work_root.path())
        .current_dir(repo_root())
        .output()
        .expect("failed to run cpp-std-complex compile script");

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("parity-test"), "stdout:\n{}", stdout);
    assert!(
        stdout.contains("--stop-after transpile"),
        "stdout:\n{}",
        stdout
    );
    assert!(stdout.contains("--cpp-module-index"), "stdout:\n{}", stdout);
    assert!(
        stdout.contains("cpp_std_complex.cppm"),
        "stdout:\n{}",
        stdout
    );
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

#[cfg(unix)]
#[test]
fn test_parity_matrix_failure_reports_first_failing_crate_and_artifact_paths() {
    let script = matrix_script();
    let work_root = tempfile::tempdir().unwrap();
    let shim_dir = tempfile::tempdir().unwrap();

    use std::fs;
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
        .arg("--crate")
        .arg("either")
        .arg("--work-root")
        .arg(work_root.path())
        .current_dir(repo_root())
        .env("PATH", shimmed_path)
        .output()
        .expect("failed to run parity matrix script");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_work_dir = work_root.path().join("either");
    assert!(
        stderr.contains("first failing crate: either"),
        "stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "baseline.txt: {}/baseline.txt",
            expected_work_dir.display()
        )),
        "stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "build.log: {}/build.log",
            expected_work_dir.display()
        )),
        "stderr:\n{}",
        stderr
    );
    assert!(
        stderr.contains(&format!("run.log: {}/run.log", expected_work_dir.display())),
        "stderr:\n{}",
        stderr
    );
}

#[test]
fn test_ci_workflow_defines_parity_matrix_job() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("parity-matrix:"));
    assert!(workflow.contains("./tests/transpile_tests/run_parity_matrix.sh"));
    assert!(workflow.contains("--work-root \"${RUNNER_TEMP}/rusty-parity-matrix\""));
}

#[test]
fn test_ci_workflow_defines_cpp_module_interop_compile_job() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("cpp-module-interop-compile:"));
    assert!(workflow.contains("./tests/transpile_tests/run_cpp_module_interop_compile.sh"));
    assert!(workflow.contains("--work-dir \"${RUNNER_TEMP}/rusty-cpp-module-interop\""));
}

#[test]
fn test_ci_workflow_defines_cpp_std_complex_compile_job() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("cpp-std-complex-compile:"));
    assert!(workflow.contains("./tests/transpile_tests/run_cpp_std_complex_compile.sh"));
    assert!(workflow.contains("--work-dir \"${RUNNER_TEMP}/rusty-cpp-std-complex\""));
}

#[test]
fn test_ci_workflow_uploads_per_crate_artifacts_on_failure() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("Upload parity matrix artifacts on failure"));
    assert!(workflow.contains("if: failure()"));
    assert!(workflow.contains("actions/upload-artifact@v4"));

    for crate_name in [
        "either",
        "tap",
        "cfg-if",
        "take_mut",
        "arrayvec",
        "semver",
        "bitflags",
        "smallvec",
        "itertools",
        "once_cell",
        "serde_bytes",
        "serde_repr",
        "pollster",
    ] {
        assert!(
            workflow.contains(&format!(
                "${{{{ runner.temp }}}}/rusty-parity-matrix/{}/**",
                crate_name
            )),
            "missing artifact upload path for crate '{}'",
            crate_name
        );
    }
}

#[test]
fn test_ci_workflow_uploads_cpp_module_interop_artifacts_on_failure() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("Upload cpp module interop artifacts on failure"));
    assert!(workflow.contains("if: failure()"));
    assert!(workflow.contains("actions/upload-artifact@v4"));
    assert!(workflow.contains("${{ runner.temp }}/rusty-cpp-module-interop/**"));
}

#[test]
fn test_ci_workflow_uploads_cpp_std_complex_artifacts_on_failure() {
    let workflow = std::fs::read_to_string(ci_workflow_file()).expect("read ci workflow");
    assert!(workflow.contains("Upload cpp std complex artifacts on failure"));
    assert!(workflow.contains("if: failure()"));
    assert!(workflow.contains("actions/upload-artifact@v4"));
    assert!(workflow.contains("${{ runner.temp }}/rusty-cpp-std-complex/**"));
}
