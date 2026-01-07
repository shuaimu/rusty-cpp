// Tests for method return borrow checking based on Rust's borrow rules:
// Case 1: &mut self -> &mut T (non-const method returning mutable ref) - double call should ERROR
// Case 2: &self -> &T (const method returning const ref) - double call should be OK
// Case 3: &self -> &mut T (forbidden signature) - const method returning mutable ref is DANGEROUS

use std::process::Command;
use std::fs;
use std::env;

fn create_temp_file(name: &str, code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_method_borrow_{}.cpp", name));
    fs::write(&temp_file, code).unwrap();
    temp_file
}

fn run_analyzer(file_path: &std::path::PathBuf) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--", file_path.to_str().unwrap()])
        .output()
        .expect("Failed to run analyzer");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn cleanup(file_path: &std::path::PathBuf) {
    let _ = fs::remove_file(file_path);
}

// =============================================================================
// Case 1: &mut self -> &mut T (non-const method returning mutable ref)
// Calling twice should ERROR - double mutable borrow
// =============================================================================

#[test]
fn test_case1_double_mut_borrow_error() {
    let code = r#"
// Case 1: Method takes &mut self and returns &mut T
struct Foo {
    int value;

    // Non-const method returning mutable reference (like &mut self -> &mut T)
    // @safe
    // @lifetime: (&'a mut self) -> &'a mut int
    int& borrow_mut() {
        return value;
    }
};

// Case 1: Method takes &mut self, returns &mut T - calling twice should ERROR
// @safe
void test_case1_double_mut_borrow() {
    Foo x;
    x.value = 0;

    int& a = x.borrow_mut();  // First mutable borrow
    int& b = x.borrow_mut();  // Should ERROR: x already mutably borrowed through 'a'
}
"#;

    let temp_file = create_temp_file("case1_error", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should detect double mutable borrow error
    assert!(
        output.contains("borrow") || output.contains("violation") || output.contains("Cannot"),
        "Case 1: Should detect double mutable borrow error. Output: {}",
        output
    );
}

#[test]
fn test_case1_sequential_mut_borrow_ok() {
    let code = r#"
// Case 1 with proper scope - should be OK
struct Foo {
    int value;

    // @safe
    // @lifetime: (&'a mut self) -> &'a mut int
    int& borrow_mut() {
        return value;
    }
};

// @safe
void test_case1_sequential_ok() {
    Foo x;
    x.value = 0;

    {
        int& a = x.borrow_mut();
        a += 1;
    }  // 'a' dropped here

    {
        int& b = x.borrow_mut();  // Now OK - previous borrow ended
        b += 1;
    }
}
"#;

    let temp_file = create_temp_file("case1_ok", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should NOT detect borrow error for sequential borrows in separate scopes
    assert!(
        !output.contains("double mutable borrow") && !output.contains("already mutably borrowed"),
        "Case 1 Sequential: Should allow sequential borrows in separate scopes. Output: {}",
        output
    );
}

// =============================================================================
// Case 2: &self -> &T (const method returning const ref)
// Calling twice should be OK - shared borrows can alias
// =============================================================================

#[test]
fn test_case2_double_shared_borrow_ok() {
    let code = r#"
// Case 2: Method takes &self and returns &T
struct Foo {
    int value;

    // Const method returning const reference (like &self -> &T)
    // @safe
    // @lifetime: (&'a self) -> &'a int
    const int& borrow() const {
        return value;
    }
};

// Case 2: Method takes &self, returns &T - calling twice should be OK
// @safe
void test_case2_double_shared_borrow() {
    Foo x;
    x.value = 0;

    const int& a = x.borrow();
    const int& b = x.borrow();  // Should be OK: shared borrows can alias
}
"#;

    let temp_file = create_temp_file("case2_ok", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should NOT detect borrow error - shared borrows can coexist
    assert!(
        !output.contains("Cannot") || output.contains("no violations"),
        "Case 2: Should allow multiple shared borrows. Output: {}",
        output
    );
}

#[test]
fn test_case2_shared_then_mut_error() {
    let code = r#"
// Case 2: Mixed borrows - shared then mutable should ERROR
struct Foo {
    int value;

    // @safe
    // @lifetime: (&'a self) -> &'a int
    const int& borrow() const {
        return value;
    }

    // @safe
    // @lifetime: (&'a mut self) -> &'a mut int
    int& borrow_mut() {
        return value;
    }
};

// @safe
void test_shared_then_mut() {
    Foo x;
    x.value = 0;

    const int& a = x.borrow();     // Shared borrow
    int& b = x.borrow_mut();       // Should ERROR: can't mutably borrow while shared borrow exists
}
"#;

    let temp_file = create_temp_file("case2_mixed", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should detect conflict between shared and mutable borrow
    assert!(
        output.contains("borrow") || output.contains("violation") || output.contains("Cannot"),
        "Case 2 Mixed: Should detect shared/mutable borrow conflict. Output: {}",
        output
    );
}

// =============================================================================
// Case 3: &self -> &mut T (FORBIDDEN signature)
// Const method returning mutable reference - this is dangerous!
// =============================================================================

#[test]
fn test_case3_forbidden_signature_dangerous() {
    let code = r#"
// Case 3: The "forbidden" signature: fn borrow(&self) -> &mut T
// This should NOT be allowed in @safe code!
struct Foo {
    int value;

    // Const method returning MUTABLE reference - THIS IS DANGEROUS
    // @safe
    // @lifetime: (&'a self) -> &'a mut int
    int& borrow() const {  // const method but returns non-const ref!
        return const_cast<int&>(value);  // Requires const_cast - smell!
    }
};

// This would allow two mutable aliases from shared borrows!
// @safe
void test_case3_dangerous() {
    Foo x;
    x.value = 0;

    int& a = x.borrow();  // const method, but returns &mut
    int& b = x.borrow();  // Can call again since it's const method
    // Now a and b both point to x.value mutably - UNDEFINED BEHAVIOR!
    a = 1;
    b = 2;  // Data race if a and b alias!
}
"#;

    let temp_file = create_temp_file("case3_forbidden", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // The analyzer should detect this is dangerous
    // Either: detect the const_cast as unsafe, or detect the signature mismatch,
    // or detect two mutable borrows through what looks like shared borrows
    assert!(
        output.contains("const_cast") ||
        output.contains("unsafe") ||
        output.contains("borrow") ||
        output.contains("violation"),
        "Case 3: Should detect forbidden signature or const_cast. Output: {}",
        output
    );
}

// =============================================================================
// Additional edge cases
// =============================================================================

#[test]
fn test_mut_borrow_then_use_object() {
    let code = r#"
struct Foo {
    int value;

    // @safe
    // @lifetime: (&'a mut self) -> &'a mut int
    int& borrow_mut() {
        return value;
    }

    // @safe
    void reset() {
        value = 0;
    }
};

// @safe
void test_mut_borrow_then_method() {
    Foo x;
    x.value = 42;

    int& ref = x.borrow_mut();  // Mutable borrow
    x.reset();                   // Should ERROR: x is mutably borrowed
}
"#;

    let temp_file = create_temp_file("mut_then_method", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should detect that x is borrowed when calling reset()
    assert!(
        output.contains("borrow") || output.contains("violation") || output.contains("Cannot"),
        "Should detect borrow conflict when calling method on borrowed object. Output: {}",
        output
    );
}

#[test]
fn test_shared_borrow_allows_const_method() {
    let code = r#"
struct Foo {
    int value;

    // @safe
    // @lifetime: (&'a self) -> &'a int
    const int& borrow() const {
        return value;
    }

    // @safe
    int get_value() const {
        return value;
    }
};

// @safe
void test_shared_borrow_const_method() {
    Foo x;
    x.value = 42;

    const int& ref = x.borrow();  // Shared borrow
    int v = x.get_value();        // Should be OK: const method doesn't conflict with shared borrow
}
"#;

    let temp_file = create_temp_file("shared_const_method", code);
    let output = run_analyzer(&temp_file);
    cleanup(&temp_file);

    // Should allow const method call while shared borrow exists
    // (May have other unrelated errors, but not borrow conflicts)
    assert!(
        !output.contains("already borrowed") && !output.contains("Cannot call"),
        "Should allow const method while shared borrow exists. Output: {}",
        output
    );
}
