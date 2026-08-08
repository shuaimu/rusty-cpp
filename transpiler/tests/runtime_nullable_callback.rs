use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

    let run = Command::new(&bin_path)
        .output()
        .expect("run compiled binary");
    assert!(
        run.status.success(),
        "C++ binary failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn nullable_move_only_function_survives_assignment_replace_and_refmut_guard() {
    let source = r#"
#include <rusty/rusty.hpp>
#include <cassert>
#include <memory>
#include <type_traits>
#include <utility>

using ConstCallback = rusty::Function<int(int) const>;
using MutCallback = rusty::Function<void()>;

static_assert(!std::is_copy_constructible_v<ConstCallback>);
static_assert(!std::is_copy_constructible_v<MutCallback>);

int main() {
    ConstCallback empty{};
    assert(!static_cast<bool>(empty));

    empty = ConstCallback([payload = std::make_unique<int>(40)](int value) {
        return *payload + value;
    });
    assert(static_cast<bool>(empty));
    const ConstCallback& const_view = empty;
    assert(const_view(2) == 42);

    ConstCallback moved = std::move(empty);
    assert(!static_cast<bool>(empty));
    assert(moved(3) == 43);

    int calls = 0;
    rusty::RefCell<MutCallback> callback_slot(MutCallback{});
    MutCallback previous = callback_slot.replace(MutCallback(
        [payload = std::make_unique<int>(2), &calls]() mutable {
            calls += *payload;
            ++*payload;
        }));
    assert(!static_cast<bool>(previous));

    {
        auto&& callback_guard = callback_slot.borrow_mut();
        auto& callback = rusty::detail::deref_if_pointer_like(callback_guard);
        assert(static_cast<bool>(callback));
        callback();
        callback();
    }
    assert(calls == 5);

    MutCallback installed = callback_slot.replace(MutCallback{});
    assert(static_cast<bool>(installed));
    auto callback_guard = callback_slot.borrow_mut();
    assert(!static_cast<bool>(rusty::detail::deref_if_pointer_like(callback_guard)));
    return 0;
}
"#;
    compile_and_run_cpp(source, "nullable_move_only_callback");
}
