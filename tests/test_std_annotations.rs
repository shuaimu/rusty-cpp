/// Tests for std usage in the two-state safety model
///
/// With the two-state model:
/// - All STL functions are @unsafe by default (not analyzed by RustyCpp)
/// - @safe code must use @unsafe blocks to call STL
/// - Users can mark specific external functions as [safe] via external annotations

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
// Tests demonstrating STL requires @unsafe blocks in @safe code
// ============================================================================

#[test]
fn test_stl_without_unsafe_block_fails() {
    // STL operations without @unsafe block should fail in @safe code
    let code = r#"
    #include <vector>

    // @safe
    void use_vector() {
        std::vector<int> vec;  // ERROR: STL is unsafe by default
        vec.push_back(1);
    }

    int main() {
        use_vector();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        !success,
        "STL without @unsafe block should fail in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_stl_with_unsafe_block_succeeds() {
    // STL operations with @unsafe block should succeed
    let code = r#"
    #include <vector>

    // @safe
    void use_vector() {
        // @unsafe
        {
            std::vector<int> vec;
            vec.push_back(1);
            vec.push_back(2);
            int size = vec.size();
        }
    }

    int main() {
        use_vector();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "STL with @unsafe block should succeed. Output: {}",
        output
    );
}

#[test]
fn test_string_with_unsafe_block() {
    // String operations with @unsafe block should succeed
    let code = r#"
    #include <string>

    // @safe
    void use_string() {
        // @unsafe
        {
            std::string s1 = "Hello";
            std::string s2 = " World";
            std::string s3 = s1 + s2;
            s3.append("!");
        }
    }

    int main() {
        use_string();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "String with @unsafe block should succeed. Output: {}",
        output
    );
}

#[test]
fn test_map_with_unsafe_block() {
    // Map operations with @unsafe block should succeed
    let code = r#"
    #include <map>
    #include <string>

    // @safe
    void use_map() {
        // @unsafe
        {
            std::map<int, std::string> m;
            m[1] = "one";
            m[2] = "two";
        }
    }

    int main() {
        use_map();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Map with @unsafe block should succeed. Output: {}",
        output
    );
}

#[test]
fn test_smart_pointers_with_unsafe_block() {
    // Smart pointer operations with @unsafe block should succeed
    let code = r#"
    #include <memory>

    // @safe
    void use_smart_pointers() {
        // @unsafe
        {
            auto ptr1 = std::make_unique<int>(42);
            int value1 = *ptr1;
            auto ptr2 = std::make_shared<int>(100);
            int value2 = *ptr2;
        }
    }

    int main() {
        use_smart_pointers();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Smart pointers with @unsafe block should succeed. Output: {}",
        output
    );
}

#[test]
fn test_algorithms_with_unsafe_block() {
    // Algorithm operations with @unsafe block should succeed
    let code = r#"
    #include <vector>
    #include <algorithm>

    // @safe
    void use_algorithms() {
        // @unsafe
        {
            std::vector<int> vec = {5, 2, 8, 1, 9};
            std::sort(vec.begin(), vec.end());
            auto it = std::find(vec.begin(), vec.end(), 8);
        }
    }

    int main() {
        use_algorithms();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Algorithms with @unsafe block should succeed. Output: {}",
        output
    );
}

#[test]
fn test_cout_with_unsafe_block() {
    // iostream operations with @unsafe block should succeed
    let code = r#"
    #include <iostream>
    #include <string>

    // @safe
    void use_cout() {
        // @unsafe
        {
            std::cout << "Hello" << std::endl;
            std::cout << 42 << std::endl;
        }
    }

    int main() {
        use_cout();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "cout with @unsafe block should succeed. Output: {}",
        output
    );
}

// ============================================================================
// Tests for @unsafe functions (STL allowed directly)
// ============================================================================

#[test]
fn test_unsafe_function_can_use_stl_directly() {
    // @unsafe functions can use STL without @unsafe blocks
    let code = r#"
    #include <vector>
    #include <algorithm>

    // @unsafe
    void use_stl() {
        std::vector<int> vec = {1, 2, 3};
        vec.push_back(4);
        std::sort(vec.begin(), vec.end());
    }

    int main() {
        use_stl();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "@unsafe functions should be able to use STL directly. Output: {}",
        output
    );
}

// ============================================================================
// Tests for external annotations allowing [safe] marking
// ============================================================================

#[test]
fn test_external_safe_annotation_allows_direct_call() {
    // Functions marked [safe] via external annotations can be called from @safe code
    let code = r#"
    // @external: {
    //   my_safe_function: [safe, () -> void]
    // }

    void my_safe_function();

    // @safe
    void caller() {
        my_safe_function();  // OK: marked [safe] in external annotations
    }

    int main() {
        caller();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "External [safe] annotation should allow direct call from @safe code. Output: {}",
        output
    );
}

#[test]
fn test_external_unsafe_annotation_requires_unsafe_block() {
    // Functions marked [unsafe] via external annotations require @unsafe block
    let code = r#"
    // @external: {
    //   my_unsafe_function: [unsafe, () -> void]
    // }

    void my_unsafe_function();

    // @safe
    void caller() {
        my_unsafe_function();  // ERROR: marked [unsafe], needs @unsafe block
    }

    int main() {
        caller();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        !success,
        "External [unsafe] annotation should require @unsafe block. Output: {}",
        output
    );
}

#[test]
fn test_external_unsafe_with_block_succeeds() {
    // Functions marked [unsafe] can be called with @unsafe block
    let code = r#"
    // @external: {
    //   my_unsafe_function: [unsafe, () -> void]
    // }

    void my_unsafe_function();

    // @safe
    void caller() {
        // @unsafe
        {
            my_unsafe_function();  // OK: in @unsafe block
        }
    }

    int main() {
        caller();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "External [unsafe] with @unsafe block should succeed. Output: {}",
        output
    );
}
