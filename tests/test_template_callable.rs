/// Tests for template callable parameters
///
/// Template callable parameters (like F&& write_fn in template<typename F>)
/// should be recognized as safe to call in @safe functions.

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

    let output = cmd.output()
        .expect("Failed to execute analyzer");

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

// ============================================================================
// Tests for template callable parameters
// ============================================================================

#[test]
fn test_template_callable_forwarding_ref() {
    // Template callable with forwarding reference (F&&) should be safe
    let code = r#"
    template<typename F>
    // @safe
    void call_with_value(int value, F&& write_fn) {
        write_fn(value);  // Should be OK: callable parameter
    }

    // @safe
    void safe_callback(int x) {
        int y = x + 1;
    }

    // @safe
    void test_template_callable() {
        call_with_value(42, safe_callback);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Template callable with F&& should be recognized as safe. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_const_ref() {
    // Template callable with const reference should be safe
    let code = r#"
    template<typename F>
    // @safe
    void call_with_const_ref(int value, const F& callback) {
        callback(value);  // Should be OK: callable parameter
    }

    // @safe
    void test() {
        auto lambda = [](int x) { return x + 1; };
        call_with_const_ref(42, lambda);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Template callable with const F& should be recognized as safe. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_by_value() {
    // Template callable passed by value should be safe
    let code = r#"
    template<typename Func>
    // @safe
    void apply(int x, Func f) {
        f(x);  // Should be OK: callable parameter
    }

    // @safe
    void print_value(int v) {
        int result = v * 2;
    }

    // @safe
    void test() {
        apply(10, print_value);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Template callable passed by value should be recognized as safe. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_multiple_callables() {
    // Multiple callable parameters should all be safe
    let code = r#"
    template<typename F, typename G>
    // @safe
    void call_both(int value, F&& first, G&& second) {
        first(value);   // Should be OK
        second(value);  // Should be OK
    }

    // @safe
    void callback1(int x) { }

    // @safe
    void callback2(int x) { }

    // @safe
    void test() {
        call_both(42, callback1, callback2);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Multiple template callable parameters should be recognized as safe. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_with_return() {
    // Template callable that returns a value
    let code = r#"
    template<typename F>
    // @safe
    int transform(int value, F&& transformer) {
        return transformer(value);  // Should be OK
    }

    // @safe
    int double_it(int x) {
        return x * 2;
    }

    // @safe
    void test() {
        int result = transform(21, double_it);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Template callable with return value should work. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_mixed_with_regular_params() {
    // Callable mixed with non-callable parameters
    let code = r#"
    template<typename F>
    // @safe
    void process(int x, double y, F&& handler, const char* name) {
        handler(x);  // Should be OK: callable parameter
        // x, y, name are regular parameters - not callable
    }

    // @safe
    void my_handler(int val) { }

    // @safe
    void test() {
        process(42, 3.14, my_handler, "test");
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Callable mixed with regular params should work. Output: {}",
        output
    );
}

#[test]
fn test_non_callable_still_fails() {
    // Regular unsafe function calls should still fail
    let code = r#"
    void unsafe_function() {
        int* ptr = nullptr;
    }

    // @safe
    void test() {
        unsafe_function();  // Should FAIL: not a callable parameter
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        !success,
        "Non-callable unsafe function should still fail. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_nested_call() {
    // Nested calls with callable parameters
    let code = r#"
    template<typename F>
    // @safe
    void outer(F&& fn) {
        fn(42);
    }

    template<typename G>
    // @safe
    void inner(G&& callback) {
        callback(10);
    }

    // @safe
    void handler(int x) { }

    // @safe
    void test() {
        outer(inner<void(*)(int)>);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Nested template callable should work. Output: {}",
        output
    );
}

#[test]
fn test_template_callable_in_class() {
    // Template callable in class method
    let code = r#"
    class Processor {
    public:
        template<typename F>
        // @safe
        void process(int value, F&& handler) {
            handler(value);  // Should be OK
        }
    };

    // @safe
    void callback(int x) { }

    // @safe
    void test() {
        Processor p;
        p.process(42, callback);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Template callable in class method should work. Output: {}",
        output
    );
}
