//! Runtime behaviour of the shape `Default::default()` lowers to.
//!
//! This test exists because of a bug the string-assertion tests could not
//! see, and in fact locked in: `Default::default()` emitted
//! `rusty::default_value<T>()`, a function that **has never existed** in
//! `include/`. Three codegen tests asserted that exact spelling and passed,
//! because asserting on emitted text cannot distinguish a valid call from an
//! invented symbol. The real helper is `rusty::default_like<T>()`.
//!
//! The consequence was not academic: every "the DSL cannot spell a default
//! ctor" hand-bridge in a downstream consumer traced back to it. Someone
//! tried the obvious spelling, got output that did not compile, and recorded
//! the correct conclusion from wrong evidence -- a tool that fails by
//! emitting a plausible-looking symbol makes "unsupported" and "misspelled"
//! indistinguishable at the call site.
//!
//! So these tests compile and RUN the emitted shape across the tiers
//! `default_like` dispatches over, rather than asserting on its text:
//!
//!   * a type with a `default_()` static member (the struct-emission shape),
//!   * a plain aggregate with no members at all (value-init tier),
//!   * a `std::`-typed value, and
//!   * a move-only `rusty::Function`, which is the case a downstream
//!     consumer needed and the one that reads worst when it breaks.
//!
//! Same rationale as `runtime_guard_dispatch.rs`: the codegen unit tests pin
//! WHAT is emitted; these close the half that asks whether it is real.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Ask for clang by name rather than taking whatever `c++` happens to be --
/// same reasoning as the sibling runtime harnesses. Skip rather than fall
/// back, so a non-clang system compiler cannot redden this.
fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["clang++", "clang++-22", "clang++-21"] {
        let status = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

fn compile_and_run_cpp(source: &str, test_name: &str) {
    let Some(compiler) = find_clang() else {
        eprintln!("skipping {test_name}: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join(format!("{test_name}.cpp"));
    let bin_path = temp.path().join(format!("{test_name}.bin"));

    std::fs::write(&source_path, source).expect("write C++ source");

    let compile = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(project_include_dir())
        .arg(&source_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke C++ compiler");
    assert!(
        compile.status.success(),
        "C++ compile failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "C++ binary failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

/// The helper `Default::default()` lowers to must EXIST and pick the right
/// tier. A string assertion cannot check either.
#[test]
fn default_like_resolves_across_its_dispatch_tiers() {
    let source = r#"
#include <rusty/slice.hpp>
#include <rusty/dispatch.hpp>
#include <cassert>
#include <string>

// Tier 1: struct emissions carry a `default_()` static member.
struct WithMember {
    int v;
    static WithMember default_() { return WithMember{7}; }
};

// Tier 3: a plain aggregate falls through to value-init.
struct PlainAggregate { int a; double b; const char* c; };

int main() {
    // Tier 1 must prefer the member over value-init.
    assert(rusty::default_like<WithMember>().v == 7);

    // Tier 3 value-inits every field.
    const PlainAggregate p = rusty::default_like<PlainAggregate>();
    assert(p.a == 0);
    assert(p.b == 0.0);
    assert(p.c == nullptr);

    // A std::-typed value is reachable too -- this is the family that was
    // hand-bridged downstream on the belief it was not.
    assert(rusty::default_like<std::string>().empty());
    assert(rusty::default_like<int>() == 0);
    return 0;
}
"#;
    compile_and_run_cpp(source, "default_like_tiers");
}

/// The move-only case. `rusty::Function` has no `default_()` member, so it
/// must reach the value-init tier and come back empty-but-usable.
///
/// Worth its own test because a move-only default is where a wrong lowering
/// is most confusing: bind it with the wrong constness downstream and the
/// error is `call to deleted constructor`, which points at the binding rather
/// than at the default.
#[test]
fn default_like_builds_an_empty_move_only_function() {
    let source = r#"
#include <rusty/slice.hpp>
#include <rusty/dispatch.hpp>
#include <rusty/function.hpp>
#include <cassert>

using Sink = rusty::Function<void(int&)>;

int main() {
    // Default-constructed: empty, and contextually false.
    Sink empty = rusty::default_like<Sink>();
    assert(!static_cast<bool>(empty));

    // Still assignable and callable afterwards -- a default that cannot be
    // filled in would be useless to the call sites that need one.
    int seen = 0;
    empty = Sink([](int& out) { out = 5; });
    assert(static_cast<bool>(empty));
    empty(seen);
    assert(seen == 5);
    return 0;
}
"#;
    compile_and_run_cpp(source, "default_like_move_only");
}
