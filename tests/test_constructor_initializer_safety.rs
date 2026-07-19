use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn constructor_initializers_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("constructor_initializer_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

struct S {
    int x;

    // @safe
    S() : x(get_raw_int()) {}
};
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
        "checker should reject unsafe calls in constructor initializers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int") && stdout.contains("constructor initializer"),
        "constructor initializer unsafe call should be reported. Output: {}",
        stdout
    );
}
