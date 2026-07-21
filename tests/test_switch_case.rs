use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn switch_case_bodies_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("switch_case.cpp");

    fs::write(
        &file_path,
        r#"
void dangerous_case() {}
void dangerous_default() {}

// @safe
void test_switch(int mode) {
    switch (mode) {
        case 1:
            dangerous_case();
            break;
        default:
            dangerous_default();
            break;
    }
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .output()
        .expect("run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "checker should reject unsafe calls inside switch cases. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("dangerous_case"),
        "case body unsafe call should be reported. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("dangerous_default"),
        "default body unsafe call should be reported. Output: {}",
        stdout
    );
}

// Regression: stacked case labels (`case 1: case 2: foo();`) nest the inner
// label inside the outer CaseStmt in clang's AST. The shared body must still
// be scanned rather than dropped.
#[test]
fn stacked_case_labels_share_body_and_are_checked() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("stacked_case.cpp");

    fs::write(
        &file_path,
        r#"
void dangerous_shared() {}

// @safe
void test_stacked(int mode) {
    switch (mode) {
        case 1:
        case 2:
            dangerous_shared();
            break;
        default:
            break;
    }
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .output()
        .expect("run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "checker should reject unsafe calls under stacked case labels. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("dangerous_shared"),
        "unsafe call under stacked case labels should be reported. Output: {}",
        stdout
    );
}
