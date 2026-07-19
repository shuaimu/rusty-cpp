use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn safe_function_call_arguments_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("function_call_arguments.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(
        &file_path,
        r#"
#include <rusty/box.hpp>

// @unsafe
int get_raw_int();

// @safe
void sink(int);

// @safe
void f() {
    sink(get_raw_int());
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .arg("-I")
        .arg(include_dir)
        .output()
        .expect("run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        !output.status.success(),
        "checker should reject unsafe calls in safe function arguments. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "unsafe argument call should be reported. Output: {}",
        stdout
    );
}
