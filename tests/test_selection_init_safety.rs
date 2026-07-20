use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn selection_init_statements_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("selection_init_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int unsafe_if_init();

// @unsafe
int unsafe_switch_init();

// @safe
void f() {
    if (int x = unsafe_if_init(); x > 0) {}

    switch (int y = unsafe_switch_init(); y) {
        case 0:
            break;
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
        "expected unsafe calls in selection init statements to fail. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("unsafe_if_init"),
        "expected diagnostic to mention unsafe if init call. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("unsafe_switch_init"),
        "expected diagnostic to mention unsafe switch init call. Output: {}",
        stdout
    );
}
