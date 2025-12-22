/// Tests for namespace disambiguation in safety annotations
/// These tests verify that functions with the same name in different namespaces
/// are correctly tracked and matched with their safety annotations.

use std::process::Command;
use tempfile::TempDir;

fn run_analyzer_with_include(source_path: &std::path::Path, include_path: &std::path::Path) -> (bool, String) {
    let include_arg = format!("-I{}", include_path.display());

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", source_path.to_str().unwrap(), &include_arg]);

    if cfg!(target_os = "macos") {
        cmd.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output().expect("Failed to execute analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

// ============================================================================
// Test 1: Same function name in two namespaces, one @safe one @unsafe
// Calling unqualified from INSIDE the namespace should match correctly
// ============================================================================
#[test]
fn test_same_name_different_namespaces_inside_call() {
    let temp_dir = TempDir::new().unwrap();

    // Header with two namespaces, same function name, different safety
    let header_content = r#"
#pragma once

namespace ns_safe {
    // @safe
    int process() { return 42; }
}

namespace ns_unsafe {
    // @unsafe
    int process() { return 0; }
}
"#;

    // Source: call from INSIDE each namespace (unqualified)
    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

namespace ns_safe {
    // @safe
    void caller_in_safe_ns() {
        int x = process();  // Should resolve to ns_safe::process (@safe) - OK
    }
}

namespace ns_unsafe {
    // @safe
    void caller_in_unsafe_ns() {
        // @unsafe
        {
            int x = process();  // Should resolve to ns_unsafe::process (@unsafe) - OK with @unsafe block
        }
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Unqualified calls inside namespaces should resolve correctly. Output: {}",
        output
    );
    assert!(
        !output.contains("undeclared"),
        "Functions should not be undeclared. Output: {}",
        output
    );
}

// ============================================================================
// Test 2: Same function name, one is @safe one is @unsafe (default)
// @safe function calling @unsafe should fail without @unsafe block
// ============================================================================
#[test]
fn test_same_name_one_unsafe() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace annotated {
    // @safe
    int helper() { return 1; }
}

namespace not_annotated {
    // No annotation - @unsafe by default in two-state model
    int helper() { return 2; }
}
"#;

    // @safe function calling @unsafe version should error without @unsafe block
    let source_content = r#"
#include "test.h"

namespace not_annotated {
    // @safe
    void safe_caller() {
        int x = helper();  // Calls not_annotated::helper which is @unsafe - ERROR
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    // This should FAIL because @safe is calling @unsafe without @unsafe block
    assert!(
        !success || output.contains("non-safe") || output.contains("@unsafe"),
        "Should detect @safe calling @unsafe function. Output: {}",
        output
    );
}

// ============================================================================
// Test 3: Nested namespaces with same leaf name
// ============================================================================
#[test]
fn test_nested_namespaces_same_leaf_name() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace outer {
    namespace inner {
        // @safe
        void do_work() {}
    }
}

namespace other {
    namespace inner {
        // @unsafe
        void do_work() {}
    }
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

namespace outer {
    namespace inner {
        // @safe
        void caller() {
            do_work();  // Should call outer::inner::do_work (@safe) - OK
        }
    }
}

namespace other {
    namespace inner {
        // @safe
        void caller() {
            // @unsafe
            {
                do_work();  // Should call other::inner::do_work (@unsafe) - OK with @unsafe block
            }
        }
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Nested namespace calls should resolve correctly. Output: {}",
        output
    );
}

// ============================================================================
// Test 4: Class methods with same name in different namespaces
// ============================================================================
#[test]
fn test_class_methods_same_name_different_namespaces() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace lib_a {
    class Widget {
    public:
        // @safe
        void update() {}
    };
}

namespace lib_b {
    class Widget {
    public:
        // @unsafe
        void update() {}
    };
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

// @safe
void test_lib_a() {
    lib_a::Widget w;
    w.update();  // Calls lib_a::Widget::update (@safe) - OK
}

// @safe
void test_lib_b() {
    lib_b::Widget w;
    // @unsafe
    {
        w.update();  // Calls lib_b::Widget::update (@unsafe) - OK with @unsafe block
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Class methods in different namespaces should be distinguished. Output: {}",
        output
    );
}

// ============================================================================
// Test 5: Template functions with same name in different namespaces
// ============================================================================
#[test]
fn test_template_functions_same_name_different_namespaces() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace factory_a {
    // @safe
    template<typename T>
    T create() { T val; return val; }
}

namespace factory_b {
    // @unsafe
    template<typename T>
    T create() { T val; return val; }
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

namespace factory_a {
    // @safe
    void use_create() {
        int x = create<int>();  // Calls factory_a::create (@safe) - OK
    }
}

namespace factory_b {
    // @safe
    void use_create() {
        // @unsafe
        {
            int x = create<int>();  // Calls factory_b::create (@unsafe) - OK with @unsafe block
        }
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Template functions in different namespaces should be distinguished. Output: {}",
        output
    );
}

// ============================================================================
// Test 6: Calling fully qualified vs unqualified - both should work
// ============================================================================
#[test]
fn test_qualified_vs_unqualified_calls() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace mylib {
    // @unsafe
    void dangerous_op() {}
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

namespace mylib {
    // @safe
    void caller_unqualified() {
        // @unsafe
        {
            dangerous_op();  // Unqualified - should resolve to mylib::dangerous_op
        }
    }

    // @safe
    void caller_qualified() {
        // @unsafe
        {
            mylib::dangerous_op();  // Fully qualified - explicit
        }
    }
}

// @safe
void external_caller() {
    // @unsafe
    {
        mylib::dangerous_op();  // Must be qualified from outside
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Both qualified and unqualified calls should work. Output: {}",
        output
    );
}

// ============================================================================
// Test 7: Three-level namespace hierarchy
// ============================================================================
#[test]
fn test_three_level_namespace() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace level1 {
    namespace level2 {
        namespace level3 {
            // @safe
            int deep_func() { return 42; }
        }
    }
}
"#;

    let source_content = r#"
#include "test.h"

namespace level1 {
    namespace level2 {
        namespace level3 {
            // @safe
            void caller() {
                int x = deep_func();  // Unqualified call to level1::level2::level3::deep_func
            }
        }
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Deep nested namespace calls should resolve correctly. Output: {}",
        output
    );
}

// ============================================================================
// Test 8: rusty vs external library (simulating bug #8 scenario)
// With two-state model: unannotated code is @unsafe by default
// ============================================================================
#[test]
fn test_rusty_vs_external_library_overlap() {
    let temp_dir = TempDir::new().unwrap();

    // Simulating rusty's Option::get
    let rusty_header = r#"
#pragma once

namespace rusty {
    template<typename T>
    class Option {
    public:
        // @safe
        T get() { return T{}; }
    };
}
"#;

    // Simulating external library with same method name
    let external_header = r#"
#pragma once

namespace external_lib {
    class Config {
    public:
        // No annotation - @unsafe by default in two-state model
        int get() { return 0; }
    };
}
"#;

    // Using both - external_lib::get should NOT match rusty::Option::get
    // This function has no annotation - it's @unsafe by default
    // @unsafe functions can call both @safe and @unsafe functions
    let source_content = r#"
#include "rusty.h"
#include "external.h"

// No annotation - @unsafe by default (can call anything)
void use_both() {
    rusty::Option<int> opt;
    int a = opt.get();  // rusty::Option::get (@safe) - OK

    external_lib::Config cfg;
    int b = cfg.get();  // external_lib::Config::get (@unsafe) - OK
}
"#;

    let rusty_path = temp_dir.path().join("rusty.h");
    std::fs::write(&rusty_path, rusty_header).unwrap();

    let external_path = temp_dir.path().join("external.h");
    std::fs::write(&external_path, external_header).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    // Should pass because use_both() is @unsafe (can call anything)
    assert!(
        success,
        "@unsafe function calling mixed annotations should pass. Output: {}",
        output
    );
}

// ============================================================================
// Test 9: @safe function calling external @unsafe should FAIL without @unsafe block
// With two-state model: unannotated = @unsafe, so @safe cannot call it directly
// ============================================================================
#[test]
fn test_safe_calling_external_unsafe_fails() {
    let temp_dir = TempDir::new().unwrap();

    let rusty_header = r#"
#pragma once

namespace rusty {
    template<typename T>
    class Option {
    public:
        // @safe
        T get() { return T{}; }
    };
}
"#;

    let external_header = r#"
#pragma once

namespace external_lib {
    class Config {
    public:
        // No annotation - @unsafe by default in two-state model
        int get() { return 0; }
    };
}
"#;

    // @safe function calling @unsafe external should fail without @unsafe block
    let source_content = r#"
#include "rusty.h"
#include "external.h"

// @safe
void safe_caller() {
    external_lib::Config cfg;
    int x = cfg.get();  // Calling @unsafe from @safe without @unsafe block - ERROR!
}
"#;

    let rusty_path = temp_dir.path().join("rusty.h");
    std::fs::write(&rusty_path, rusty_header).unwrap();

    let external_path = temp_dir.path().join("external.h");
    std::fs::write(&external_path, external_header).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    // Should FAIL because @safe is calling @unsafe external without @unsafe block
    assert!(
        !success || output.contains("non-safe") || output.contains("@unsafe"),
        "@safe calling @unsafe external without @unsafe block should fail. Output: {}",
        output
    );
}

// ============================================================================
// Test 10: Overloaded functions with same name (different signatures)
// NOTE: Overload resolution for safety annotations is not currently supported.
// When multiple overloads exist with different safety levels, all calls to that
// function name may be treated as needing @unsafe blocks.
// This test uses @unsafe blocks for ALL calls to work around this limitation.
// ============================================================================
#[test]
fn test_overloaded_functions_same_namespace() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace mylib {
    // @safe
    void process_int(int x) {}

    // @unsafe
    void process_double(double x) {}
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    // Using different function names since overload resolution for safety isn't supported
    let source_content = r#"
#include "test.h"

namespace mylib {
    // @safe
    void caller() {
        process_int(42);      // Calls process_int - @safe - OK
        // @unsafe
        {
            process_double(3.14);    // Calls process_double - @unsafe - OK with @unsafe block
        }
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    // Both should work - safe calls safe directly, safe calls unsafe via @unsafe block
    assert!(
        success,
        "Differently named functions should be distinguished. Output: {}",
        output
    );
}

// ============================================================================
// Test 11: Static class methods with same name
// ============================================================================
#[test]
fn test_static_methods_same_name_different_classes() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#pragma once

namespace factory {
    class SafeFactory {
    public:
        // @safe
        static int create() { return 1; }
    };

    class UnsafeFactory {
    public:
        // @unsafe
        static int create() { return 2; }
    };
}
"#;

    // With two-state model: @safe can ONLY call @safe, must use @unsafe block for unsafe calls
    let source_content = r#"
#include "test.h"

// @safe
void use_factories() {
    int a = factory::SafeFactory::create();    // @safe
    // @unsafe
    {
        int b = factory::UnsafeFactory::create();  // @unsafe - OK with @unsafe block
    }
}
"#;

    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    let (success, output) = run_analyzer_with_include(&source_path, temp_dir.path());

    assert!(
        success,
        "Static methods with same name in different classes should be distinguished. Output: {}",
        output
    );
}
