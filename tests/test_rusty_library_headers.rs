//! Tests that verify the rusty library headers pass the checker
//!
//! These tests run the analyzer directly on the library headers to ensure
//! they don't have any violations. This is important because:
//! 1. Library headers are skipped when analyzing user code (treated as system headers)
//! 2. But they should still be valid when checked directly
//! 3. This catches issues like missing @unsafe blocks in Cell::set()

use std::process::Command;
use std::path::PathBuf;

fn get_project_root() -> PathBuf {
    // The CARGO_MANIFEST_DIR is set to the package root during testing
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    PathBuf::from(manifest_dir)
}

fn run_analyzer_on_header(header_path: &str) -> (bool, String, String) {
    let project_root = get_project_root();
    let binary_path = project_root.join("target/release/rusty-cpp-checker");

    // Use relative paths - this is important because the analyzer's is_system_header_or_std
    // check uses path.contains("/include/rusty/") which only triggers for absolute paths
    // With relative paths like "include/rusty/cell.hpp", the file gets analyzed
    let mut cmd = Command::new(&binary_path);
    cmd.args(&[
        header_path,          // Use relative path
        "-I", "include"       // Use relative include path
    ])
    .current_dir(&project_root);

    // Set library paths based on platform
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output()
        .expect("Failed to execute analyzer");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status.success(), stdout, stderr)
}

fn check_header_passes(header_path: &str) {
    let (success, stdout, stderr) = run_analyzer_on_header(header_path);
    let combined = format!("{}{}", stdout, stderr);

    // Look for our specific violation output pattern "Found N violation(s)"
    let has_violations = combined.contains("Found") && combined.contains("violation(s)");

    if has_violations {
        panic!(
            "Header {} has violations:\n{}",
            header_path, combined
        );
    }

    // Exit code 0 means success (no violations)
    // Exit code 1 means violations found
    assert!(success,
        "Header {} check failed (exit code != 0):\nstdout: {}\nstderr: {}",
        header_path, stdout, stderr
    );
}

fn check_header_has_expected_violations(header_path: &str, expected_count: usize) {
    let (_success, stdout, stderr) = run_analyzer_on_header(header_path);
    let combined = format!("{}{}", stdout, stderr);

    // Count violations - look in both stdout and stderr
    let violation_line = combined.lines()
        .find(|l| l.contains("Found") && l.contains("violation"));

    if let Some(line) = violation_line {
        // Extract number from "Found N violation(s)"
        let count: usize = line.split_whitespace()
            .find_map(|w| w.parse().ok())
            .unwrap_or(0);

        assert_eq!(
            count, expected_count,
            "Header {} expected {} violations but found {}:\n{}",
            header_path, expected_count, count, combined
        );
    } else if expected_count > 0 {
        panic!(
            "Header {} expected {} violations but found none:\n{}",
            header_path, expected_count, combined
        );
    }
}

// ============================================================================
// Tests for headers that should pass with no violations
// ============================================================================

#[test]
fn test_box_hpp_passes() {
    check_header_passes("include/rusty/box.hpp");
}

#[test]
fn test_move_hpp_passes() {
    check_header_passes("include/rusty/move.hpp");
}

// cell.hpp and unsafe_cell.hpp now pass with no violations
#[test]
fn test_cell_hpp_passes() {
    check_header_passes("include/rusty/cell.hpp");
}

#[test]
fn test_unsafe_cell_hpp_passes() {
    check_header_passes("include/rusty/unsafe_cell.hpp");
}

// ============================================================================
// Tests for headers that have known violations (to be fixed)
// These tests document the current state and will fail when fixed
// ============================================================================

#[test]
#[ignore] // Remove ignore when option.hpp is fixed (currently 29 violations)
fn test_option_hpp_passes() {
    check_header_passes("include/rusty/option.hpp");
}

#[test]
#[ignore] // Remove ignore when result.hpp is fixed (currently 18 violations)
fn test_result_hpp_passes() {
    check_header_passes("include/rusty/result.hpp");
}

#[test]
#[ignore] // Remove ignore when fn.hpp is fixed (currently 28 violations)
fn test_fn_hpp_passes() {
    check_header_passes("include/rusty/fn.hpp");
}

// Test that documents current violations in option.hpp
#[test]
fn test_option_hpp_current_violations() {
    // option.hpp currently has 29 violations (lifetime annotations, etc.)
    check_header_has_expected_violations("include/rusty/option.hpp", 29);
}

// Test that documents current violations in result.hpp
#[test]
fn test_result_hpp_current_violations() {
    // result.hpp currently has 18 violations
    check_header_has_expected_violations("include/rusty/result.hpp", 18);
}

// Test that documents current violations in fn.hpp
#[test]
fn test_fn_hpp_current_violations() {
    // fn.hpp currently has 28 violations
    check_header_has_expected_violations("include/rusty/fn.hpp", 28);
}
