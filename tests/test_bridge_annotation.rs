// Tests for the `@bridge` safety annotation.
//
// `@bridge` marks a function whose own body is not subject to @safe body
// checks, but whose calls from @safe callers are nonetheless allowed.
// Semantics: the bridge propagates safety from its callees rather than
// gating on its own body. Used for universal dispatchers like
// `rusty::deref_call`, which exist only to forward to a caller-provided
// lambda.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_checker(source: &str) -> (bool, String) {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.cpp");
    fs::write(&file_path, source).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", file_path.to_str().unwrap()])
        .output()
        .expect("failed to run checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    (output.status.success(), stdout.into_owned())
}

fn violations(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| {
            // Match the same shape the existing safe/unsafe tests use.
            (line.contains("unsafe") || line.contains("violation"))
                && !line.contains("warning:")
                && !line.contains("-->")
                && !line.trim().starts_with("|")
                && !line.contains("\u{2713}") // ✓ checkmark
        })
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn test_safe_can_call_bridge_function() {
    // @safe caller invokes a @bridge function — the bridge's own body is
    // trusted; the call is allowed without an @unsafe block.
    let source = r#"
// @bridge
int my_bridge() {
    return 42;
}

// @safe
int caller() {
    return my_bridge();  // OK: @safe may call @bridge
}
"#;
    let (_status, output) = run_checker(source);
    let violations = violations(&output);
    assert!(
        violations.is_empty(),
        "expected no violations, got: {:#?}\n--- full output ---\n{}",
        violations,
        output
    );
}

#[test]
fn test_bridge_body_not_subject_to_safe_checks() {
    // The bridge's own body can do operations that would normally require
    // an @unsafe block — the contract is "trust the bridge author."
    let source = r#"
// @unsafe
int raw_helper() { return 1; }

// @bridge
int my_bridge() {
    return raw_helper();  // OK: @bridge body is not @safe-checked
}

// @safe
int caller() {
    return my_bridge();  // OK: @safe may call @bridge
}
"#;
    let (_status, output) = run_checker(source);
    let violations = violations(&output);
    assert!(
        violations.is_empty(),
        "expected no violations, got: {:#?}\n--- full output ---\n{}",
        violations,
        output
    );
}

#[test]
fn test_safe_to_unsafe_without_bridge_still_errors() {
    // Sanity check: an ordinary @unsafe function called from @safe still
    // requires an @unsafe block. The @bridge feature must not relax this.
    let source = r#"
// @unsafe
int dangerous() { return 1; }

// @safe
int caller() {
    return dangerous();  // ERROR: @safe cannot call @unsafe directly
}
"#;
    let (_status, output) = run_checker(source);
    let violations = violations(&output);
    assert!(
        !violations.is_empty(),
        "expected at least one violation for @safe calling @unsafe, got none\n--- full output ---\n{}",
        output
    );
}

#[test]
fn test_unsafe_can_still_call_bridge() {
    // @unsafe caller invoking @bridge: trivially allowed (the @unsafe
    // caller can call anything).
    let source = r#"
// @bridge
int my_bridge() { return 42; }

// @unsafe
int caller() {
    return my_bridge();
}
"#;
    let (_status, output) = run_checker(source);
    let violations = violations(&output);
    assert!(
        violations.is_empty(),
        "expected no violations, got: {:#?}\n--- full output ---\n{}",
        violations,
        output
    );
}
