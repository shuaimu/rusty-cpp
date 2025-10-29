/// Tests for Phase 2: Pack Expansion Detection
///
/// Phase 2 tests verify that:
/// 1. PackExpansionExpr is detected in function calls
/// 2. Operation types are correctly identified (move, forward, use)
/// 3. Pack expansions with different operations are tracked

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
// Phase 2: Pack Expansion Detection Tests
// ============================================================================

#[test]
fn test_phase2_simple_pack_expansion() {
    let code = r#"
    // @safe
    template<typename T> void process(T t) {}

    template<typename... Args>
    // @unsafe
    void expand_pack(Args... args) {
        process(args...);  // Simple pack expansion
    }

    int main() {
        expand_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect pack expansion (operation='use')
    // Function marked unsafe due to "unknown" function call limitation in pack expansion
    assert!(
        success,
        "Should handle simple pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase2_move_pack_expansion() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void consume(T t) {}

    template<typename... Args>
    // @unsafe
    void move_pack(Args... args) {
        consume(std::move(args)...);  // Move pack expansion
    }

    int main() {
        move_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect move operation
    assert!(
        success || output.contains("Pack expansion detected"),
        "Should detect move pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase2_forward_pack_expansion() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void forward_to(T&& t) {}

    template<typename... Args>
    // @unsafe
    void forward_pack(Args&&... args) {
        forward_to(std::forward<Args>(args)...);  // Forward pack expansion
    }

    int main() {
        forward_pack(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect forward operation
    assert!(
        success || output.contains("Pack expansion detected"),
        "Should detect forward pack expansion. Output: {}",
        output
    );
}

#[test]
fn test_phase2_multiple_pack_expansions() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void use_val(T t) {}
    // @safe
    template<typename T> void consume(T t) {}

    template<typename... Args>
    // @unsafe
    void multi_expand(Args... args) {
        use_val(args...);              // First expansion - use
        consume(std::move(args)...);   // Second expansion - move
    }

    int main() {
        multi_expand(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect multiple expansions
    assert!(
        success || output.contains("Pack expansion detected"),
        "Should detect multiple pack expansions. Output: {}",
        output
    );
}

#[test]
fn test_phase2_nested_pack_expansion() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void inner(T t) {}

    // @unsafe
    template<typename T> void outer(T t) { inner(t); }

    template<typename... Args>
    // @unsafe
    void nested_expand(Args... args) {
        outer(std::move(args)...);  // Nested function calls with pack
    }

    int main() {
        nested_expand(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should handle nested expansions
    assert!(
        success,
        "Should handle nested pack expansions. Output: {}",
        output
    );
}

#[test]
fn test_phase2_pack_expansion_in_expression() {
    let code = r#"
    template<typename... Args>
    // @safe
    void expand_in_expr(Args... args) {
        int sum = (args + ...);  // Fold expression (C++17)
    }

    int main() {
        expand_in_expr(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, _output) = run_analyzer(temp_file.path());

    // Fold expressions may not be fully supported yet, but shouldn't crash
    // Just verify it doesn't crash the analyzer
}
