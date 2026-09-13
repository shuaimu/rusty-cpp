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
        .arg("-pthread")
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

#[test]
fn nullable_callbacks_with_auto_traits_translate_and_run_in_both_languages() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join("nullable_auto_traits.rs");
    let cpp_path = temp.path().join("nullable_auto_traits.cpp");
    let rust_bin = temp.path().join("nullable_auto_traits_rust");
    let source = r#"
use std::cell::RefCell;

type Callback = Box<dyn FnMut(&mut i32) + Send + Sync>;
type MaybeCallback = Option<Box<dyn FnMut(&mut i32)>>;

unsafe fn bump(value: *mut i32) { unsafe { *value += 1; } }

fn spawn_unit<F>(body: F) -> std::thread::JoinHandle<()>
where F: FnOnce() + Send + 'static {
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        if result.is_err() { std::process::abort(); }
    })
}

struct Holder {
    callback: RefCell<Option<Callback>>,
}

impl Holder {
    fn new() -> Self {
        Self { callback: RefCell::new(None) }
    }

    fn set(&self, callback: Callback) {
        self.callback.replace(Some(callback));
    }

    fn invoke(&self, value: &mut i32) {
        let mut slot = self.callback.borrow_mut();
        if let Some(callback) = slot.as_mut() {
            callback(value);
        }
    }

    fn clear(&self) -> bool {
        let previous = self.callback.replace(None);
        previous.is_some()
    }
}

pub fn check_nullable() -> i32 {
    let holder = Holder::new();
    let mut value = 0;
    holder.invoke(&mut value);
    if value != 0 { return 1; }

    let mut increment = 1;
    let callback: Callback = Box::new(move |value: &mut i32| {
        *value += increment;
        increment += 1;
    });
    holder.set(callback);
    holder.invoke(&mut value);
    holder.invoke(&mut value);
    if value != 3 { return 2; }
    if !holder.clear() { return 3; }
    if holder.clear() { return 4; }
    holder.invoke(&mut value);
    if value != 3 { return 5; }

    let mut plain: Option<Box<dyn FnMut() -> i32 + Send>> =
        Some(Box::new(|| -> i32 { 7 }));
    let taken = plain.take();
    if plain.is_some() || taken.is_none() { return 6; }
    let mut callback = taken.unwrap();
    if callback() != 7 { return 7; }

    let shared: Option<Box<dyn Fn(i32) -> i32 + Send + Sync>> =
        Some(Box::new(|value: i32| -> i32 { value + 1 }));
    if let Some(callback) = &shared {
        if callback(8) != 9 { return 8; }
    } else {
        return 9;
    }
    if shared.as_ref().unwrap()(10) != 11 { return 10; }
    let borrowed = shared.as_ref();
    if borrowed.unwrap()(11) != 12 { return 11; }
    if shared.is_none() { return 12; }
    let mut mutable: Option<Box<dyn FnMut() -> i32 + Send>> = Some(Box::new({
        let mut count = 0;
        move || { count += 1; count }
    }));
    if mutable.as_mut().unwrap()() != 1 { return 13; }
    {
        let borrowed = mutable.as_mut();
        if borrowed.unwrap()() != 2 { return 14; }
    }
    if let Some(callback) = mutable.as_mut() {
        if callback() != 3 { return 15; }
    }
    let empty: Option<Box<dyn Fn() -> i32>> = None;
    if empty.as_ref().is_some() { return 16; }
    let missing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        empty.as_ref().unwrap()()
    }));
    if missing.is_ok() { return 17; }
    let mutex = std::sync::Mutex::new(mutable);
    let mut guard = mutex.lock().unwrap();
    if !guard.is_some() { return 18; }
    if guard.as_mut().unwrap()() != 4 { return 19; }
    let mut callback: MaybeCallback = Some(Box::new(move |value: &mut i32| {
        unsafe {
            let pointer = value as *mut i32;
            bump(pointer);
        }
    }));
    let mut calls = 0;
    callback.as_mut().unwrap()(&mut calls);
    if calls != 1 { return 20; }
    let mut explicit: Callback = Box::new(|value: &mut i32| -> () {
        if *value == 1 { return unsafe { bump(value as *mut i32) }; }
        *value += 10;
    });
    explicit(&mut calls);
    explicit(&mut calls);
    if calls != 12 { return 21; }
    let mut expression: MaybeCallback = Some(Box::new(|value: &mut i32| *value += 1));
    expression.as_mut().unwrap()(&mut calls);
    if calls != 13 { return 22; }
    let mut empty: MaybeCallback = Some(Box::new(|_value: &mut i32| ()));
    empty.as_mut().unwrap()(&mut calls);
    if calls != 13 { return 23; }
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = completed.clone();
    let thread = spawn_unit(move || {
        observed.store(true, std::sync::atomic::Ordering::Release);
    });
    if thread.join().is_err() { return 24; }
    if !completed.load(std::sync::atomic::Ordering::Acquire) { return 25; }
    0
}
"#;
    std::fs::write(&source_path, source).expect("write Rust source");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source_path)
        .arg("-o")
        .arg(&cpp_path)
        .output()
        .expect("invoke transpiler");
    assert!(
        transpile.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&transpile.stderr)
    );
    let mut cpp = std::fs::read_to_string(&cpp_path).expect("read generated C++");
    assert!(
        !cpp.contains("rusty::Option<rusty::Function"),
        "nullable callbacks must use the Function empty state:\n{cpp}"
    );
    cpp.push_str("\nint main() { return check_nullable(); }\n");
    compile_and_run_cpp(&cpp, "nullable_auto_traits");

    let mut rust = source.to_string();
    rust.push_str("\nfn main() { assert_eq!(check_nullable(), 0); }\n");
    std::fs::write(&source_path, rust).expect("write Rust runtime check");
    let compile_rust = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source_path)
        .arg("-o")
        .arg(&rust_bin)
        .output()
        .expect("invoke rustc");
    assert!(
        compile_rust.status.success(),
        "Rust callback bounds failed to type-check:\n{}",
        String::from_utf8_lossy(&compile_rust.stderr)
    );
    let run_rust = Command::new(&rust_bin).output().expect("run Rust probe");
    assert!(
        run_rust.status.success(),
        "Rust runtime check failed:\n{}",
        String::from_utf8_lossy(&run_rust.stderr)
    );
}
