//! Runtime behaviour of the guard-dispatch shapes the transpiler emits for
//! GENERIC (un-typeable) receivers — issues #32 and #34.
//!
//! The codegen unit tests pin WHAT is emitted (`rusty::deref_call(..)` /
//! `rusty::detail::deref_if_pointer_like(..)`); those are string assertions and
//! cannot tell whether the emitted C++ actually compiles or does the right
//! thing. These tests close that half: they compile and RUN the emitted shapes
//! against BOTH types a generic receiver can turn out to be —
//!
//!   * a `RefCell`, where `borrow()`/`borrow_mut()` yield a `Ref`/`RefMut`
//!     guard that must be dereferenced to reach the value, and
//!   * a plain type whose `borrow()`/`borrow_mut()` returns a reference
//!     directly (the identity case the free-helper/collapse paths exist for).
//!
//! Getting either wrong compiles-or-works for one shape and breaks the other,
//! which is exactly the failure mode both issues were.
//!
//! Because these compile against whatever `$CXX`/`c++` resolves to rather than
//! the project's default clang, they double as a guard that the runtime
//! headers stay portable: writing them is what caught `rusty.hpp` failing to
//! compile under GCC entirely (see the lookup note in `arch.hpp`).

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_cpp_compiler() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["c++", "g++", "clang++"] {
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
    let compiler = find_cpp_compiler().expect("no C++ compiler found in PATH or CXX");
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

/// Issue #32: `w.cell.borrow().get()` through a generic receiver lowers to
/// `rusty::deref_call(rusty::borrow(..), __mdisp_get{})`. It must reach the
/// value through a `Ref` guard AND still work when `borrow` is the identity
/// helper over a plain field.
#[test]
fn deref_call_dispatch_reaches_value_through_guard_and_identity() {
    let source = r#"
#include <rusty/rusty.hpp>
#include <rusty/refcell.hpp>
#include <cassert>

struct Inner { int v; int get() const { return v; } };
struct WCell  { rusty::RefCell<Inner> cell; };  // borrow() -> Ref<Inner> guard
struct WPlain { Inner cell; };                  // identity borrow

namespace rusty { namespace detail {
RUSTY_METHOD_DISPATCH(get)
} }

template<typename T>
int read_gen(const T& w) {
    return rusty::deref_call(rusty::borrow(w.cell), rusty::detail::__mdisp_get{});
}

int main() {
    WCell a{rusty::RefCell<Inner>(Inner{42})};
    assert(read_gen(a) == 42);

    WPlain b{Inner{7}};
    assert(read_gen(b) == 7);
    return 0;
}
"#;
    compile_and_run_cpp(source, "guard_dispatch_deref_call");
}

/// Issue #34: `*w.cell.borrow_mut() = v` through a generic receiver lowers to
/// `rusty::detail::deref_if_pointer_like(..) = v`. It must assign THROUGH a
/// `RefMut` guard, and still assign through a plain returned reference.
#[test]
fn deref_tolerant_assignment_targets_the_value_not_the_guard() {
    let source = r#"
#include <rusty/rusty.hpp>
#include <rusty/refcell.hpp>
#include <cassert>

struct WCell { rusty::RefCell<int> cell; };     // borrow_mut() -> RefMut<int>
struct Plain {
    int inner;
    int& borrow_mut() const { return const_cast<int&>(inner); }
};
struct WPlain { Plain cell; };                  // borrow_mut() -> int&

template<typename T>
void set_gen(const T& w, int v) {
    rusty::detail::deref_if_pointer_like(w.cell.borrow_mut()) = v;
}

int main() {
    WCell a{rusty::RefCell<int>(1)};
    set_gen(a, 42);
    assert(*a.cell.borrow() == 42);   // assigned the VALUE, not the guard

    WPlain b{Plain{7}};
    set_gen(b, 99);
    assert(b.cell.inner == 99);
    return 0;
}
"#;
    compile_and_run_cpp(source, "guard_dispatch_deref_assign");
}
