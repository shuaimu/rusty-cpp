use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_checker(source: &str) -> String {
    let dir = TempDir::new().expect("create temp dir");
    let file_path = dir.path().join("conditional_expression_safety.cpp");
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("include");

    fs::write(&file_path, source).expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&file_path)
        .arg("-I")
        .arg(include_dir)
        .output()
        .expect("run checker");

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ternary_expressions_are_checked_for_unsafe_calls() {
    let output = run_checker(
        r#"
#include <rusty/box.hpp>

// @unsafe
int get_raw_int();

// @safe
void f(bool cond) {
    int x = cond ? get_raw_int() : 0;
}
"#,
    );

    assert!(
        output.contains("get_raw_int"),
        "ternary unsafe call should be reported. Output: {}",
        output
    );
}

#[test]
fn logical_expressions_with_bool_literals_are_checked_for_unsafe_calls() {
    let output = run_checker(
        r#"
#include <rusty/box.hpp>

// @unsafe
int get_raw_int();

// @safe
void f() {
    if (get_raw_int() && true) {
    }
    if (false || get_raw_int()) {
    }
}
"#,
    );

    assert!(
        output.contains("get_raw_int"),
        "logical expression unsafe call should be reported. Output: {}",
        output
    );
}
