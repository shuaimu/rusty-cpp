use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn new_expressions_are_checked_for_nested_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("new_expression_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
int* f() {
    return new int(get_raw_int());
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
        "checker should reject unsafe calls nested inside new expressions. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "new expression unsafe call should be reported. Output: {}",
        stdout
    );
}
