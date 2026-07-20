use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn try_and_catch_bodies_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("try_catch_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
void unsafe_in_try();

// @unsafe
void unsafe_in_catch();

// @safe
void f() {
    try {
        unsafe_in_try();
    } catch (...) {
        unsafe_in_catch();
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
        "checker should reject unsafe calls inside try/catch bodies. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("unsafe_in_try") && stdout.contains("unsafe_in_catch"),
        "try and catch unsafe calls should both be reported. Output: {}",
        stdout
    );
}
