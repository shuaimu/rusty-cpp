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
