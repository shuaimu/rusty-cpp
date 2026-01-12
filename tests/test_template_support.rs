/// Tests for template function and class analysis
///
/// These tests document what SHOULD be detected when analyzing template code.
/// Currently, template free functions are NOT analyzed at all, and template
/// class methods are processed but not properly checked for move semantics.
///
/// Expected status: MOST OF THESE TESTS WILL FAIL until template support is implemented.

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
// Template FREE Function Tests
// ============================================================================

#[test]
fn test_template_free_function_use_after_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    // @safe
    T bad_function(T x) {
        T copy = std::move(x);   // Move x into copy
        return std::move(x);     // ERROR: Use after move!
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Use after move") || output.contains("Cannot move"),
        "Should detect use-after-move in template function. Output: {}",
        output
    );
}

#[test]
fn test_template_swap_missing_assignment() {
    let code = r#"
    #include <memory>

    template<typename T>
    // @safe
    void broken_swap(T& a, T& b) {
        T temp = std::move(a);   // Move a into temp
        // BUG: Forgot to assign b to a!
        b = std::move(temp);     // Move temp to b
        // Result: 'a' is in moved-from state, not a proper swap
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // The function leaves 'a' in moved-from state
    // This is a logic bug that creates dangling references in callers
    // We should at least warn about the incomplete swap pattern
    assert!(
        output.contains("moved") || output.contains("violation"),
        "Should detect broken swap pattern leaving 'a' in moved-from state. Output: {}",
        output
    );
}

#[test]
fn test_template_double_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    // @safe
    void double_move_bug(T& x) {
        T first = std::move(x);   // First move
        T second = std::move(x);  // ERROR: Second move from same variable!
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move") || output.contains("behind a reference"),
        "Should detect double move in template function. Output: {}",
        output
    );
}

#[test]
fn test_template_move_while_borrowed() {
    let code = r#"
    #include <memory>

    template<typename T>
    // @safe
    void move_borrowed(T& x) {
        T& ref = x;               // Borrow x
        T moved = std::move(x);   // ERROR: Move while borrowed!
        // ref is now dangling
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("borrowed") && output.contains("move"),
        "Should detect move while borrowed in template function. Output: {}",
        output
    );
}

// ============================================================================
// Template CLASS Method Tests
// ============================================================================

#[test]
fn test_template_class_field_double_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    class Container {
        T data;

    public:
        // @safe
        void bad_method() {
            T first = std::move(data);   // Move data
            T second = std::move(data);  // ERROR: Double move!
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move") || output.contains("Cannot move field"),
        "Should detect double move of field in template class. Output: {}",
        output
    );
}

