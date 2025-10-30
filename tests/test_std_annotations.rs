/// Tests for std_annotation.hpp - verifying that common std functions
/// can be used in @safe code without additional annotations

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
// Tests for common std usage patterns
// ============================================================================

#[test]
fn test_vector_operations_in_safe_code() {
    let code = r#"
    #include <vector>
    #include <algorithm>

    // @safe
    void use_vector() {
        std::vector<int> vec = {1, 2, 3, 4, 5};
        vec.push_back(6);
        vec.pop_back();

        std::sort(vec.begin(), vec.end());

        int size = vec.size();
        bool empty = vec.empty();
    }

    int main() {
        use_vector();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed without requiring @unsafe blocks
    assert!(
        success,
        "Vector operations should work in @safe code with std_annotation.hpp. Output: {}",
        output
    );
}

#[test]
fn test_cout_operations_in_safe_code() {
    let code = r#"
    #include <iostream>
    #include <string>

    // @safe
    void use_cout() {
        std::cout << "Hello" << std::endl;
        std::cout << 42 << std::endl;

        std::string s = "World";
        std::cout << s << std::endl;
    }

    int main() {
        use_cout();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - iostream operations marked safe
    assert!(
        success,
        "cout operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_string_operations_in_safe_code() {
    let code = r#"
    #include <string>

    // @safe
    void use_string() {
        std::string s1 = "Hello";
        std::string s2 = " World";
        std::string s3 = s1 + s2;

        s3.append("!");
        size_t len = s3.length();
        char c = s3[0];
    }

    int main() {
        use_string();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - string operations marked safe
    assert!(
        success,
        "String operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_smart_pointers_in_safe_code() {
    let code = r#"
    #include <memory>

    // @safe
    void use_smart_pointers() {
        auto ptr1 = std::make_unique<int>(42);
        int value1 = *ptr1;

        auto ptr2 = std::make_shared<int>(100);
        int value2 = *ptr2;
    }

    int main() {
        use_smart_pointers();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - smart pointer operations marked safe
    assert!(
        success,
        "Smart pointer operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_algorithms_in_safe_code() {
    let code = r#"
    #include <vector>
    #include <algorithm>
    #include <numeric>

    // @safe
    void use_algorithms() {
        std::vector<int> vec = {5, 2, 8, 1, 9};

        std::sort(vec.begin(), vec.end());

        auto it = std::find(vec.begin(), vec.end(), 8);

        int sum = std::accumulate(vec.begin(), vec.end(), 0);

        std::vector<int> vec2(5);
        std::copy(vec.begin(), vec.end(), vec2.begin());
    }

    int main() {
        use_algorithms();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - algorithm operations marked safe
    assert!(
        success,
        "Algorithm operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_map_operations_in_safe_code() {
    let code = r#"
    #include <map>
    #include <string>

    // @safe
    void use_map() {
        std::map<int, std::string> m;
        m[1] = "one";
        m[2] = "two";

        auto it = m.find(1);

        size_t size = m.size();
        bool empty = m.empty();
    }

    int main() {
        use_map();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - map operations marked safe
    assert!(
        success,
        "Map operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_utility_functions_in_safe_code() {
    let code = r#"
    #include <utility>
    #include <algorithm>

    // @safe
    void use_utilities() {
        int a = 5;
        int b = 10;

        std::swap(a, b);

        int&& r = std::move(a);

        auto p = std::make_pair(1, std::string("one"));
    }

    int main() {
        use_utilities();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - utility functions marked safe
    assert!(
        success,
        "Utility functions should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_optional_in_safe_code() {
    let code = r#"
    #include <optional>

    // @safe
    void use_optional() {
        std::optional<int> opt1 = 42;
        std::optional<int> opt2;

        if (opt1.has_value()) {
            int value = opt1.value();
        }

        int value_or = opt2.value_or(0);
    }

    int main() {
        use_optional();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - optional operations marked safe
    assert!(
        success,
        "Optional operations should work in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_complex_std_usage() {
    let code = r#"
    #include <vector>
    #include <map>
    #include <string>
    #include <algorithm>
    #include <memory>
    #include <iostream>

    // @safe
    void complex_example() {
        // Containers
        std::vector<int> vec = {1, 2, 3};
        std::map<std::string, int> m;
        m["one"] = 1;
        m["two"] = 2;

        // Algorithms
        std::sort(vec.begin(), vec.end());
        auto it = std::find(vec.begin(), vec.end(), 2);

        // Smart pointers
        auto ptr = std::make_unique<std::string>("test");

        // I/O
        std::cout << "Size: " << vec.size() << std::endl;

        // Utilities
        std::swap(vec[0], vec[1]);
    }

    int main() {
        complex_example();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - all operations marked safe
    assert!(
        success,
        "Complex std usage should work in @safe code. Output: {}",
        output
    );
}
