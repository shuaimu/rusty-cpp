/// Tests for Phase 5: Template Argument Pack Expansion
///
/// Phase 5 tests verify that:
/// 1. Pack expansion in template arguments is detected (std::tuple<Args...>)
/// 2. Nested pack expansions work (std::tuple<std::tuple<Args>...>)
/// 3. Type modifiers with packs are handled (const Args&...)
/// 4. Multiple independent packs in template arguments work

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
// Phase 5: Template Argument Pack Expansion Tests
// ============================================================================

#[test]
fn test_phase5_tuple_pack_expansion() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class TupleWrapper {
        std::tuple<Args...> data;  // Pack expansion in template arguments
    };

    int main() {
        TupleWrapper<int, double, char> tw;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack expansion in std::tuple
    assert!(
        success,
        "Should detect pack expansion in std::tuple<Args...>. Output: {}",
        output
    );
}

#[test]
fn test_phase5_variant_pack_expansion() {
    let code = r#"
    #include <variant>

    template<typename... Args>
    class VariantWrapper {
        std::variant<Args...> data;  // Pack expansion in std::variant
    };

    int main() {
        VariantWrapper<int, double, char> vw;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack expansion in std::variant
    assert!(
        success,
        "Should detect pack expansion in std::variant<Args...>. Output: {}",
        output
    );
}

#[test]
fn test_phase5_nested_pack_expansion() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class NestedPack {
        std::tuple<std::tuple<Args>...> nested;  // Nested pack expansion
    };

    int main() {
        NestedPack<int, double> np;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect nested pack expansion
    assert!(
        success,
        "Should detect nested pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase5_const_ref_pack() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class ConstRefPack {
        std::tuple<const Args&...> refs;  // Pack with type modifiers
    };

    int main() {
        ConstRefPack<int, double> crp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack expansion with type modifiers
    assert!(
        success,
        "Should detect pack expansion with type modifiers. Output: {}",
        output
    );
}

#[test]
fn test_phase5_rvalue_ref_pack() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class RValueRefPack {
        std::tuple<Args&&...> rval_refs;  // Rvalue reference pack
    };

    int main() {
        RValueRefPack<int, double> rrp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect rvalue reference pack expansion
    assert!(
        success,
        "Should detect rvalue reference pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase5_multiple_independent_packs() {
    let code = r#"
    #include <tuple>

    template<typename... Ts, typename... Us>
    class DualPack {
        std::tuple<Ts...> first;   // First pack
        std::tuple<Us...> second;  // Second pack
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
        "Should handle multiple independent pack expansions. Output: {}",
        output
    );
}

#[test]
fn test_phase5_custom_template_pack() {
    let code = r#"
    template<typename... Ts>
    struct MyContainer { int data; };

    template<typename... Args>
    class CustomPack {
        MyContainer<Args...> container;  // Custom template with pack
    };

    int main() {
        CustomPack<int, double> cp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack expansion in custom templates
    assert!(
        success,
        "Should detect pack expansion in custom templates. Output: {}",
        output
    );
}

#[test]
fn test_phase5_function_return_type_pack() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class ReturnPack {
    public:
        // @safe
        std::tuple<Args...> get_all();  // Return type with pack expansion
    };

    int main() {
        ReturnPack<int, double> rp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should handle pack expansion in return types
    assert!(
        success,
        "Should handle pack expansion in return types. Output: {}",
        output
    );
}

#[test]
fn test_phase5_mixed_pack_and_nonpack_args() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class MixedArgs {
        std::tuple<int, Args..., double> mixed;  // Mixed pack and non-pack
    };

    int main() {
        MixedArgs<char, float> ma;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should handle mixed template arguments
    assert!(
        success,
        "Should handle mixed pack and non-pack template arguments. Output: {}",
        output
    );
}

#[test]
fn test_phase5_pointer_pack() {
    let code = r#"
    #include <tuple>

    template<typename... Args>
    class PointerPack {
        std::tuple<Args*...> pointers;  // Pointer pack
    };

    int main() {
        PointerPack<int, double> pp;
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pointer pack expansion
    assert!(
        success,
        "Should detect pointer pack expansion. Output: {}",
        output
    );
}
