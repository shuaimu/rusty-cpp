use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn array_subscripts_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("array_subscript_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

// @safe
int f() {
    int arr[4] = {1, 2, 3, 4};
    return arr[get_raw_int()];
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
        "checker should reject unsafe calls inside array subscript expressions. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int"),
        "array subscript unsafe call should be reported. Output: {}",
        stdout
    );
}
