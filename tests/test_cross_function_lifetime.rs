/// Tests for cross-function lifetime enforcement
///
/// These tests verify that RustyCpp catches lifetime violations that span
/// across function boundaries - a key feature of Rust's borrow checker.
///
/// Categories tested:
/// 1. Returning reference to temporary
/// 2. Returning reference to local variable
/// 3. Struct storing reference that outlives referent
/// 4. Output parameter lifetime violations
/// 5. Method returns reference to dying object
/// 6. Chained function calls with lifetime issues

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
// CATEGORY 1: Returning reference to temporary
// =============================================================================

#[test]
fn test_return_ref_to_temporary_literal() {
    // Passing a temporary literal to a function that returns a reference
    // The temporary dies at end of statement, leaving a dangling reference
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void bad() {
    const int& ref = identity(42);  // ERROR: 42 is temporary, dies after statement
    int y = ref;  // Using dangling reference
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect reference to temporary. Output: {}",
        output
    );
    assert!(
        output.contains("lifetime") || output.contains("dangling") || output.contains("temporary"),
        "Error should mention lifetime/dangling/temporary. Got: {}",
        output
    );
}

#[test]
fn test_return_ref_to_temporary_rvalue() {
    // Temporary created from expression
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void bad() {
    int a = 10, b = 20;
    const int& ref = identity(a + b);  // ERROR: a+b creates temporary
    int y = ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect reference to temporary expression. Output: {}",
        output
    );
}

