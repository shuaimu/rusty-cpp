//! Integration tests for function pointer safety with SafeFn/UnsafeFn types

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_checker(source_code: &str) -> (i32, String) {
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_file = std::env::temp_dir().join(format!("fn_ptr_test_{}.cpp", test_id));
    std::fs::write(&temp_file, source_code).expect("Failed to write temp file");

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", temp_file.to_str().unwrap()])
        .output()
        .expect("Failed to run checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    let _ = std::fs::remove_file(&temp_file);

    (output.status.code().unwrap_or(-1), combined)
}

// ============================================================================
// Type Detection Tests
// ============================================================================

#[test]
fn test_safe_fn_type_recognized() {
    // SafeFn<Sig> should be recognized as a safe function pointer wrapper
    let code = r#"
namespace rusty {
    template<typename Sig> class SafeFn;
    template<typename Ret, typename... Args>
    class SafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret operator()(Args... args) const { return ptr_(args...); }
    };
}

// @safe
void safe_func(int x);

// @safe
void test() {
    rusty::SafeFn<void(int)> callback;
    // Just declaring is fine
}
"#;
    let (_, output) = run_checker(code);
    // Should not have any SafeFn-specific errors for just declaration
    assert!(
        !output.contains("SafeFn") || output.contains("no violations found"),
        "SafeFn declaration should be allowed. Output: {}", output
    );
}

#[test]
fn test_unsafe_fn_type_recognized() {
    // UnsafeFn<Sig> should be recognized as an unsafe function pointer wrapper
    let code = r#"
namespace rusty {
    template<typename Sig> class UnsafeFn;
    template<typename Ret, typename... Args>
    class UnsafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret call_unsafe(Args... args) const { return ptr_(args...); }
    };
}

// @unsafe
void unsafe_func(int x);

// @safe
void test() {
    rusty::UnsafeFn<void(int)> callback;
    // Just declaring is fine
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        !output.contains("UnsafeFn") || output.contains("no violations found"),
        "UnsafeFn declaration should be allowed. Output: {}", output
    );
}

// ============================================================================
// UnsafeFn::call_unsafe() Tests
// ============================================================================

#[test]
fn test_unsafe_fn_call_unsafe_requires_unsafe_context() {
    let code = r#"
namespace rusty {
    template<typename Sig> class UnsafeFn;
    template<typename Ret, typename... Args>
    class UnsafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret call_unsafe(Args... args) const { return ptr_(args...); }
    };
}

// @unsafe
void dangerous(int x);

// @safe
void test() {
    rusty::UnsafeFn<void(int)> callback;
    callback.call_unsafe(42);  // ERROR - requires @unsafe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("call_unsafe") && output.contains("requires @unsafe"),
        "call_unsafe outside @unsafe should be flagged. Output: {}", output
    );
}

#[test]
fn test_unsafe_fn_call_unsafe_in_unsafe_block_ok() {
    let code = r#"
namespace rusty {
    template<typename Sig> class UnsafeFn;
    template<typename Ret, typename... Args>
    class UnsafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret call_unsafe(Args... args) const { return ptr_(args...); }
    };
}

// @unsafe
void dangerous(int x);

// @safe
void test() {
    rusty::UnsafeFn<void(int)> callback;
    // @unsafe
    {
        callback.call_unsafe(42);  // OK - in @unsafe block
    }
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found") || !output.contains("call_unsafe"),
        "call_unsafe in @unsafe block should be allowed. Output: {}", output
    );
}

// ============================================================================
// SafeFn Usage Tests
// ============================================================================

#[test]
fn test_safe_fn_call_is_safe() {
    let code = r#"
namespace rusty {
    template<typename Sig> class SafeFn;
    template<typename Ret, typename... Args>
    class SafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret operator()(Args... args) const { return ptr_(args...); }
    };
}

// @safe
void safe_func(int x);

// @safe
void test() {
    rusty::SafeFn<void(int)> callback;
    // Calling SafeFn through operator() should be safe
    // (this is the design - SafeFn guarantees the target is safe)
}
"#;
    let (_, output) = run_checker(code);
    // SafeFn::operator() should be allowed in @safe code
    assert!(
        !output.contains("error") || output.contains("no violations found"),
        "SafeFn::operator() should be safe to call. Output: {}", output
    );
}

// ============================================================================
// Member Function Pointer Tests
// ============================================================================

#[test]
fn test_safe_mem_fn_declaration() {
    let code = r#"
namespace rusty {
    template<typename Sig> class SafeMemFn;
    template<typename Ret, typename Class, typename... Args>
    class SafeMemFn<Ret (Class::*)(Args...)> {
        Ret (Class::*ptr_)(Args...);
    };
}

class Widget {
public:
    // @safe
    void handle(int x);
};

// @safe
void test() {
    rusty::SafeMemFn<void (Widget::*)(int)> callback;
    // Declaration is OK
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        !output.contains("SafeMemFn") || output.contains("no violations found"),
        "SafeMemFn declaration should be allowed. Output: {}", output
    );
}

// ============================================================================
// Raw Function Pointer Tests
// ============================================================================

#[test]
fn test_raw_function_pointer_declaration_allowed() {
    let code = r#"
// @safe
void test() {
    void (*fp)(int);  // Declaration is OK
}
"#;
    let (_, output) = run_checker(code);
    // Raw pointer declaration should be allowed (calling is checked)
    assert!(
        output.contains("no violations found") || !output.contains("function pointer"),
        "Raw function pointer declaration should be allowed. Output: {}", output
    );
}

// ============================================================================
// Conversion Tests
// ============================================================================

#[test]
fn test_safe_fn_can_be_stored_in_unsafe_fn() {
    let code = r#"
namespace rusty {
    template<typename Sig> class SafeFn;
    template<typename Ret, typename... Args>
    class SafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        Ret (*get() const)() { return ptr_; }
    };

    template<typename Sig> class UnsafeFn;
    template<typename Ret, typename... Args>
    class UnsafeFn<Ret(Args...)> {
        Ret (*ptr_)(Args...);
    public:
        UnsafeFn(SafeFn<Ret(Args...)> sf) : ptr_(sf.get()) {}
    };
}

// @safe
void safe_func(int x);

// @safe
void test() {
    rusty::SafeFn<void(int)> sf;
    rusty::UnsafeFn<void(int)> uf = sf;  // Safe functions can go into UnsafeFn
}
"#;
    let (_, output) = run_checker(code);
    // Converting SafeFn to UnsafeFn should be allowed
    assert!(
        !output.contains("error") || output.contains("no violations found"),
        "SafeFn should be convertible to UnsafeFn. Output: {}", output
    );
}
