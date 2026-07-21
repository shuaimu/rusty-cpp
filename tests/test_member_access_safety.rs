use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn member_access_receivers_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("member_access_safety.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(
        &file_path,
        r#"
#include <rusty/box.hpp>

struct S {
    int x;
};

// @unsafe
S get_raw_s();

// @safe
void f() {
    int x = get_raw_s().x;
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
        "checker should reject unsafe calls used as member access receivers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_s"),
        "member access receiver unsafe call should be reported. Output: {}",
        stdout
    );
}
