//! Regression guards for nested-type safety in `// @safe` classes.
//!
//! Investigation note: a prior diagnosis claimed that the analyzer
//! propagated a class's `// @safe` annotation into structs declared
//! inside method bodies, then tripped the "mutable field not allowed
//! in safe class" rule on their `mutable std::atomic<…>` fields
//! (the reactor.cpp tarpit shape — `Reactor::spawn_stackless_task`
//! defines `struct EarlyWakeState` / `struct TaskState` locally).
//!
//! These tests confirm the diagnosis does NOT reproduce: the analyzer
//! qualifies nested types as `Outer::Nested` but does not inherit the
//! `// @safe` annotation from `Outer`, so the mutable-field rule
//! correctly stays silent. The tests guard against future regressions
//! that would either (a) make the analyzer propagate class-@safe to
//! local-method-body structs (which would be wrong — they have local
//! visibility) or (b) make it stop the current correct behavior of
//! treating nested types as @unsafe by default.

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
fn test_local_struct_with_mutable_field_in_safe_method() {
    // The reactor.cpp tarpit shape. The `// @safe` class annotation
    // should not propagate to a struct declared inside one of its
    // method bodies. Local-method-body types have visibility scoped
    // to the function, not the enclosing class.
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
#include <atomic>

// @safe
class Outer {
public:
    void method() const {
        struct LocalState {
            mutable std::atomic<int> counter{0};
        };
    }
};
"#;

    let cpp_path = temp_dir.path().join("local_struct_mutable.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (success, output) = run_analyzer_on_file(&cpp_path);

    assert!(
        success,
        "analyzer must accept a local struct with mutable fields declared\n\
         inside a @safe method. The enclosing class's safety must NOT\n\
         propagate to types whose visibility ends with the function scope.\n\n\
         Analyzer output:\n{}",
        output
    );
    assert!(
        output.contains("no violations found"),
        "no violations expected.\nOutput:\n{}",
        output
    );
    assert!(
        !(output.contains("Found ") && output.contains("violation(s)")),
        "no violations expected.\nOutput:\n{}",
        output
    );
    // Specifically: the "mutable field" rule must not fire on LocalState.
    assert!(
        !output.contains("Mutable field"),
        "mutable-field rule must not fire on local struct in @safe method.\nOutput:\n{}",
        output
    );
}

#[test]
fn test_class_scope_nested_struct_with_mutable_field() {
    // Class-scope nested structs (declared inside the class body, NOT
    // inside a method body) are also not flagged today. This documents
    // the current limitation: even nested struct declarations at class
    // scope don't inherit @safe.
    //
    // A future improvement could propagate @safe to class-scope nested
    // types specifically while keeping local-method-body types
    // unaffected — but the AST walker would need to distinguish the
    // two cases. Until that lands, this test pins the current behavior
    // so accidental partial fixes don't introduce the local-method-body
    // false positive without also fixing the class-scope case.
    let temp_dir = TempDir::new().unwrap();

    let cpp = r#"
#include <atomic>

// @safe
class Outer {
public:
    struct Nested {
        mutable std::atomic<int> counter{0};
    };
};
"#;

    let cpp_path = temp_dir.path().join("class_scope_nested_mutable.cpp");
    fs::write(&cpp_path, cpp).unwrap();

    let (_success, output) = run_analyzer_on_file(&cpp_path);

    // Current behavior: no violations. Documenting this for now.
    assert!(
        output.contains("no violations found"),
        "current behavior: class-scope nested types do not inherit @safe.\n\
         If this assertion ever flips to true ('Found N violation(s)'),\n\
         that means the analyzer now propagates @safe to class-scope\n\
         nested types — verify that the local-method-body case in\n\
         test_local_struct_with_mutable_field_in_safe_method still\n\
         does NOT propagate.\nOutput:\n{}",
        output
    );
}
