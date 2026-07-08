// Tests for unbraced `if`/`else`/loop bodies, `else if` chains, loop
// conditions, and the blank-line rules for block-level `@unsafe`
// annotations (PR #28 + follow-ups). Before these fixes, unsafe
// operations in unbraced substatements and loop conditions were
// silently dropped at parse time, and only the first violation of a
// branching function was reported.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(source: &str) -> Result<Vec<String>, String> {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.cpp");
    fs::write(&file_path, source).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", file_path.to_str().unwrap()])
        .output()
        .map_err(|e| format!("Failed to run checker: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut violations = Vec::new();
    for line in stdout.lines() {
        if (line.contains("unsafe") || line.contains("violation"))
            && !line.contains("warning:")
            && !line.contains("-->")
            && !line.trim().starts_with("|")
            && !line.contains("✓")
        {
            violations.push(line.to_string());
        }
    }

    Ok(violations)
}

const UNSAFE_HELPERS: &str = r#"
// @unsafe
void unsafe_helper();
// @unsafe
bool unsafe_pred();
// @unsafe
int unsafe_get();
"#;

fn with_helpers(body: &str) -> String {
    format!("{}\n{}", UNSAFE_HELPERS, body)
}

#[test]
fn test_unbraced_if_body_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    if (flag) unsafe_helper();
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_helper")),
        "unsafe call in unbraced if body should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_unbraced_else_body_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    if (flag) { }
    else unsafe_helper();
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_helper")),
        "unsafe call in unbraced else body should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_unbraced_while_body_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    while (flag) unsafe_helper();
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_helper")),
        "unsafe call in unbraced while body should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_else_if_branch_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool a, bool b) {
    if (a) { }
    else if (b) {
        unsafe_helper();
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_helper")),
        "unsafe call in else-if branch should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_violations_in_both_branches_all_reported() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    if (flag) {
        unsafe_helper();
    } else {
        unsafe_helper();
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    let count = violations
        .iter()
        .filter(|v| v.contains("unsafe_helper"))
        .count();
    assert!(
        count >= 2,
        "both branch violations should be reported, got {}: {:?}",
        count,
        violations
    );
}

#[test]
fn test_while_condition_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f() {
    while (unsafe_pred()) { }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_pred")),
        "unsafe call in while condition should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_do_while_condition_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f() {
    do { } while (unsafe_pred());
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_pred")),
        "unsafe call in do-while condition should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_for_condition_unsafe_call_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f() {
    for (int i = 0; unsafe_get() > i; ++i) { }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_get")),
        "unsafe call in for condition should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_unbraced_if_pointer_deref_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(int* p, bool flag) {
    if (flag) *p = 1;
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("pointer")),
        "pointer deref in unbraced if body should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_unbraced_compound_assign_pointer_deref_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(int* q, bool flag) {
    if (flag) *q += 1;
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("pointer")),
        "pointer deref in unbraced compound assignment should be flagged: {:?}",
        violations
    );
}

#[test]
fn test_loop_nested_in_unbraced_if_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    if (flag) while (unsafe_pred()) { }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_pred")),
        "loop as unbraced if body should still be analyzed: {:?}",
        violations
    );
}

#[test]
fn test_unsafe_block_inside_branch_not_flagged() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    if (flag) {
        // @unsafe
        {
            unsafe_helper();
        }
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.is_empty(),
        "@unsafe block inside a branch must still suppress errors: {:?}",
        violations
    );
}

#[test]
fn test_clean_for_loop_no_false_positive() {
    let source = with_helpers(
        r#"
// @safe
void f(int n) {
    for (int i = 0; i < n; ++i) {
        int x = i;
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.is_empty(),
        "plain counting loop must not produce violations: {:?}",
        violations
    );
}

#[test]
fn test_unsafe_annotation_attaches_across_one_blank_line() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    // @unsafe

    {
        unsafe_helper();
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.is_empty(),
        "@unsafe separated by ONE blank line should still attach: {:?}",
        violations
    );
}

#[test]
fn test_unsafe_annotation_severed_by_two_blank_lines() {
    let source = with_helpers(
        r#"
// @safe
void f(bool flag) {
    // @unsafe: stray note, not adjacent to the block


    {
        unsafe_helper();
    }
}
"#,
    );
    let violations = compile_and_check(&source).unwrap();
    assert!(
        violations.iter().any(|v| v.contains("unsafe_helper")),
        "@unsafe two blank lines above must NOT silence the block: {:?}",
        violations
    );
}
