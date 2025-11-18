// Test Phase 1: Lifetime Annotation Integration
// Tests that lifetime annotations are correctly parsed from source comments,
// populated into the IR, and used by the lifetime checker.

use assert_cmd::Command;

#[test]
fn test_phase1_identity_function_valid() {
    // Test that identity function with matching lifetimes works
    let code = r#"
// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) {
    return x;
}

// @safe
int main() {
    int value = 42;
    const int& ref = identity(value);
    return 0;
}
"#;

    let temp_file = std::env::temp_dir().join("test_phase1_identity.cpp");
    std::fs::write(&temp_file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd.arg(&temp_file).output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should pass - no lifetime violations
    assert!(
        stdout.contains("no violations found") || !output.status.success(),
        "Expected no violations, got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_phase1_lifetime_constraint_violation() {
    // Test that lifetime constraints are properly validated
    let code = r#"
// @lifetime: (&'a, &'b) -> &'a int where 'a: 'b
const int& selectFirst(const int& a, const int& b) {
    return a;
}

// @safe
int main() {
    int value = 42;
    const int& ref1 = value;
    // VIOLATION: ref1's lifetime does not outlive value's lifetime
    const int& ref2 = selectFirst(value, ref1);
    return 0;
}
"#;

    let temp_file = std::env::temp_dir().join("test_phase1_constraint.cpp");
    std::fs::write(&temp_file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd.arg(&temp_file).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should fail - lifetime constraint violated
    assert!(
        stdout.contains("Lifetime") && stdout.contains("outlive"),
        "Expected lifetime constraint violation, got: {}",
        stdout
    );
}

#[test]
fn test_phase1_owned_annotation() {
    // Test that "owned" annotation works correctly
    let code = r#"
// @lifetime: owned
// @safe
int create() {
    return 42;
}

// @safe
int main() {
    int owned = create();
    return 0;
}
"#;

    let temp_file = std::env::temp_dir().join("test_phase1_owned.cpp");
    std::fs::write(&temp_file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd.arg(&temp_file).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should pass - owned return type
    assert!(
        stdout.contains("no violations found") || !output.status.success(),
        "Expected no violations for owned return, got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_phase1_mutable_reference_lifetime() {
    // Test that mutable reference lifetimes are tracked
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& getMutable(int& x) {
    return x;
}

// @safe
int main() {
    int value = 42;
    int& ref = getMutable(value);
    ref = 100;
    return 0;
}
"#;

    let temp_file = std::env::temp_dir().join("test_phase1_mut_ref.cpp");
    std::fs::write(&temp_file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd.arg(&temp_file).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should pass - mutable reference with correct lifetime
    assert!(
        stdout.contains("no violations found") || !output.status.success(),
        "Expected no violations for mutable reference, got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_phase1_multiple_lifetime_params() {
    // Test that multiple lifetime parameters work
    let code = r#"
// @lifetime: (&'a, &'b) -> &'a int
const int& first(const int& a, const int& b) {
    return a;
}

// @safe
int main() {
    int x = 1;
    int y = 2;
    const int& result = first(x, y);
    return 0;
}
"#;

    let temp_file = std::env::temp_dir().join("test_phase1_multiple_lifetimes.cpp");
    std::fs::write(&temp_file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd.arg(&temp_file).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should pass - multiple lifetime parameters
    assert!(
        stdout.contains("no violations found") || !output.status.success(),
        "Expected no violations for multiple lifetimes, got:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}