#[test]
fn test_return_ref_to_valid_variable_ok() {
    // This should be OK - returning reference to a variable that outlives the reference
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void good() {
    int x = 42;
    const int& ref = identity(x);  // OK: x outlives ref
    int y = ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Reference to valid variable should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 2: Returning reference to local variable
// =============================================================================

#[test]
fn test_return_ref_to_local() {
    // Classic dangling reference - returning reference to stack local
    let source = r#"
// @safe
int& bad() {
    int x = 42;
    return x;  // ERROR: returning reference to local variable
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect return of reference to local. Output: {}",
        output
    );
    assert!(
        output.contains("local") || output.contains("dangling") || output.contains("lifetime"),
        "Error should mention local/dangling/lifetime. Got: {}",
        output
    );
}

#[test]
fn test_return_const_ref_to_local() {
    // Same issue with const reference
    let source = r#"
// @safe
const int& bad() {
    int x = 42;
    return x;  // ERROR: still dangling even if const
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect return of const reference to local. Output: {}",
        output
    );
}

#[test]
fn test_return_ref_to_parameter_ok() {
    // This should be OK - parameter outlives function
    let source = r#"
// @lifetime: (&'a mut) -> &'a mut
// @safe
int& identity(int& x) { return x; }

// @safe
void good() {
    int x = 42;
    int& ref = identity(x);  // OK
    ref = 10;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Return reference to parameter should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 3: Struct storing reference that outlives referent
// =============================================================================

#[test]
#[ignore = "TODO: Struct reference member lifetime tracking not yet implemented"]
fn test_struct_ref_outlives_referent() {
    // Struct holds reference, but referent dies before struct
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
void bad() {
    Holder* h;
    {
        int x = 42;
        h = new Holder(x);  // h->ref points to x
    }  // x dies here
    int y = h->ref;  // ERROR: h->ref is dangling
    delete h;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect struct reference outliving referent. Output: {}",
        output
    );
}

#[test]
#[ignore = "TODO: Struct reference member lifetime tracking not yet implemented"]
fn test_return_struct_with_dangling_ref() {
    // Returning a struct whose reference member points to local
    let source = r#"
// @safe
struct Holder {
    const int& ref;
};

// @safe
Holder bad() {
    int x = 42;
    return Holder{x};  // ERROR: Holder.ref will dangle
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect returning struct with dangling ref. Output: {}",
        output
    );
}

#[test]
fn test_struct_ref_same_scope_ok() {
    // Struct and referent in same scope - should be OK
    let source = r#"
// @safe
struct Holder {
    const int& ref;
    Holder(const int& r) : ref(r) {}
};

// @safe
void good() {
    int x = 42;
    Holder h(x);  // OK: x and h have same lifetime
    int y = h.ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Struct ref in same scope should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 4: Output parameter lifetime violations
// =============================================================================

#[test]
fn test_output_param_to_local() {
    // Function sets output parameter to point to local that dies
    // Note: This test uses raw pointers which require @unsafe
    let source = r#"
// @unsafe
void get_ref(int& x, int*& out) {
    out = &x;
}

// @safe
void bad() {
    int* ptr;
    {
        int x = 42;
        // @unsafe {
        get_ref(x, ptr);  // ptr = &x
        // }
    }  // x dies
    // @unsafe {
    int y = *ptr;  // ERROR: ptr points to dead x
    // }
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect output param pointing to dead local. Output: {}",
        output
    );
}

#[test]
fn test_output_param_same_scope_ok() {
    // Output param and referent in same scope - OK
    // Note: Uses raw pointers which require @unsafe for entire function
    let source = r#"
// @unsafe
void get_ref(int& x, int*& out) {
    out = &x;
}

// @unsafe - using raw pointers, so need unsafe context
void good() {
    int x = 42;
    int* ptr;
    get_ref(x, ptr);
    int y = *ptr;  // OK: x still alive
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Output param in same scope should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 5: Method returns reference to dying object
// =============================================================================

#[test]
fn test_method_ref_to_dying_this() {
    // Method returns reference to field, but object dies
    let source = r#"
// @safe
class Container {
    int value;
public:
    Container(int v) : value(v) {}
    // @lifetime: (&'self) -> &'self
    int& get() { return value; }
};

// @safe
int& bad() {
    Container c(42);
    return c.get();  // ERROR: c dies, returned ref dangles
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect method ref to dying object. Output: {}",
        output
    );
}

#[test]
fn test_method_ref_object_alive_ok() {
    // Object stays alive, reference is valid
    let source = r#"
// @safe
class Container {
    int value;
public:
    Container(int v) : value(v) {}
    // @lifetime: (&'self) -> &'self
    int& get() { return value; }
};

// @safe
void good() {
    Container c(42);
    int& ref = c.get();  // OK: c outlives ref
    ref = 10;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Method ref with alive object should be OK. Got error: {}",
        output
    );
}

#[test]
#[ignore = "TODO: Temporary lifetime tracking in chained calls not yet implemented"]
fn test_chained_method_call_dangling() {
    // Chained call creates temporary that dies
    let source = r#"
// @safe
class Builder {
    int val;
public:
    // @lifetime: (&'self mut) -> &'self mut
    Builder& set(int v) { val = v; return *this; }
    // @lifetime: (&'self) -> &'self
    int& get_value() { return val; }
};

// @safe
void bad() {
    int& ref = Builder().set(42).get_value();  // ERROR: Builder() is temporary
    int y = ref;  // Dangling
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect dangling ref from chained temp. Output: {}",
        output
    );
}

// =============================================================================
// CATEGORY 6: Lifetime annotation violations
// =============================================================================

#[test]
#[ignore = "TODO: Multi-parameter lifetime selection at call site not yet implemented"]
fn test_pick_wrong_lifetime_at_callsite() {
    // Caller uses result as if tied to longer-lived parameter
    let source = r#"
// Returns reference tied to first parameter's lifetime
// @lifetime: (&'a, &'b) -> &'a
const int& pick_first(const int& a, const int& b) { return a; }

// @safe
void bad() {
    const int& ref;
    {
        int x = 1;
        int y = 2;
        ref = pick_first(y, x);  // ref tied to y's lifetime
    }  // y dies (and x)
    int z = ref;  // ERROR: ref is dangling (tied to y which died)
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect lifetime violation at call site. Output: {}",
        output
    );
}

#[test]
fn test_lifetime_annotation_respected_ok() {
    // Correct usage respecting lifetime annotation
    let source = r#"
// @lifetime: (&'a, &'b) -> &'a
const int& pick_first(const int& a, const int& b) { return a; }

// @safe
void good() {
    int x = 1;
    const int& ref;
    {
        int y = 2;
        ref = pick_first(x, y);  // ref tied to x (outlives this scope)
    }
    int z = ref;  // OK: ref tied to x which is still alive
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Correct lifetime usage should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 7: Container/Iterator lifetime issues
// =============================================================================

#[test]
#[ignore = "Iterator invalidation detection not yet implemented - requires semantic analysis of container modifications"]
fn test_vector_iterator_invalidation() {
    // Iterator invalidated by push_back
    let source = r#"
#include <vector>

// @safe
void bad() {
    std::vector<int> v = {1, 2, 3};
    auto it = v.begin();
    v.push_back(4);  // May invalidate iterators
    int x = *it;     // ERROR: iterator may be invalid
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect iterator invalidation. Output: {}",
        output
    );
}

#[test]
#[ignore = "Reference invalidation from container modification not yet implemented - requires semantic analysis of container operations"]
fn test_vector_ref_invalidation() {
    // Reference to element invalidated by push_back
    let source = r#"
#include <vector>

// @safe
void bad() {
    std::vector<int> v = {1, 2, 3};
    int& ref = v[0];
    v.push_back(4);  // May reallocate, invalidating ref
    ref = 10;        // ERROR: ref may be dangling
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect reference invalidation from push_back. Output: {}",
        output
    );
}

#[test]
fn test_vector_ref_no_modification_ok() {
    // Reference valid when no modification happens
    // Note: Uses std::vector which requires @unsafe block since STL is unsafe by default
    let source = r#"
#include <vector>

// @unsafe - STL requires unsafe context
void good() {
    std::vector<int> v = {1, 2, 3};
    int& ref = v[0];
    int x = ref;  // OK: no modification to v
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Vector ref without modification should be OK. Got error: {}",
        output
    );
}

// =============================================================================
// CATEGORY 8: unique_ptr lifetime issues
// =============================================================================

#[test]
fn test_unique_ptr_get_after_move() {
    // Getting raw pointer, then moving unique_ptr
    let source = r#"
#include <memory>

// @safe
void bad() {
    auto ptr = std::make_unique<int>(42);
    int* raw = ptr.get();
    auto ptr2 = std::move(ptr);  // ptr is now null, raw dangles
    int x = *raw;  // ERROR: raw points to moved-from ptr
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect use of raw ptr after unique_ptr move. Output: {}",
        output
    );
}

#[test]
#[ignore = "Reference invalidation after unique_ptr reset not yet implemented - requires tracking of smart pointer state changes"]
fn test_unique_ptr_ref_after_reset() {
    // Reference to unique_ptr contents after reset
    let source = r#"
#include <memory>

// @safe
void bad() {
    auto ptr = std::make_unique<int>(42);
    int& ref = *ptr;
    ptr.reset();  // Deletes the int
    ref = 10;     // ERROR: ref is dangling
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect ref to reset unique_ptr. Output: {}",
        output
    );
}

#[test]
#[ignore = "Dangling pointer from returning ptr.get() not yet implemented - requires tracking of returned pointer lifetime"]
fn test_return_ptr_get_from_local_unique_ptr() {
    // Returning raw pointer from local unique_ptr
    let source = r#"
#include <memory>

// @safe
int* bad() {
    auto ptr = std::make_unique<int>(42);
    return ptr.get();  // ERROR: ptr dies, returned pointer dangles
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect returning ptr.get() from local. Output: {}",
        output
    );
}

// =============================================================================
// CATEGORY 9: Complex scenarios
// =============================================================================

#[test]
fn test_nested_function_lifetime_violation() {
    // Lifetime violation through nested function calls
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& inner(const int& x) { return x; }

// @lifetime: (&'a) -> &'a
// @safe
const int& outer(const int& x) { return inner(x); }

// @safe
void bad() {
    const int& ref = outer(42);  // ERROR: 42 temporary flows through both functions
    int y = ref;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect lifetime violation through nested calls. Output: {}",
        output
    );
}

#[test]
fn test_conditional_lifetime_violation() {
    // One branch creates dangling reference
    let source = r#"
// @lifetime: (&'a, &'b) -> &'a where 'b: 'a
// @safe
const int& choose(bool cond, const int& a, const int& b) {
    return cond ? a : b;
}

// @safe
void bad() {
    int x = 1;
    const int& ref = choose(true, 42, x);  // If true, ref tied to temporary 42
    int y = ref;  // ERROR: may be dangling
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect conditional lifetime violation. Output: {}",
        output
    );
}

#[test]
#[ignore = "TODO: Loop iteration lifetime tracking not yet implemented"]
fn test_loop_accumulates_dangling_refs() {
    // Loop creates references that may dangle
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void bad() {
    const int* refs[10];
    for (int i = 0; i < 10; i++) {
        int temp = i * 2;
        // @unsafe {
        refs[i] = &identity(temp);  // ERROR: temp dies each iteration
        // }
    }
    // @unsafe {
    int x = *refs[0];  // All dangling
    // }
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect loop creating dangling refs. Output: {}",
        output
    );
}
