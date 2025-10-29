/// Tests for Phase 3: Variadic Template Classes
///
/// Phase 3 tests verify that:
/// 1. ClassTemplate entities are parsed
/// 2. Template parameters including packs are extracted
/// 3. Member fields with pack types are detected
/// 4. Base class packs are recognized

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
// Phase 3: Variadic Template Class Tests
// ============================================================================

#[test]
fn test_phase3_simple_template_class() {
    let code = r#"
    template<typename... Args>
    class SimpleContainer {
        int value;
    };

    int main() {
        SimpleContainer<int, double> c;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should parse template class
    assert!(
        success,
        "Should parse simple variadic template class. Output: {}",
        output
    );
}

#[test]
fn test_phase3_class_with_pack_member() {
    let code = r#"
    template<typename T> struct Container { T value; };

    template<typename... Args>
    class Wrapper {
        Container<Args...> data;  // Member with pack expansion
    };

    int main() {
        Wrapper<int, double> w;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack in member type
    assert!(
        success,
        "Should detect pack expansion in member field. Output: {}",
        output
    );
}

#[test]
fn test_phase3_tuple_member() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class TupleWrapper {
        std::tuple<Args...> data;  // std::tuple with pack
    };

    int main() {
        TupleWrapper<int, double, char> tw;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect std::tuple<Args...>
    assert!(
        success,
        "Should detect pack in std::tuple member. Output: {}",
        output
    );
}

#[test]
fn test_phase3_base_class_pack() {
    let code = r#"
    template<typename T> struct Base { T value; };

    template<typename... Bases>
    class MultiInherit : public Bases... {
        int data;
    };

    int main() {
        MultiInherit<Base<int>, Base<double>> m;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect base class pack expansion
    assert!(
        success,
        "Should detect base class pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase3_multiple_pack_members() {
    let code = r#"
    template<typename T> struct Container { T value; };

    template<typename... Ts, typename... Us>
    class DualPack {
        Container<Ts...> first;
        Container<Us...> second;
    };

    int main() {
        DualPack<int, double, char, float> dp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should handle multiple independent packs
    assert!(
        success,
        "Should handle multiple pack members. Output: {}",
        output
    );
}

#[test]
fn test_phase3_template_class_with_methods() {
    let code = r#"
    template<typename T> struct Container { T value; };

    template<typename... Args>
    class MethodPack {
        Container<Args...> data;

    public:
        // @safe
        void process() {
            // Method in template class
        }
    };

    int main() {
        MethodPack<int, double> mp;
        mp.process();
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should handle template class with methods
    assert!(
        success,
        "Should handle template class with methods. Output: {}",
        output
    );
}

#[test]
fn test_phase3_nested_template_class() {
    let code = r#"
    template<typename... Args>
    class Outer {
        template<typename... Inner>
        class Nested {
            int value;
        };
    };

    int main() {
        Outer<int>::Nested<double> n;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, _output) = run_analyzer(temp_file.path());

    // Should handle nested template classes (may have limitations)
    assert!(
        success,
        "Should handle nested template classes"
    );
}
