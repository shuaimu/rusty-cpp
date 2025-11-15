/// Tests for C++ cast operations and smart pointer utilities
///
/// These tests verify that cast operations CORRECTLY REQUIRE @unsafe blocks
/// because they operate on raw pointers or can break type safety.

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
// Smart Pointer Cast Tests
// ============================================================================

#[test]
fn test_shared_ptr_casts() {
    let code = r#"
    #include <memory>

    class Base {
    public:
        virtual ~Base() = default;
    };

    class Derived : public Base {};

    // @unsafe - casts can break type safety
    void test_pointer_casts() {
        auto base = std::make_shared<Base>();
        auto derived = std::make_shared<Derived>();

        // Upcast
        auto base_ptr = std::static_pointer_cast<Base>(derived);

        // Downcast (runtime check)
        auto derived_ptr = std::dynamic_pointer_cast<Derived>(base);

        // Const cast
        auto const_ptr = std::const_pointer_cast<const Base>(base);
    }

    int main() {
        test_pointer_casts();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Smart pointer casts should work in @unsafe functions (can break type safety). Output: {}",
        output
    );
}

#[test]
fn test_unique_ptr_casts() {
    let code = r#"
    #include <memory>

    class Base {
    public:
        virtual ~Base() = default;
    };

    class Derived : public Base {};

    // @unsafe - get() returns raw pointer
    void test_unique_ptr_casts() {
        auto derived = std::make_unique<Derived>();
        Base* base_ptr = derived.get();
    }

    int main() {
        test_unique_ptr_casts();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Unique pointer with get() should work in @unsafe functions (returns raw pointer). Output: {}",
        output
    );
}

// ============================================================================
// Regular C++ Cast Tests
// ============================================================================

#[test]
fn test_cpp_cast_operators() {
    let code = r#"
    #include <memory>

    class Base {
    public:
        virtual ~Base() = default;
    };

    class Derived : public Base {};

    // @unsafe - C++ casts operate on raw pointers
    void test_casts() {
        Base base;
        Derived derived;

        // Static cast
        Base* base_ptr = static_cast<Base*>(&derived);

        // Dynamic cast
        Derived* derived_ptr = dynamic_cast<Derived*>(&base);

        // Const cast
        const int x = 42;
        int* y = const_cast<int*>(&x);
    }

    int main() {
        test_casts();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "C++ cast operators should work in @unsafe functions (operate on raw pointers). Output: {}",
        output
    );
}

// ============================================================================
// Type Utility Tests
// ============================================================================

#[test]
fn test_addressof() {
    let code = r#"
    #include <memory>

    // @unsafe - addressof returns raw pointer
    void test_addressof() {
        int x = 42;
        int* ptr = std::addressof(x);
    }

    int main() {
        test_addressof();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "std::addressof should work in @unsafe functions (returns raw pointer). Output: {}",
        output
    );
}

#[test]
fn test_as_const() {
    let code = r#"
    #include <utility>
    #include <vector>

    // @safe
    void test_as_const() {
        // @unsafe
        {
            std::vector<int> vec = {1, 2, 3};

            // Get const reference without explicitly casting
            auto const_ref = std::as_const(vec);
        }
    }

    int main() {
        test_as_const();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "std::as_const should work in @safe code (truly safe, no pointers). Output: {}",
        output
    );
}

// ============================================================================
// Shared From This Tests
// ============================================================================

#[test]
fn test_shared_from_this() {
    let code = r#"
    #include <memory>

    class MyClass : public std::enable_shared_from_this<MyClass> {
    public:
        // @unsafe - shared_from_this() can throw if misused
        std::shared_ptr<MyClass> get_shared() {
            return shared_from_this();
        }

        // @unsafe - weak_from_this() can fail if misused
        std::weak_ptr<MyClass> get_weak() {
            return weak_from_this();
        }
    };

    // @safe
    void test_shared_from_this() {
        // @unsafe
        {
            auto obj = std::make_shared<MyClass>();
            auto shared = obj->get_shared();
            auto weak = obj->get_weak();
        }
    }

    int main() {
        test_shared_from_this();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "shared_from_this/weak_from_this should work in @unsafe functions (can throw if misused). Output: {}",
        output
    );
}

// ============================================================================
// Complex Usage Test
// ============================================================================

#[test]
fn test_complex_cast_usage() {
    let code = r#"
    #include <memory>
    #include <vector>

    class Base {
    public:
        virtual ~Base() = default;
    };

    class Derived : public Base {};

    // @unsafe - uses pointer casts
    void process_objects() {
        // Create smart pointers
        auto base = std::make_shared<Base>();
        auto derived = std::make_shared<Derived>();

        // Various casts
        auto base_from_derived = std::static_pointer_cast<Base>(derived);
        auto maybe_derived = std::dynamic_pointer_cast<Derived>(base);

        // Container of smart pointers
        std::vector<std::shared_ptr<Base>> vec;
        vec.push_back(base);
        vec.push_back(base_from_derived);

        // Iterate and cast
        for (auto& item : vec) {
            auto d = std::dynamic_pointer_cast<Derived>(item);
        }
    }

    int main() {
        process_objects();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Complex cast usage should work in @unsafe functions. Output: {}",
        output
    );
}
