use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn throw_operands_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("throw_expression_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    throw get_raw_int();
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
        "expected unsafe call in throw operand to fail. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "expected diagnostic to mention unsafe throw operand call. Output: {}",
        stdout
    );
}
