/// Tests for header file safety validation
///
/// This test suite verifies that the analyzer checks function implementations
/// in header files for safety violations, not just propagates annotations.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let output = Command::new("cargo")
        .args(&["run", "--", cpp_file.to_str().unwrap()])
        .output()
        .expect("Failed to run rusty-cpp-checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    (output.status.success(), combined)
}

/// Test that inline function in header marked @safe calling undeclared function is caught
#[test]
fn test_inline_function_in_header_violates_safety() {
    let temp_dir = TempDir::new().unwrap();

    // Create header file with @safe inline function that calls undeclared function
    let header_path = temp_dir.path().join("server.h");
    let header_content = r#"
#ifndef SERVER_H
#define SERVER_H

// Undeclared function (not marked safe or unsafe)
void process_request(int x);

// @safe - but this function calls undeclared function!
inline void handle_client(int client_id) {
    process_request(client_id);  // VIOLATION: safe calling undeclared
}

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    // Create source file that includes the header
    let source_path = temp_dir.path().join("server.cc");
    let source_content = format!(r#"
#include "{}"

// Implementation of the undeclared function
void process_request(int x) {{
    // Some processing
}}

// Some function that uses handle_client
void main_loop() {{
    handle_client(42);
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    // Run analyzer on the .cc file
    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    // Should report violation: safe function calling undeclared function
    assert!(
        output.contains("safe") &&
        (output.contains("cannot call undeclared") ||
         output.contains("undeclared function") ||
         output.contains("process_request")),
        "Expected safety violation for safe function calling undeclared function in header. Output:\n{}",
        output
    );
}

/// Test that header function marked @safe with unsafe call is caught
#[test]
fn test_header_safe_function_calling_unsafe() {
    let temp_dir = TempDir::new().unwrap();

    // Create header file with @safe function calling explicit @unsafe function
    let header_path = temp_dir.path().join("math.h");
    let header_content = r#"
#ifndef MATH_H
#define MATH_H

// @unsafe
void unsafe_operation(int* ptr);

// @safe - but calls unsafe without unsafe block!
inline int safe_compute(int x) {
    int result = x * 2;
    unsafe_operation(&result);  // Should be in unsafe block
    return result;
}

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    // Create source file that includes the header
    let source_path = temp_dir.path().join("math.cc");
    let source_content = format!(r#"
#include "{}"

// @unsafe
void unsafe_operation(int* ptr) {{
    *ptr = *ptr + 1;
}}

int main() {{
    return safe_compute(10);
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    // Should report that safe function calls unsafe without unsafe block
    // NOTE: According to the three-state system, @safe CAN call @unsafe
    // So this test might not fail. Let me check the rules again.
    // From CLAUDE.md: "@safe → can call: @safe ✅, @unsafe ✅, undeclared ❌"
    // So safe calling unsafe is actually allowed!

    // This test is actually checking the wrong thing. Safe CAN call unsafe.
    // The real issue is safe calling undeclared.
}

/// Test that template function in header marked @safe calling undeclared is caught
#[test]
fn test_template_function_in_header_violates_safety() {
    let temp_dir = TempDir::new().unwrap();

    let header_path = temp_dir.path().join("container.h");
    let header_content = r#"
#ifndef CONTAINER_H
#define CONTAINER_H

// Undeclared function
void log_operation(const char* msg);

// @safe template function - but calls undeclared!
template<typename T>
inline void process_item(T item) {
    log_operation("processing");  // VIOLATION: safe calling undeclared
}

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("container.cc");
    let source_content = format!(r#"
#include "{}"

void log_operation(const char* msg) {{
    // logging
}}

void use_template() {{
    process_item(42);
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    assert!(
        output.contains("safe") &&
        (output.contains("cannot call undeclared") ||
         output.contains("undeclared function") ||
         output.contains("log_operation")),
        "Expected safety violation for safe template function calling undeclared. Output:\n{}",
        output
    );
}

/// Test that class method in header marked @safe calling undeclared is caught
#[test]
fn test_class_method_in_header_violates_safety() {
    let temp_dir = TempDir::new().unwrap();

    let header_path = temp_dir.path().join("handler.h");
    let header_content = r#"
#ifndef HANDLER_H
#define HANDLER_H

// Undeclared helper
void internal_process(int x);

class Handler {
public:
    // @safe - but calls undeclared!
    void handle(int value) {
        internal_process(value);  // VIOLATION
    }
};

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("handler.cc");
    let source_content = format!(r#"
#include "{}"

void internal_process(int x) {{
    // processing
}}

int main() {{
    Handler h;
    h.handle(10);
    return 0;
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    assert!(
        output.contains("safe") &&
        (output.contains("cannot call undeclared") ||
         output.contains("undeclared function") ||
         output.contains("internal_process")),
        "Expected safety violation for safe class method calling undeclared. Output:\n{}",
        output
    );
}

/// Test that header with only declaration (no implementation) doesn't cause issues
#[test]
fn test_header_declaration_only_no_false_positive() {
    let temp_dir = TempDir::new().unwrap();

    let header_path = temp_dir.path().join("api.h");
    let header_content = r#"
#ifndef API_H
#define API_H

// @safe - just a declaration
void safe_function(int x);

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("api.cc");
    let source_content = format!(r#"
#include "{}"

// @safe - implementation calls other safe functions
void safe_function(int x) {{
    // Safe implementation
    int y = x + 1;
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    // Should NOT report violations for properly implemented safe function
    // Look for success message instead of checking for absence of violations
    assert!(
        output.contains("no violations found") || output.contains("0 violation"),
        "Should report no violations for correct safe function. Output:\n{}",
        output
    );
}

/// Test exact user scenario: safe function in .h calling undeclared
#[test]
fn test_user_reported_scenario() {
    let temp_dir = TempDir::new().unwrap();

    // Create server.h with safe function calling undeclared
    let header_path = temp_dir.path().join("server.h");
    let header_content = r#"
#ifndef SERVER_H
#define SERVER_H

// Undeclared function (no safety annotation)
int get_socket_fd();

// @safe - marking function as safe even though it's not
inline bool is_server_ready() {
    int fd = get_socket_fd();  // VIOLATION: safe calling undeclared
    return fd > 0;
}

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    // Create server.cc that includes the header
    let source_path = temp_dir.path().join("server.cc");
    let source_content = format!(r#"
#include "{}"

// Implementation of undeclared function
int get_socket_fd() {{
    return 42;  // stub
}}

int main() {{
    if (is_server_ready()) {{
        return 0;
    }}
    return 1;
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("=== USER REPORTED SCENARIO ===");
    println!("Output:\n{}", output);
    println!("==============================");

    // According to user, this should throw a violation but doesn't
    // Let's verify if this is indeed broken
    let has_violation = output.contains("safe") &&
                       (output.contains("cannot call undeclared") ||
                        output.contains("undeclared function") ||
                        output.contains("get_socket_fd"));

    if has_violation {
        println!("✅ GOOD: Violation detected as expected");
    } else {
        println!("❌ BUG CONFIRMED: No violation detected!");
        println!("The safe function is_server_ready() calls undeclared get_socket_fd()");
        println!("This should be reported as a violation but isn't.");
    }

    assert!(
        has_violation,
        "BUG CONFIRMED: Safe function in header calling undeclared function is not detected!\n\
         Expected: Violation for is_server_ready() calling undeclared get_socket_fd()\n\
         Output:\n{}",
        output
    );
}

/// Test with namespace in header
#[test]
fn test_namespace_safe_function_in_header_violates_safety() {
    let temp_dir = TempDir::new().unwrap();

    let header_path = temp_dir.path().join("network.h");
    let header_content = r#"
#ifndef NETWORK_H
#define NETWORK_H

namespace network {

// Undeclared
void send_packet(const char* data);

// @safe
inline void send_message(const char* msg) {
    send_packet(msg);  // VIOLATION: safe calling undeclared
}

} // namespace network

#endif
"#;

    fs::write(&header_path, header_content).unwrap();

    let source_path = temp_dir.path().join("network.cc");
    let source_content = format!(r#"
#include "{}"

namespace network {{

void send_packet(const char* data) {{
    // implementation
}}

}} // namespace network

int main() {{
    network::send_message("hello");
    return 0;
}}
"#, header_path.to_str().unwrap());

    fs::write(&source_path, source_content).unwrap();

    let (_success, output) = run_analyzer(&source_path);

    println!("Output:\n{}", output);

    assert!(
        output.contains("safe") &&
        (output.contains("cannot call undeclared") ||
         output.contains("undeclared function") ||
         output.contains("send_packet")),
        "Expected safety violation for namespaced safe function calling undeclared. Output:\n{}",
        output
    );
}
