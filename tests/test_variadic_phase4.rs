/// Tests for Phase 4: Pack Semantics
///
/// Phase 4 tests verify that:
/// 1. Pack ownership state is tracked (Owned -> Moved)
/// 2. Use-after-move is detected for packs
/// 3. Different operations (move, forward, use) have correct semantics
/// 4. Safe patterns are allowed

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
// Phase 4: Pack Semantics Tests
// ============================================================================

#[test]
fn test_phase4_use_after_move_detected() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void process(T t) {}
    // @safe
    template<typename T> void use_val(T t) {}

    template<typename... Args>
    // @safe
    void test_use_after_move(Args... args) {
        process(std::move(args)...);  // Move pack
        use_val(args...);              // ERROR: Use after move
    }

    int main() {
        test_use_after_move(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect use-after-move error
    assert!(
        !success && output.contains("Use after move"),
        "Should detect use-after-move for pack. Output: {}",
        output
    );
}

#[test]
fn test_phase4_double_move_detected() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void consume(T t) {}

    template<typename... Args>
    // @safe
    void test_double_move(Args... args) {
        consume(std::move(args)...);  // First move
        consume(std::move(args)...);  // ERROR: Second move
    }

    int main() {
        test_double_move(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect double move error
    assert!(
        !success && output.contains("Use after move"),
        "Should detect double move for pack. Output: {}",
        output
    );
}

#[test]
fn test_phase4_forward_then_use_detected() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void forward_to(Ttemplate<typename T> void forward_to(T&& t) {}template<typename T> void forward_to(T&& t) {} t) {}
    // @safe
    template<typename T> void use_val(T t) {}

    template<typename... Args>
    // @safe
    void test_forward_use(Args&&... args) {
        forward_to(std::forward<Args>(args)...);  // Forward pack
        use_val(args...);                          // ERROR: Use after forward
    }

    int main() {
        test_forward_use(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should detect use-after-forward error
    assert!(
        !success && output.contains("Use after move"),
        "Should detect use-after-forward for pack. Output: {}",
        output
    );
}

#[test]
fn test_phase4_use_then_move_allowed() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void use_val(T t) {}
    // @safe
    template<typename T> void consume(T t) {}

    template<typename... Args>
    // @unsafe
    void test_use_move(Args... args) {
        use_val(args...);              // Use pack (OK)
        consume(std::move(args)...);   // Then move (OK)
    }

    int main() {
        test_use_move(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should allow use-then-move pattern
    assert!(
        success,
        "Should allow use-then-move pattern. Output: {}",
        output
    );
}

#[test]
fn test_phase4_multiple_uses_allowed() {
    let code = r#"
    // @safe
    template<typename T> void use_val(T t) {}

    template<typename... Args>
    // @unsafe
    void test_multiple_uses(Args... args) {
        use_val(args...);  // First use
        use_val(args...);  // Second use (OK - no move)
        use_val(args...);  // Third use (OK)
    }

    int main() {
        test_multiple_uses(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should allow multiple uses without move
    assert!(
        success,
        "Should allow multiple uses of pack. Output: {}",
        output
    );
}

#[test]
fn test_phase4_single_forward_allowed() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void forward_to(T&& t) {}

    template<typename... Args>
    // @unsafe
    void test_single_forward(Args&&... args) {
        forward_to(std::forward<Args>(args)...);  // Single forward (OK)
    }

    int main() {
        test_single_forward(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should allow single forward
    assert!(
        success,
        "Should allow single forward of pack. Output: {}",
        output
    );
}

#[test]
fn test_phase4_unsafe_does_not_bypass_move_checking() {
    // With the new design, @unsafe only allows pointer operations, not move rule violations.
    // Use-after-move is detected even in @unsafe functions, matching Rust's behavior.
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void process(T t) {}
    // @safe
    template<typename T> void use_val(T t) {}

    template<typename... Args>
    // @unsafe
    void test_unsafe_use(Args... args) {
        process(std::move(args)...);  // Move
        use_val(args...);              // Use after move - now detected even in @unsafe
    }

    int main() {
        test_unsafe_use(1, 2, 3);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Use-after-move is now detected even in @unsafe code
    assert!(
        !success || output.contains("moved"),
        "Use-after-move should be detected even in @unsafe. Output: {}",
        output
    );
}

#[test]
fn test_phase4_error_message_includes_pack_name() {
    let code = r#"
    #include <utility>

    // @safe
    template<typename T> void consume(T t) {}
    // @safe
    template<typename T> void use_val(T t) {}

    template<typename... Args>
    // @safe
    void test_error_message(Args... args) {
        consume(std::move(args)...);
        use_val(args...);  // Error should mention 'args'
    }

    int main() {
        test_error_message(1, 2);
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Error message should include pack name
    assert!(
        output.contains("'args'") || output.contains("\"args\""),
        "Error message should include pack name 'args'. Output: {}",
        output
    );
}
