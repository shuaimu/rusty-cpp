use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn lambda_capture_initializers_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("lambda_capture_initializer_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    auto local = [x = get_raw_int()]() {
        return x;
    };
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
        "checker should reject unsafe calls in lambda capture initializers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int") && stdout.contains("lambda capture initializer"),
        "lambda capture initializer unsafe call should be reported. Output: {}",
        stdout
    );
}
