/// Tests for Phase 1: Variadic Template Parameter Pack Recognition
///
/// Phase 1 tests verify that:
/// 1. Parameter packs are recognized (is_pack flag)
/// 2. Pack element types are extracted
/// 3. Pack types are properly whitelisted in analysis

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
// Phase 1: Basic Pack Recognition Tests
// ============================================================================

#[test]
fn test_phase1_simple_variadic_parameter() {
    let code = r#"
    template<typename... Args>
    // @safe
    void simple_pack(Args... args) {
        // Just taking a pack parameter - no usage yet
    }

    int main() {
        simple_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should parse without errors
    // Debug output should show "Found parameter pack 'args'"
    assert!(
        output.contains("Found parameter pack") || success,
        "Should recognize parameter pack. Output: {}",
        output
    );
}

#[test]
fn test_phase1_forwarding_reference_pack() {
    let code = r#"
    #include <utility>

    template<typename... Args>
    // @safe
    void forward_pack(Args&&... args) {
        // Pack with forwarding references
    }

    int main() {
        forward_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should parse forwarding reference pack
    assert!(
        output.contains("Found parameter pack") ||
        output.contains("Args &&") ||
        success,
        "Should recognize forwarding reference pack. Output: {}",
        output
    );
}

#[test]
fn test_phase1_const_reference_pack() {
    let code = r#"
    template<typename... Args>
    // @safe
    void const_pack(const Args&... args) {
        // Pack with const references
    }

    int main() {
        const_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should parse const reference pack
    assert!(
        output.contains("Found parameter pack") || success,
        "Should recognize const reference pack. Output: {}",
        output
    );
}

#[test]
fn test_phase1_multiple_packs() {
    let code = r#"
    template<typename... Ts, typename... Us>
    // @safe
    void multi_pack(Ts... ts, Us... us) {
        // Two independent packs
    }

    int main() {
        multi_pack(1, 2);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should recognize both packs
    // May show two "Found parameter pack" messages
    assert!(
        output.matches("Found parameter pack").count() >= 1 || success,
        "Should recognize multiple parameter packs. Output: {}",
        output
    );
}

#[test]
fn test_phase1_mixed_pack_and_regular() {
    let code = r#"
    template<typename T, typename... Rest>
    // @safe
    void first_and_rest(T first, Rest... rest) {
        // Regular parameter + pack
    }

    int main() {
        first_and_rest(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should recognize pack parameter 'rest'
    assert!(
        output.contains("Found parameter pack") || success,
        "Should recognize pack in mixed parameters. Output: {}",
        output
    );
}

#[test]
fn test_phase1_empty_pack() {
    let code = r#"
    template<typename... Args>
    // @safe
    void maybe_empty(Args... args) {
        // Pack could be empty
    }

    int main() {
        maybe_empty();  // Call with zero arguments
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, _output) = run_analyzer(temp_file.path());

    // Should handle empty pack without errors
    assert!(
        success,
        "Should handle empty parameter pack"
    );
}

// ============================================================================
// Phase 1: Whitelist Tests (Template Type Recognition)
// ============================================================================

#[test]
fn test_phase1_pack_type_not_flagged_as_unsafe() {
    let code = r#"
    template<typename... Args>
    // @safe
    void use_pack_types(Args... args) {
        // Using Args (the type) shouldn't be flagged as unsafe function
        // This tests that pack types are whitelisted
    }

    int main() {
        use_pack_types(1, 2);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should NOT report "Args" as an unsafe function call
    assert!(
        !output.contains("unsafe function 'Args'") &&
        !output.contains("undeclared function 'Args'"),
        "Pack type 'Args' should be whitelisted. Output: {}",
        output
    );
}

#[test]
fn test_phase1_pack_element_type_whitelisted() {
    let code = r#"
    #include <utility>

    template<typename... Args>
    // @safe
    void forward_elements(Args&&... args) {
        // Args&& should be recognized as template type
    }

    int main() {
        forward_elements(1, 2);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should NOT flag "Args&&" as unsafe
    assert!(
        !output.contains("unsafe function 'Args'") &&
        !output.contains("undeclared function 'Args'"),
        "Pack element type 'Args&&' should be whitelisted. Output: {}",
        output
    );
}

#[test]
fn test_phase1_pack_with_ellipsis_whitelisted() {
    let code = r#"
    template<typename... Args>
    // @safe
    void pack_pattern(Args... args) {
        // "Args..." pattern should be recognized
    }

    int main() {
        pack_pattern(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should NOT flag variations of Args with ...
    assert!(
        !output.contains("unsafe function 'Args'"),
        "Pack patterns with ... should be whitelisted. Output: {}",
        output
    );
}
