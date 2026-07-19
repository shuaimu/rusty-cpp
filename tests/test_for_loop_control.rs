use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn for_loop_initializer_is_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("for_loop_control.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(
        &file_path,
        r#"
#include <rusty/box.hpp>

// @unsafe
int get_raw_int();

// @safe
void f() {
    for (int i = get_raw_int(); i < 30; ++i) {
    }
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
        "checker should reject unsafe calls in for-loop initializers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "for-loop initializer unsafe call should be reported. Output: {}",
        stdout
    );
}