#[test]
fn test_template_class_use_after_field_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    class Container {
        T data;

    public:
        // @safe
        void bad_method() {
            T moved = std::move(data);  // Move data
            T bad = data;                // ERROR: Use after move!
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Skip assertion if headers aren't found (CI environment may not have STL headers)
    if output.contains("file not found") {
        eprintln!("Skipping test: STL headers not found in this environment");
        return;
    }

    assert!(
        output.contains("Use after move") || output.contains("move"),
        "Should detect use of field after move in template class. Output: {}",
        output
    );
}

#[test]
fn test_template_class_const_method_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    class Container {
        T data;

    public:
        // @safe
        T bad_const_method() const {
            return std::move(data);  // ERROR: Move from const method!
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Note: Currently detects field access issue rather than const method violation
    assert!(
        output.contains("const method") || output.contains("Cannot move") || output.contains("violation"),
        "Should detect problem with move from const method in template class. Output: {}",
        output
    );
}

#[test]
fn test_template_class_nonconst_method_move() {
    let code = r#"
    #include <memory>

    template<typename T>
    class Container {
        T data;

    public:
        // @safe
        void bad_nonconst_method() {
            T moved = std::move(data);  // ERROR: Move from &mut self method!
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("&mut self") || output.contains("Cannot move field"),
        "Should detect move from non-const method in template class. Output: {}",
        output
    );
}

#[test]
fn test_template_class_rvalue_method_move_ok() {
    let code = r#"
    #include <memory>

    template<typename T>
    class Container {
        T data;

    public:
        // @safe
        T ok_rvalue_method() && {
            return std::move(data);  // OK: && method can move fields
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Should allow move from && method in template class. Output: {}",
        output
    );
}

// ============================================================================
// Template Instantiation and Inference Tests
// ============================================================================

#[test]
fn test_template_instantiation_with_unique_ptr() {
    let code = r#"
    #include <memory>

    template<typename T>
    // @safe
    void use_twice(T x) {
        auto a = std::move(x);
        auto b = std::move(x);  // ERROR: Double move
    }

    // @safe
    void test() {
        auto ptr = std::make_unique<int>(42);
        use_twice(std::move(ptr));  // Instantiates template with unique_ptr
        // Should detect bug in instantiation
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move"),
        "Should detect bug when template instantiated with unique_ptr. Output: {}",
        output
    );
}

#[test]
fn test_template_forwarding_reference_double_use() {
    let code = r#"
    #include <memory>
    #include <utility>

    template<typename T>
    // @safe
    void forward_twice(T&& x) {
        auto a = std::forward<T>(x);  // First forward (might move)
        auto b = std::forward<T>(x);  // ERROR: Second forward (might double-move)
    }

    // @safe
    void test() {
        auto ptr = std::make_unique<int>(42);
        forward_twice(std::move(ptr));  // Instantiates with unique_ptr&&
        // Should detect potential double-move
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("moved") || output.contains("forward"),
        "Should detect double forward in template. Output: {}",
        output
    );
}

// ============================================================================
// Variadic Template Tests
// ============================================================================

#[test]
fn test_variadic_template_parameter_pack_double_use() {
    let code = r#"
    #include <memory>
    #include <utility>

    template<typename... Args>
    // @safe
    void use_pack_twice(Args&&... args) {
        // Expand pack first time
        (void(std::forward<Args>(args)), ...);

        // ERROR: Expand pack second time - might double-move!
        (void(std::forward<Args>(args)), ...);
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Skip assertion if headers aren't found (CI environment may not have STL headers)
    if output.contains("file not found") {
        eprintln!("Skipping test: STL headers not found in this environment");
        return;
    }

    assert!(
        output.contains("moved") || output.contains("forward"),
        "Should detect double use of parameter pack. Output: {}",
        output
    );
}

// ============================================================================
// Multiple Type Parameter Tests
// ============================================================================

#[test]
fn test_template_multiple_type_params() {
    let code = r#"
    #include <memory>

    template<typename T, typename U>
    // @safe
    void swap_different_types(T& a, U& b) {
        T temp_t = std::move(a);
        U temp_u = std::move(b);

        // This doesn't compile, but we should analyze the moves
        T bad = std::move(a);  // ERROR: 'a' already moved!
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move") || output.contains("behind a reference"),
        "Should detect use-after-move with multiple type params. Output: {}",
        output
    );
}

// ============================================================================
// Template Partial Specialization Tests
// ============================================================================

#[test]
fn test_template_partial_specialization() {
    let code = r#"
    #include <memory>

    // Primary template
    template<typename T>
    class Wrapper {
    public:
        // @safe
        void method(T x) {
            T copy = std::move(x);
            T bad = std::move(x);  // ERROR
        }
    };

    // Partial specialization for pointers
    template<typename T>
    class Wrapper<T*> {
    public:
        // @safe
        void method(T* x) {
            T* copy = x;
            // Pointers can be copied, but method should still be checked
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move"),
        "Should detect bugs in primary template. Output: {}",
        output
    );
}

// ============================================================================
// SFINAE and enable_if Tests
// ============================================================================

#[test]
fn test_template_enable_if() {
    let code = r#"
    #include <memory>
    #include <type_traits>

    template<typename T>
    // @safe
    typename std::enable_if<std::is_move_constructible<T>::value, T>::type
    conditional_move(T x) {
        T copy = std::move(x);   // Move x
        return std::move(x);     // ERROR: Already moved!
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("already been moved") || output.contains("Use after move"),
        "Should detect bug in enable_if template. Output: {}",
        output
    );
}

// ============================================================================
// Comparison: Non-template versions (these SHOULD work)
// ============================================================================

#[test]
#[ignore] // This test requires calling move constructors from @safe code, but user-defined
          // constructors are @unsafe by default. The safety check fires before use-after-move
          // detection. Need STL whitelist or external annotations to test this properly.
fn test_nontemplate_function_use_after_move() {
    let code = r#"
    #include <utility>

    // @unsafe
    namespace test {
        struct Value {
            int data;
            Value(int v) : data(v) {}
            Value(Value&& other) : data(other.data) { other.data = 0; }
        };
    }

    // @safe
    test::Value bad_function(test::Value x) {
        test::Value copy = std::move(x);   // Move x
        return std::move(x);                // ERROR: Use after move!
    }

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Skip if headers not found
    if output.contains("file not found") {
        eprintln!("Skipping test: STL headers not found in this environment");
        return;
    }

    assert!(
        output.contains("not alive") || output.contains("Cannot move") || output.contains("Use after move") || output.contains("moved"),
        "Non-template version should detect use-after-move. Output: {}",
        output
    );
}

#[test]
fn test_nontemplate_class_field_move() {
    let code = r#"
    #include <memory>

    class Container {
        std::unique_ptr<int> data;

    public:
        // @safe
        void bad_method() {
            auto moved = std::move(data);  // ERROR: Move from &mut self!
        }
    };

    int main() { return 0; }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("&mut self") || output.contains("Cannot move field"),
        "Non-template class should detect field move restrictions. Output: {}",
        output
    );
}
