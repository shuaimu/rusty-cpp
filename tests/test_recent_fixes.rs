/// Integration tests for recent fixes:
/// 1. Commit 442b600: Fix parsing - skip alive check for function call results (temporaries)
/// 2. Commit 403d39f: Support unsafe_type as member variable for safe type

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

fn get_project_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string())
}

fn compile_and_check(source: &str) -> (bool, String) {
    let project_root = get_project_root();
    // Replace relative include paths with absolute paths
    let source_with_abs_path = source
        .replace("#include \"include/rusty/box.hpp\"", &format!("#include \"{}/include/rusty/box.hpp\"", project_root))
        .replace("#include \"include/rusty/option.hpp\"", &format!("#include \"{}/include/rusty/option.hpp\"", project_root))
        .replace("#include \"include/rusty/function.hpp\"", &format!("#include \"{}/include/rusty/function.hpp\"", project_root));

    let temp_file = create_temp_cpp_file(&source_with_abs_path);
    let (success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    let no_violations = output.contains("no violations found");

    (!has_violations || no_violations, output)
}

// =============================================================================
// Tests for commit 442b600: Fix parsing - skip alive check for temporaries
// =============================================================================

#[test]
fn test_operator_star_temporary_no_false_positive() {
    // Test that operator* results (temporaries) don't cause false positive
    // "variable is not alive" errors when borrowing from them
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test_operator_star() {
    auto box1 = rusty::Box<int>::make(42);
    // operator* returns a temporary reference - this should work
    int& ref = *box1;
    int x = ref;  // Using the reference is fine
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_operator_star_temporary_no_false_positive ===");
    println!("{}", output);

    // Should NOT have "not alive" error for operator* result
    assert!(
        !output.contains("not alive"),
        "Should not have 'not alive' error for operator* temporary. Output: {}",
        output
    );
}

#[test]
fn test_qualified_function_call_temporary_no_false_positive() {
    // Test that qualified function calls (namespace::func) returning references
    // don't cause false positive "not alive" errors
    let source = r#"
namespace utils {
    // @unsafe
    int& get_ref(int& x) {
        return x;
    }
}

// @unsafe
void test_qualified_call() {
    int value = 42;
    // Qualified call - should not cause "not alive" for the temporary
    int& ref = utils::get_ref(value);
    int x = ref;
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_qualified_function_call_temporary_no_false_positive ===");
    println!("{}", output);

    // Should NOT have "not alive" error for qualified function call result
    assert!(
        !output.contains("not alive") || !output.contains("utils::get_ref"),
        "Should not have 'not alive' error for qualified function call. Output: {}",
        output
    );
}

// =============================================================================
// Tests for commit 403d39f: Support unsafe_type as member variable for safe type
// =============================================================================

#[test]
fn test_safe_class_with_stl_container_member() {
    // Test that a @safe class can have STL container members without
    // triggering false positives about internal mutable fields
    let source = r#"
#include <unordered_map>
#include <string>

// @safe
class SafeCache {
    // std::unordered_map is an unsafe_type - its internal structure
    // should not be analyzed even though this class is @safe
    std::unordered_map<std::string, int> cache_;

public:
    // @unsafe - STL operations
    void put(const std::string& key, int value) {
        cache_[key] = value;
    }

    // @unsafe - STL operations
    int get(const std::string& key) {
        return cache_[key];
    }
};

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_safe_class_with_stl_container_member ===");
    println!("{}", output);

    // Should NOT have errors about mutable fields in unordered_map internals
    assert!(
        !output.contains("_ReuseOrAllocNode") && !output.contains("mutable field"),
        "Should not analyze internal structure of unsafe_type. Output: {}",
        output
    );
}

#[test]
fn test_safe_class_with_function_member() {
    // Test that a @safe class can have rusty::Function members
    let source = r#"
#include "include/rusty/function.hpp"

// @safe
class CallbackHolder {
    rusty::Function<void()> callback_;

public:
    // @safe
    void set_callback(rusty::Function<void()>&& cb) {
        callback_ = std::move(cb);
    }

    // @safe
    void invoke() {
        if (callback_) {
            callback_();
        }
    }
};

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_safe_class_with_function_member ===");
    println!("{}", output);

    // The Function type is marked @safe, so this should work
    // No internal mutable field errors should occur
    assert!(
        !output.contains("mutable field") || !output.contains("CallbackHolder"),
        "Should not have mutable field errors for safe class with Function member. Output: {}",
        output
    );
}

#[test]
fn test_unsafe_type_annotation_custom_type() {
    // Test that custom types can be marked as unsafe_type
    let source = r#"
// Mark MyInternalContainer as unsafe_type - internal structure should not be analyzed
// @external: {
//   MyInternalContainer: [unsafe_type]
// }

class MyInternalContainer {
    // This has a mutable field, but since it's an unsafe_type,
    // it shouldn't cause errors when used in a @safe class
    mutable int internal_state_;

public:
    void update() const {
        internal_state_++;
    }
};

// @safe
class SafeWrapper {
    MyInternalContainer container_;

public:
    // @unsafe - calls into unsafe_type
    void do_update() {
        container_.update();
    }
};

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_unsafe_type_annotation_custom_type ===");
    println!("{}", output);

    // Should NOT analyze the internal mutable field of MyInternalContainer
    assert!(
        !output.contains("internal_state_") || !output.contains("mutable"),
        "Should not analyze internal structure of custom unsafe_type. Output: {}",
        output
    );
}

// =============================================================================
// Tests for qualified function name parsing (commit 442b600)
// =============================================================================

#[test]
fn test_external_annotation_with_qualified_name() {
    // Test that external annotations work with qualified names (namespace::function)
    let source = r#"
// @external: {
//   rusty::Option::is_none: [unsafe, (&self) -> bool]
//   rusty::Option::is_some: [unsafe, (&self) -> bool]
// }

// Simulate Option-like class for this test
namespace rusty {
    template<typename T>
    class Option {
        bool has_value_;
        T value_;
    public:
        bool is_none() const { return !has_value_; }
        bool is_some() const { return has_value_; }
    };
}

// @unsafe - calling external unsafe functions
void test_qualified_external() {
    rusty::Option<int> opt;
    bool empty = opt.is_none();  // Should match "rusty::Option::is_none" annotation
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_external_annotation_with_qualified_name ===");
    println!("{}", output);

    // Should recognize the qualified function names in annotations
    // No "undeclared" errors for the annotated functions
    assert!(
        !output.contains("undeclared") || !output.contains("is_none"),
        "Should recognize qualified function name in external annotation. Output: {}",
        output
    );
}

// =============================================================================
// Tests for commit a2380da: Fix Option<T> parsing
// =============================================================================

#[test]
fn test_option_safe_annotation_recognized() {
    // Test that Option<T> is properly recognized as @safe when constructing
    // The fix strips template params so "Option<T>" becomes "Option" for lookups
    let source = r#"
#include "include/rusty/option.hpp"

// @safe
void test_option_construction() {
    // These should all work in @safe code because Option<T> is marked @safe
    rusty::Option<int> opt1;  // Default constructor
    auto opt2 = rusty::Some(42);  // Some() helper is @safe
    rusty::Option<int> opt3 = rusty::None;  // None assignment
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_option_safe_annotation_recognized ===");
    println!("{}", output);

    // Should NOT have "undeclared" errors for Option construction in @safe code
    assert!(
        !output.contains("undeclared") || !output.contains("Option"),
        "Option<T> should be recognized as @safe. Output: {}",
        output
    );
}

#[test]
fn test_option_template_param_stripping() {
    // Test that template constructors are properly matched
    // e.g., "Option<T>::Option" should match lookups for "Option"
    let source = r#"
#include "include/rusty/option.hpp"

// @safe
void test_template_constructor() {
    // Constructor call - the header registers "Option<T>" but lookup uses "Option"
    rusty::Option<double> opt(3.14);

    // Method calls on Option should work
    bool has_value = opt.is_some();
    bool no_value = opt.is_none();
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_option_template_param_stripping ===");
    println!("{}", output);

    // Template parameter stripping should allow constructor lookup to work
    assert!(
        !output.contains("undeclared") || !output.contains("Option"),
        "Template constructors should be found after stripping params. Output: {}",
        output
    );
}

#[test]
fn test_none_t_safe() {
    // Test that None_t struct is recognized as @safe
    let source = r#"
#include "include/rusty/option.hpp"

// @safe
void test_none_type() {
    // None_t is a @safe struct, using it should work in @safe code
    rusty::None_t n;
    rusty::Option<int> opt = rusty::None;
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_none_t_safe ===");
    println!("{}", output);

    // None_t should be recognized as @safe
    assert!(
        !output.contains("undeclared") || !output.contains("None"),
        "None_t should be recognized as @safe. Output: {}",
        output
    );
}

#[test]
fn test_some_helper_safe() {
    // Test that Some<T>() helper function is recognized as @safe
    let source = r#"
#include "include/rusty/option.hpp"

// @safe
void test_some_helper() {
    // Some() is a @safe helper function
    auto opt1 = rusty::Some(42);
    auto opt2 = rusty::Some<int>(100);
    auto opt3 = rusty::Some(std::string("hello"));
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    println!("=== test_some_helper_safe ===");
    println!("{}", output);

    // Some() helper should be recognized as @safe
    assert!(
        !output.contains("undeclared") || !output.contains("Some"),
        "Some() helper should be recognized as @safe. Output: {}",
        output
    );
}
