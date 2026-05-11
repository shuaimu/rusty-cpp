/// Tests for borrow tracking on structs with reference members.
///
/// When a struct has a reference member (e.g., `const T& ref;`) and is
/// constructed from a variable, that variable is considered immutably
/// borrowed for the lifetime of the struct. While the borrow is active:
///   - assigning to the source is rejected
///   - moving from the source is rejected (via existing transitive check)
///   - multiple immutable borrows of the same source are still allowed
///   - the borrow is released when the struct goes out of scope
///
/// These tests pin the behavior added by the StructBorrow IR node and the
/// "cannot assign to borrowed variable" check in the Assign handler.
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);

    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output().expect("Failed to execute analyzer");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

fn create_temp_cpp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn analyze(source: &str) -> (bool, String) {
    let temp_file = create_temp_cpp_file(source);
    let (_success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    let no_violations = output.contains("no violations found");

    (!has_violations || no_violations, output)
}

// =============================================================================
// Borrow conflict: assignment to borrowed source
// =============================================================================

#[test]
fn test_assign_to_borrowed_source_fails() {
    // A struct with a reference member borrows from its constructor argument.
    // While the struct is alive, assigning to the source is forbidden.
    // Rust equivalent: error[E0506]: cannot assign to `x` because it is borrowed.
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
int test() {
    int x = 42;
    Holder h(x);          // h borrows x
    x = 100;              // ERROR: cannot assign to x while borrowed
    return h.ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect assignment to borrowed source. Output: {}",
        output
    );
    assert!(
        output.contains("Cannot assign to 'x'") && output.contains("borrowed by"),
        "Error should mention assignment-to-borrowed. Got: {}",
        output
    );
}

#[test]
fn test_assign_to_borrowed_source_brace_init_fails() {
    // Same as above but with brace-initialization syntax. The struct has an
    // explicit constructor so the parser still emits a constructor CallExpr.
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
int test() {
    int x = 42;
    Holder h{x};          // brace init still calls explicit ctor
    x = 100;              // ERROR
    return h.ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect assignment to borrowed source with brace init. Output: {}",
        output
    );
    assert!(
        output.contains("Cannot assign to 'x'") && output.contains("borrowed by"),
        "Error should mention assignment-to-borrowed. Got: {}",
        output
    );
}

// =============================================================================
// Borrow conflict: move of borrowed source
// =============================================================================

#[test]
fn test_move_borrowed_source_fails() {
    // std::move on a value that's borrowed by a struct's reference member.
    // The existing transitive-borrow check on Move catches this once the
    // StructBorrow has been recorded.
    let source = r#"
namespace std { template<typename T> T&& move(T& t) { return static_cast<T&&>(t); } }

struct Foo { int v; };

// @safe
struct Holder {
    const Foo& ref;
    Holder(const Foo& r) : ref(r) {}
};

// @safe
int test() {
    Foo x{42};
    Holder h(x);                 // h borrows x
    Foo y = std::move(x);        // ERROR: cannot move x while borrowed
    return h.ref.v;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect move of borrowed source. Output: {}",
        output
    );
    assert!(
        output.contains("Cannot move 'x'") && output.contains("borrowed by"),
        "Error should mention move-of-borrowed. Got: {}",
        output
    );
}

// =============================================================================
// Negative cases: things that should NOT trigger an error
// =============================================================================

#[test]
fn test_multiple_immutable_borrows_ok() {
    // Multiple structs each holding an immutable reference to the same source
    // are allowed, matching Rust's "any number of &T" rule.
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
int test() {
    int x = 42;
    Holder h1(x);
    Holder h2(x);                // OK: multiple immutable borrows
    return h1.ref + h2.ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Multiple immutable borrows should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_borrow_released_at_end_of_scope_ok() {
    // Once the borrowing struct goes out of scope, the source can be modified
    // again. This relies on the existing scope-exit cleanup that clears
    // borrows from the dying variable.
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
int test() {
    int x = 42;
    {
        Holder h(x);             // h borrows x
        int y = h.ref;
    }                            // h dies here, borrow released
    x = 100;                     // OK: no live borrow
    return x;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Assignment after scope exit should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_assign_unrelated_variable_ok() {
    // Sanity check: assigning to a variable that is NOT borrowed should be
    // allowed even when an unrelated borrow is active.
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
int test() {
    int x = 42;
    int y = 0;
    Holder h(x);                 // h borrows x, NOT y
    y = 100;                     // OK: y is not borrowed
    return h.ref + y;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Assignment to unrelated variable should be allowed. Output: {}",
        output
    );
}
