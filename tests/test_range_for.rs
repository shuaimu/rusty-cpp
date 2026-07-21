use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn range_for_expression_is_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("range_for.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(
        &file_path,
        r#"
#include <vector>
#include <rusty/box.hpp>

// @unsafe
std::vector<int>& get_vec_raw();

// @safe
void f() {
    for (int x : get_vec_raw()) {
        // do something
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
        "checker should reject unsafe calls in range-for expressions. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_vec_raw"),
        "range-for expression unsafe call should be reported. Output: {}",
        stdout
    );
}
