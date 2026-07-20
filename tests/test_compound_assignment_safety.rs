use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn compound_assignment_rhs_is_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("compound_assignment_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
void f() {
    int x = 0;
    x += get_raw_int();
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
        "expected unsafe call in compound assignment RHS to fail. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "expected diagnostic to mention unsafe compound assignment RHS call. Output: {}",
        stdout
    );
}
