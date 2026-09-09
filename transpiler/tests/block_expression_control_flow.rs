//! Execute block-valued initializers in both languages.

use std::path::PathBuf;
use std::process::Command;

const SOURCE: &str = r#"
use std::sync::Mutex;

pub fn outer_control() -> i32 {
    let mut n = 0;
    let mut total = 0;
    loop {
        n += 1;
        let amount = {
            let doubled = n * 2;
            if n == 2 { continue; }
            if n == 5 { break; }
            doubled
        };
        total += amount;
    }
    total
}

pub fn labeled_control() -> i32 {
    let mut n = 0;
    let mut total = 0;
    'outer: loop {
        n += 1;
        let amount = {
            let doubled = n * 2;
            loop {
                if n == 3 { break 'outer; }
                break;
            }
            doubled
        };
        total += amount;
    }
    total
}

pub fn inner_control() -> i32 {
    let result = {
        let mut value = 0;
        loop {
            value += 1;
            if value < 3 { continue; }
            break;
        }
        value
    };
    result
}

pub fn iterable_control() -> i32 {
    let mut n = 0;
    let mut total = 0;
    loop {
        n += 1;
        let amount = {
            let doubled = n * 2;
            for _ in { if n == 3 { break; } [0] } {}
            doubled
        };
        total += amount;
    }
    total
}

pub fn choose(enabled: bool) -> Option<i32> {
    let selected = {
        let candidate = 42_i32;
        if enabled { Some(candidate) } else { None }
    };
    selected
}

pub fn choose_value(enabled: bool) -> i32 {
    let selected = {
        let candidate = 42_i32;
        if enabled { Some(candidate) } else { None }
    };
    if let Some(value) = selected { value } else { -1 }
}

pub fn guard_is_released() -> i32 {
    let state = Mutex::new(7_i32);
    let value = {
        let guard = state.lock().unwrap();
        if *guard == 0 { return -1; }
        7_i32
    };
    let next = state.try_lock().unwrap();
    value + *next
}

pub fn shadowed() -> i32 {
    let value = 4_i32;
    let value = {
        let value = value + 3;
        if value == 0 { return -1; }
        value
    };
    value
}

pub fn sink_name_collision() -> i32 {
    let value = {
        let _let_block_value = 9_i32;
        if _let_block_value == 0 { return -1; }
        _let_block_value
    };
    value
}

pub fn reference_result(stop: bool, input: &mut i32) -> i32 {
    let result = {
        if stop { return 0; }
        input
    };
    *result += 2;
    *result
}

pub fn immutable_reference_result(stop: bool, input: &i32) -> i32 {
    let result = {
        if stop { return 0; }
        input
    };
    *result
}

pub fn view_result(stop: bool, input: &str) -> usize {
    let result = {
        if stop { return 0; }
        input
    };
    result.len()
}
"#;

fn checked(command: &mut Command) {
    let output = command.output().expect("run fixture command");
    assert!(output.status.success(), "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn block_initializers_preserve_control_targets_and_option_result_types() {
    let scratch = tempfile::tempdir().unwrap();
    let rust = scratch.path().join("blocks.rs");
    let rust_main = scratch.path().join("rust_main.rs");
    let rust_exe = scratch.path().join("rust_run");
    std::fs::write(&rust, SOURCE).unwrap();
    std::fs::write(&rust_main, format!("{SOURCE}\nfn main() {{
        assert_eq!(outer_control(), 16);
        assert_eq!(labeled_control(), 6);
        assert_eq!(inner_control(), 3);
        assert_eq!(iterable_control(), 6);
        assert_eq!(choose(true), Some(42));
        assert_eq!(choose(false), None);
        assert_eq!(choose_value(true), 42);
        assert_eq!(choose_value(false), -1);
        assert_eq!(guard_is_released(), 14);
        assert_eq!(shadowed(), 7);
        assert_eq!(sink_name_collision(), 9);
        let mut value = 11_i32;
        assert_eq!(reference_result(true, &mut value), 0);
        assert_eq!(value, 11);
        assert_eq!(reference_result(false, &mut value), 13);
        assert_eq!(value, 13);
        assert_eq!(immutable_reference_result(false, &value), 13);
        assert_eq!(view_result(false, \"payload\"), 7);
    }}")).unwrap();
    checked(Command::new("rustc").args(["--edition=2024", "-O"])
        .arg(&rust_main).arg("-o").arg(&rust_exe));
    checked(&mut Command::new(&rust_exe));

    let module = scratch.path().join("blocks.cppm");
    checked(Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust).arg("-o").arg(&module)
        .args(["-m", "block_runtime", "--cxx-namespace", "fixture"]));

    let clang = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../include");
    let pcm = scratch.path().join("blocks.pcm");
    let object = scratch.path().join("blocks.o");
    let importer = scratch.path().join("importer.cpp");
    let executable = scratch.path().join("cpp_run");
    let flags = ["-std=c++23", "-O2", "-DRUSTY_PORTABLE_INTRINSICS=1"];
    std::fs::write(&importer, r#"
#include <cstdint>
import block_runtime;
int main() {
    if (fixture::outer_control() != 16) return 1;
    if (fixture::labeled_control() != 6) return 2;
    if (fixture::inner_control() != 3) return 3;
    auto some = fixture::choose(true);
    if (!some.is_some() || some.unwrap() != 42) return 4;
    if (!fixture::choose(false).is_none()) return 5;
    if (fixture::guard_is_released() != 14) return 6;
    if (fixture::choose_value(true) != 42) return 7;
    if (fixture::choose_value(false) != -1) return 8;
    if (fixture::iterable_control() != 6) return 9;
    if (fixture::shadowed() != 7) return 10;
    if (fixture::sink_name_collision() != 9) return 11;
    int32_t value = 11;
    if (fixture::reference_result(true, value) != 0 || value != 11) return 12;
    if (fixture::reference_result(false, value) != 13 || value != 13) return 13;
    if (fixture::immutable_reference_result(false, value) != 13) return 14;
    if (fixture::view_result(false, "payload") != 7) return 15;
}
"#).unwrap();
    checked(Command::new(&clang).args(flags).arg("-I").arg(&include)
        .arg("--precompile").arg(&module).arg("-o").arg(&pcm));
    checked(Command::new(&clang).args(flags).arg("-c").arg(&pcm)
        .arg("-o").arg(&object));
    checked(Command::new(&clang).args(flags).arg("-I").arg(&include)
        .arg(format!("-fmodule-file=block_runtime={}", pcm.display()))
        .arg(&importer).arg(&object).arg("-o").arg(&executable));
    checked(&mut Command::new(executable));
}
