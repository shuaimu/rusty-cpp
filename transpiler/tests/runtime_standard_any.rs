use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn transpile(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.rs");
    let output = dir.path().join("output.cpp");
    std::fs::write(&input, source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::read_to_string(output).unwrap()
}

fn compile_run(source: &str, abort: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.cpp");
    let output = dir.path().join("test");
    std::fs::write(&input, source).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let include: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    let mut command = Command::new(compiler);
    command.args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1"]);
    if abort {
        command.arg("-DRUSTY_PANIC_ABORT=1");
    }
    let result = command
        .arg("-I")
        .arg(include)
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(output).output().unwrap();
    if !abort {
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    result
}

#[test]
fn typed_standard_panic_and_boxed_any_downcasts_compile_and_run() {
    let cpp = transpile(
        r#"
use std::any::Any;
use std::panic::{catch_unwind, panic_any};
type Payload = Box<dyn Any + Send>;
type Borrowed<'a> = &'a Payload;
pub fn describe(payload: Payload) -> Option<String> {
    if let Some(text) = payload.downcast_ref::<&str>() { return Some((*text).to_string()); }
    if let Some(text) = payload.downcast_ref::<String>() { return Some(text.clone()); }
    None
}
pub fn literal_panic() -> Payload { catch_unwind(|| panic_any("hello\0there")).unwrap_err() }
pub fn owned_panic() -> Payload { catch_unwind(|| panic_any(String::from("owned"))).unwrap_err() }
pub fn number() -> Payload { Box::new(41_i32) }
pub fn resume(payload: Payload) { std::panic::resume_unwind(payload) }
pub fn mutate(payload: &mut Payload) -> bool {
    if let Some(value) = payload.downcast_mut::<i32>() { *value += 1; return true; }
    false
}
pub fn is_integer(payload: Borrowed<'_>) -> bool { payload.is::<i32>() }
"#,
    );
    assert!(cpp.contains("rusty::any_types::BoxAny"), "{cpp}");
    compile_run(
        &format!(
            r#"{cpp}
#include <cassert>
int main() {{
    auto literal = literal_panic();
    assert(literal.downcast_ref<std::string_view>().is_some());
    assert(literal.downcast_ref<rusty::String>().is_none());
    assert(describe(std::move(literal)).unwrap().as_str() == std::string_view("hello\0there", 11));
    auto owned = owned_panic();
    assert(owned.downcast_ref<rusty::String>().is_some());
    assert(owned.downcast_ref<std::string_view>().is_none());
    assert(describe(std::move(owned)).unwrap() == "owned");
    auto resumed = rusty::panic::catch_unwind_std([] {{ resume(number()); }}).unwrap_err();
    assert(resumed.downcast_ref<int32_t>().unwrap() == 41);
    auto value = number();
    assert(is_integer(value)); assert(mutate(value));
    assert(value.downcast_ref<int32_t>().unwrap() == 42);
    assert(describe(std::move(value)).is_none());
}}
"#
        ),
        false,
    );
}

#[test]
fn any_owner_preserves_move_only_values_borrows_drops_and_foreign_exceptions() {
    compile_run(
        r#"
#include <rusty/rusty.hpp>
#include <cassert>
struct Tracked {
    std::unique_ptr<int> value;
    int* drops;
    Tracked(int value, int& drops) : value(std::make_unique<int>(value)), drops(&drops) {}
    Tracked(Tracked&&) = default;
    ~Tracked() { if (value) ++*drops; }
};
struct Foreign : std::runtime_error {
    int code = 73;
    Foreign() : std::runtime_error("foreign diagnostic") {}
};
int main() {
    int drops = 0;
    {
        auto caught = rusty::panic::catch_unwind_std([&] { rusty::panic::panic_any(Tracked(7, drops)); });
        auto payload = caught.unwrap_err();
        const auto* address = &payload.downcast_ref<Tracked>().unwrap();
        auto moved = std::move(payload);
        assert(&moved.downcast_ref<Tracked>().unwrap() == address);
        *moved.downcast_mut<Tracked>().unwrap().value = 8;
        assert(moved.downcast_ref<int>().is_none());
        auto resumed = rusty::panic::catch_unwind_std([&] { rusty::panic::resume_unwind_std(std::move(moved)); });
        assert(*resumed.unwrap_err().downcast_ref<Tracked>().unwrap().value == 8);
    }
    assert(drops == 1);
    auto foreign = rusty::panic::catch_unwind_std([] { throw Foreign(); });
    auto payload = foreign.unwrap_err();
    assert(payload.downcast_ref<rusty::String>().unwrap() == "foreign diagnostic");
    try { rusty::panic::resume_unwind_std(std::move(payload)); }
    catch (const Foreign& e) { assert(e.code == 73); }
    auto opaque = rusty::panic::catch_unwind_std([] { throw 91; });
    auto opaque_payload = opaque.unwrap_err();
    assert(opaque_payload.downcast_ref<rusty::String>().is_none());
    assert(opaque_payload.downcast_ref<std::string_view>().is_none());
    try { rusty::panic::resume_unwind_std(std::move(opaque_payload)); }
    catch (int value) { assert(value == 91); }
    auto legacy = rusty::panic::catch_unwind([] { throw Foreign(); });
    assert(rusty::panic::payload_message(legacy.unwrap_err()).unwrap() == "foreign diagnostic");
    auto copied_exception = rusty::panic::catch_unwind([] {
        rusty::panic::panic_any(rusty::String::from("shared exception"));
    }).unwrap_err();
    for (int attempt = 0; attempt < 2; ++attempt) {
        auto repeated = rusty::panic::catch_unwind_std([&] {
            rusty::panic::resume_unwind(copied_exception);
        }).unwrap_err();
        assert(repeated.has_value());
        assert(repeated.downcast_ref<rusty::String>().unwrap() == "shared exception");
    }
}
"#,
        false,
    );
}

#[test]
fn local_any_and_box_names_do_not_acquire_standard_type_erasure() {
    for source in [
        "trait Any {} pub fn own(value: Box<dyn Any>) {}",
        "mod std { pub mod any { pub trait Any {} } } pub fn own(value: Box<dyn std::any::Any>) {}",
        "struct Box<T>(T); use std::any::Any; pub fn own(value: Box<dyn Any>) {}",
        "extern crate alternate as std; pub fn own(value: Box<dyn std::any::Any>) {}",
    ] {
        let cpp = transpile(source);
        assert!(!cpp.contains("rusty::any_types::BoxAny"), "{cpp}");
    }
}

#[test]
fn standard_panic_any_respects_abort_strategy() {
    let result = compile_run(
        r#"
#include <rusty/panic.hpp>
int main() { rusty::panic::panic_any(rusty::String::from("abort diagnostic")); }
"#,
        true,
    );
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("abort diagnostic"));
}
