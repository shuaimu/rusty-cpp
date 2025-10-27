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
    // Replace relative include path with absolute path
    let project_root = get_project_root();
    let include_directive = format!("#include \"{}/include/rusty/box.hpp\"", project_root);
    let source_with_abs_path = source.replace("#include \"include/rusty/box.hpp\"", &include_directive);

    let temp_file = create_temp_cpp_file(&source_with_abs_path);
    let (success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    let no_violations = output.contains("no violations found");

    (!has_violations || no_violations, output)
}

#[test]
fn test_reassignment_with_borrow_error() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    int& r = *box1;  // r borrows from box1

    auto box2 = rusty::Box<int>::make(100);
    box1 = std::move(box2);  // ERROR: assignment would drop box1

    int x = r;  // Use r
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(!success, "Expected error but got success");
    assert!(
        output.contains("Cannot assign to 'box1'") && output.contains("borrowed"),
        "Expected assignment-while-borrowed error, got: {}",
        output
    );
}

#[test]
fn test_reassignment_no_borrow_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    auto box2 = rusty::Box<int>::make(100);

    box1 = std::move(box2);  // OK: no borrows

    int x = *box1;  // Use box1
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(
        success,
        "Expected success but got error: {}",
        output
    );
}

#[test]
fn test_reassignment_after_borrow_ends_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);

    {
        int& r = *box1;  // r borrows from box1
        int x = r;       // Use r
    }  // r goes out of scope, borrow ends

    auto box2 = rusty::Box<int>::make(100);
    box1 = std::move(box2);  // OK: borrow has ended

    int y = *box1;
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(
        success,
        "Expected success but got error: {}",
        output
    );
}

#[test]
fn test_move_from_borrowed_error() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    int& r = *box1;  // r borrows from box1

    auto box2 = std::move(box1);  // ERROR: can't move while borrowed

    int x = r;  // Use r
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(!success, "Expected error but got success");
    assert!(
        output.contains("Cannot move 'box1'") && output.contains("borrowed"),
        "Expected move-while-borrowed error, got: {}",
        output
    );
}

#[test]
fn test_operator_star_creates_borrow() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    int& r = *box1;  // operator* should create borrow

    auto box2 = rusty::Box<int>::make(100);
    box1 = std::move(box2);  // ERROR: r borrows from box1

    int x = r;
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(!success, "Expected error but got success");
    assert!(
        output.contains("Cannot assign") || output.contains("borrowed"),
        "Expected borrow-related error, got: {}",
        output
    );
}

#[test]
fn test_reassignment_moves_from_source() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    auto box2 = rusty::Box<int>::make(100);

    box1 = std::move(box2);  // Moves box2

    int x = *box2;  // ERROR: box2 was moved
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(!success, "Expected error but got success");
    assert!(
        output.contains("moved") || output.contains("Use after move"),
        "Expected use-after-move error, got: {}",
        output
    );
}

#[test]
fn test_multiple_reassignments_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(1);
    auto box2 = rusty::Box<int>::make(2);
    auto box3 = rusty::Box<int>::make(3);

    box1 = std::move(box2);  // OK
    box1 = std::move(box3);  // OK - no borrows

    int x = *box1;
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(
        success,
        "Expected success but got error: {}",
        output
    );
}

#[test]
fn test_reassignment_with_mutable_borrow_error() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    int& r = *box1;  // Mutable borrow

    auto box2 = rusty::Box<int>::make(100);
    box1 = std::move(box2);  // ERROR: can't assign while borrowed

    r = 50;  // Use r
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);
    assert!(!success, "Expected error but got success");
    assert!(
        output.contains("Cannot assign") && output.contains("borrowed"),
        "Expected assignment-while-borrowed error, got: {}",
        output
    );
}
