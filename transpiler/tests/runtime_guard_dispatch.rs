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
//! These compile with clang specifically -- it is the project's toolchain and
//! the only compiler these shapes are required to work under.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Ask for clang by name rather than taking whatever `c++` happens to be.
/// clang is the project's toolchain (CLAUDE.md) and the only compiler these
/// shapes have to work under, so a non-clang system compiler must not be able
/// to redden this test. If no clang is present the test skips rather than
/// falling back -- a gcc failure here would be noise, not a regression.
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

/// Issue #35: the same two shapes for a CONCRETE receiver — an inline
/// `deref_call` on the guard, and a guard bound with `auto&&` and dereferenced
/// through `deref_if_pointer_like`. Both must reach the guarded value, and the
/// bound form must also still work when the call yields a plain reference,
/// which is the case `auto&&` exists to cover.
#[test]
fn concrete_receiver_guard_shapes_reach_the_value_bound_or_inline() {
    let source = r#"
#include <rusty/rusty.hpp>
#include <rusty/refcell.hpp>
#include <cassert>

namespace rusty { namespace detail {
RUSTY_METHOD_DISPATCH(push_back)
} }

struct Guarded { rusty::RefCell<rusty::VecDeque<int>> v; };

// borrow_mut() returns a plain reference rather than a guard.
struct Plain {
    rusty::VecDeque<int> inner;
    rusty::VecDeque<int>& borrow_mut() { return inner; }
};

int main() {
    Guarded g{rusty::RefCell<rusty::VecDeque<int>>(rusty::VecDeque<int>())};

    // inline: dispatched through the guard
    rusty::deref_call(g.v.borrow_mut(), rusty::detail::__mdisp_push_back{}, 1);
    // bound: auto&& holds the guard prvalue alive, deref stays tolerant
    {
        auto&& held = g.v.borrow_mut();
        rusty::detail::deref_if_pointer_like(held).push_back(2);
    }
    {
        auto held = g.v.borrow();
        assert((*held).len() == 2);
        assert((*held)[0] == 1);
        assert((*held)[1] == 2);
    }

    // the same bound shape over a plain reference must alias, not copy
    Plain p{rusty::VecDeque<int>()};
    {
        auto&& held = p.borrow_mut();
        rusty::detail::deref_if_pointer_like(held).push_back(7);
    }
    assert(p.inner.len() == 1);
    assert(p.inner[0] == 7);
    return 0;
}
"#;
    compile_and_run_cpp(source, "guard_dispatch_concrete_receiver");
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
