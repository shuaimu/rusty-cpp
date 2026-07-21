use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn brace_initializers_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("brace_init_safety.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(
        &file_path,
        r#"
#include <rusty/box.hpp>

// @unsafe
int get_raw_int();

// @safe
int f() {
    int x{get_raw_int()};
    int y = int{get_raw_int()};
    return x + y;
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
        "checker should reject unsafe calls nested inside brace initializers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "brace initializer unsafe call should be reported. Output: {}",
        stdout
    );
}
