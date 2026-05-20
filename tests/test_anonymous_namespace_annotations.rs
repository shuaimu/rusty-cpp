//! Regression tests for annotation handling in anonymous namespaces.
//!
//! Bug: `extract_namespace_name` returned `None` for `namespace { ... }`,
//! so anonymous namespaces were silently dropped from the parser's
//! context stack. Combined with the brace-tracking pop logic, this
//! could desync the qualified-name builder from libclang's view of the
//! AST (where anonymous namespaces are skipped when forming qualified
//! names).
//!
//! These tests run the full analyzer end-to-end on small C++ files
//! that mirror real-world shapes (matching the rrr/reactor/reactor.cpp
//! tarpit at line 1582+). They assert that `// @safe` / `// @unsafe`
//! annotations placed directly above free functions inside anonymous
//! namespaces are honored by the analyzer.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer_on_file(cpp_file: &std::path::Path) -> (bool, String) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()]);

    if cfg!(target_os = "macos") {
        cmd.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output().expect("Failed to execute analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

#[test]
fn test_unsafe_func_in_anonymous_namespace_can_use_raw_pointer() {
    // A function explicitly marked @unsafe inside an anonymous namespace
    // should be allowed to perform raw pointer operations even when the
    // surrounding named namespace is @safe.
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
// @safe
namespace outer {

namespace {

// @unsafe
inline void helper() {
    int* p = nullptr;
    *p = 1;  // raw pointer deref — only allowed because @unsafe overrides
}

}  // anonymous

void caller() {
    // @unsafe
    {
        helper();
    }
}

}  // namespace outer
"#;

    let cpp_path = temp_dir.path().join("anon_ns_unsafe_helper.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (success, output) = run_analyzer_on_file(&cpp_path);

    assert!(
        success,
        "analyzer must accept code where @unsafe override on a function inside\n\
         an anonymous namespace permits raw-pointer operations.\n\n\
         Analyzer output:\n{}",
        output
    );
    // The analyzer prints "no violations found!" on success — check for
    // the failure marker explicitly instead of the substring "violation".
    assert!(
        output.contains("no violations found"),
        "no violations expected.\nOutput:\n{}",
        output
    );
    assert!(
        !output.contains("Found ") || !output.contains("violation(s)"),
        "no violations expected.\nOutput:\n{}",
        output
    );
}

#[test]
fn test_safe_func_in_anonymous_namespace_inherits_outer_safety() {
    // A function with no explicit annotation inside an anonymous namespace
    // nested in a @safe namespace should inherit @safe — and consequently
    // raw pointer operations inside it must trigger a violation.
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
// @safe
namespace outer {

namespace {

inline void helper() {
    int* p = nullptr;
    *p = 1;  // raw pointer deref — should fail in @safe code
}

}  // anonymous

}  // namespace outer
"#;

    let cpp_path = temp_dir.path().join("anon_ns_safe_inherits.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (_success, output) = run_analyzer_on_file(&cpp_path);

    // The analyzer should report violations. Look for the explicit failure
    // marker ("Found N violation(s)") rather than the substring "violation"
    // because the success path also prints "no violations found".
    let reported_violations = output.contains("Found ") && output.contains("violation(s)");
    assert!(
        reported_violations,
        "analyzer must reject raw pointer ops in a function that inherits \
         @safe from the outer namespace through an anonymous namespace.\n\n\
         Analyzer output:\n{}",
        output
    );
}
