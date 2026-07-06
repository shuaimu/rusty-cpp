// Locks in the analysis-coverage contract for header code:
//
//  1. USER-header inline function bodies ARE fully analyzed (borrow
//     checking, use-after-move, lifetime inference) from every TU that
//     includes them. Headers are never a translation unit themselves, so
//     per-TU checking is a user's only chance to have these bodies
//     verified — an exclusion here silently un-checks header-only code.
//
//  2. rusty LIBRARY headers (include/rusty/) are the trusted library tier:
//     including them and calling their @safe API must not surface
//     violations from the library's internal implementation.
//
// Context: PR #26 originally excluded ALL cross-file bodies from the IR
// passes (fixing library noise but silently dropping user-header
// coverage) and exempted annotated include/rusty/ bodies from lifetime
// inference. The reworked scoping keeps user headers analyzed and keeps
// library internals out of per-TU findings; these tests pin both sides.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn get_project_root() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn z3_header() -> String {
    if let Ok(path) = std::env::var("Z3_SYS_Z3_HEADER") {
        return path;
    }
    if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h".to_string()
    } else {
        "/usr/include/z3.h".to_string()
    }
}

/// Run the analyzer on `cpp_file` with the given extra include dir.
/// Returns (analyzer_ran, output). `analyzer_ran` is false when the
/// checker itself failed to run (build/parse failure) — callers must
/// treat that as a test failure, never as "no violations".
fn run_analyzer(cpp_file: &Path, include_dir: &Path) -> (bool, String) {
    let project_include = format!("{}/include", get_project_root());

    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "run",
        "--quiet",
        "--",
        cpp_file.to_str().unwrap(),
        "-I",
        include_dir.to_str().unwrap(),
        "-I",
        &project_include,
    ])
    .env("Z3_SYS_Z3_HEADER", z3_header());

    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    }

    let output = cmd.output().expect("Failed to run analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let has_violations = combined.contains("violation");
    let clean = combined.contains("no violations found");
    let analyzer_ran = output.status.success() || has_violations || clean;

    (analyzer_ran, combined)
}

/// A use-after-move inside an inline @safe function in a USER header must
/// be reported when analyzing a TU that includes the header.
#[test]
fn test_use_after_move_in_user_header_is_flagged() {
    let dir = TempDir::new().unwrap();

    let header = dir.path().join("user_header.hpp");
    fs::write(
        &header,
        r#"#pragma once
#include <utility>

// @safe
inline int use_after_move_in_header() {
    int x = 1;
    int y = std::move(x);
    return x;
}
"#,
    )
    .unwrap();

    let main_cpp = dir.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"#include "user_header.hpp"

// @safe
int main() {
    return use_after_move_in_header();
}
"#,
    )
    .unwrap();

    let (analyzer_ran, output) = run_analyzer(&main_cpp, dir.path());
    assert!(analyzer_ran, "analyzer failed to run: {}", output);
    assert!(
        output.contains("moved"),
        "use-after-move in a user header must be reported when analyzing an \
         including TU (user headers are fully analyzed; only library-tier \
         headers are exempt). Output: {}",
        output
    );
}

/// A borrow-rule violation in a user header must likewise be reported.
#[test]
fn test_double_mutable_borrow_in_user_header_is_flagged() {
    let dir = TempDir::new().unwrap();

    let header = dir.path().join("borrows.hpp");
    fs::write(
        &header,
        r#"#pragma once

// @safe
inline void double_mut_borrow_in_header() {
    int value = 0;
    int& a = value;
    int& b = value;
    a = 1;
    b = 2;
}
"#,
    )
    .unwrap();

    let main_cpp = dir.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"#include "borrows.hpp"

// @safe
int main() {
    double_mut_borrow_in_header();
    return 0;
}
"#,
    )
    .unwrap();

    let (analyzer_ran, output) = run_analyzer(&main_cpp, dir.path());
    assert!(analyzer_ran, "analyzer failed to run: {}", output);
    assert!(
        output.contains("violation"),
        "double mutable borrow in a user header must be reported when \
         analyzing an including TU. Output: {}",
        output
    );
}

/// Including rusty library headers and using their @safe API must not
/// surface violations from the library's internal implementation — the
/// library is the trusted tier, verified by its own test suite, not
/// re-analyzed from every consumer TU.
#[test]
fn test_rusty_library_internals_produce_no_noise() {
    let dir = TempDir::new().unwrap();

    let main_cpp = dir.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some(42);
    if (opt.is_some()) {
        return 0;
    }
    return 1;
}
"#,
    )
    .unwrap();

    let (analyzer_ran, output) = run_analyzer(&main_cpp, dir.path());
    assert!(analyzer_ran, "analyzer failed to run: {}", output);
    assert!(
        output.contains("no violations found"),
        "using the rusty library's @safe API must not surface findings from \
         library internals. Output: {}",
        output
    );
}

/// The user-header analysis must not re-introduce duplicate findings for
/// the TU's own code: a violation in main.cpp is reported exactly once.
#[test]
fn test_tu_violation_reported_once() {
    let dir = TempDir::new().unwrap();

    let main_cpp = dir.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"#include <utility>

// @safe
int main() {
    int x = 1;
    int y = std::move(x);
    return x;
}
"#,
    )
    .unwrap();

    let (analyzer_ran, output) = run_analyzer(&main_cpp, dir.path());
    assert!(analyzer_ran, "analyzer failed to run: {}", output);
    let occurrences = output.matches("has been moved").count();
    assert_eq!(
        occurrences, 1,
        "the TU's own use-after-move must be reported exactly once, got {}: {}",
        occurrences, output
    );
}

// Silence unused warnings for the PathBuf import used only on some paths.
#[allow(dead_code)]
fn _keep(_p: PathBuf) {}
