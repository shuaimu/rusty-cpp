use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn constructor_initializers_are_checked_for_unsafe_calls() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("constructor_initializer_safety.cpp");

    fs::write(
        &file_path,
        r#"
// @unsafe
int get_raw_int();

struct S {
    int x;

    // @safe
    S() : x(get_raw_int()) {}
};
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
        "checker should reject unsafe calls in constructor initializers. Output: {}",
        stdout
    );
    assert!(
        stdout.contains("get_raw_int") && stdout.contains("constructor initializer"),
        "constructor initializer unsafe call should be reported. Output: {}",
        stdout
    );
}

// Regression: a struct with a default-initialized member triggers an implicit
// default constructor whose member initializers are compiler-synthesized and
// carry no source location. Extracting those must not panic the checker.
#[test]
fn implicit_constructor_synthesized_member_init_does_not_crash() {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("implicit_ctor.cpp");

    fs::write(
        &file_path,
        r#"
#include <string>

struct Wrapper {
    std::string value;
};

// @safe
void use_wrapper() {
    Wrapper w;   // implicit default constructor: synthesized member init for `value`
    (void)w;
}
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .output()
        .expect("run checker");

    // The checker must run to completion (exit 0/1), not crash (signal / 101).
    let code = output.status.code();
    assert!(
        matches!(code, Some(0) | Some(1)),
        "checker should not crash on an implicit constructor's synthesized member \
         initializers; got exit {:?}, stderr: {}",
        code,
        String::from_utf8_lossy(&output.stderr)
    );
}
