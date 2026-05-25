//! Regression tests for move-and-return patterns.
//!
//! Bug: the IR's `extract_return_source` Move arm returned `Some(var)`
//! for `return std::move(x);` and `return Wrapper(std::move(x));`,
//! which then triggered the Return-statement ownership check at
//! `analysis/mod.rs:1791`. Because the same Move statement marks `x`
//! as Moved BEFORE the Return is processed, the check would always
//! fire with "Cannot return 'x' because it has been moved" on the
//! canonical std::move-into-return idiom (the rrr/fiber_channel.cpp
//! shape — `return rusty::Some(std::move(f));`).
//!
//! Fix: the Move arm still pushes the IR Move statement (so subsequent
//! uses of `x` are flagged), but returns None as the Return source.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer_on_file(cpp_file: &std::path::Path) -> (bool, String) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()]);

    if cfg!(target_os = "macos") {
        cmd.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output().expect("Failed to execute analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

#[test]
fn test_return_std_move_param_does_not_fire() {
    // `return std::move(p);` where `p` is a parameter — the canonical
    // move-into-return for forwarding values. Must not fire
    // "Cannot return 'p' because it has been moved".
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
#include <utility>

struct Owned {
    int x;
};

// @safe
Owned forward_value(Owned p) {
    return std::move(p);
}
"#;

    let cpp_path = temp_dir.path().join("return_std_move_param.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (_success, output) = run_analyzer_on_file(&cpp_path);

    // The constructor of Owned may not be marked @safe, but that's a
    // different violation. What we care about is the move-then-return
    // pattern not firing "Cannot return 'p' because it has been moved".
    assert!(
        !output.contains("Cannot return"),
        "no 'Cannot return ... has been moved' expected on \
         `return std::move(p);`.\nOutput:\n{}",
        output
    );
}

#[test]
fn test_use_after_explicit_move_still_flagged() {
    // After std::move, the variable IS Moved. A subsequent USE (not
    // a return-of-move) MUST still fire.
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
#include <utility>

struct Owned {
    int x;
};

void consume(Owned o);

// @safe
void misuse() {
    Owned local{42};
    consume(std::move(local));
    consume(std::move(local));   // use-after-move — should fire
}
"#;

    let cpp_path = temp_dir.path().join("use_after_move.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (_success, output) = run_analyzer_on_file(&cpp_path);

    // The second consume(std::move(local)) is a use-after-move.
    assert!(
        output.to_lowercase().contains("moved") || output.contains("Use after move"),
        "use-after-move must still be flagged on a non-return use site.\nOutput:\n{}",
        output
    );
}
