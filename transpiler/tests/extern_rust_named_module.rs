use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn find_tool(candidates: &[&str]) -> Option<String> {
    for candidate in candidates {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some((*candidate).to_string());
        }
    }
    None
}

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    find_tool(&["clang++", "clang++-22", "clang++-21", "clang++-20"])
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn extern_rust_named_module_links_to_implementation_unit_with_module_abi() {
    let Some(clang) = find_clang() else {
        eprintln!("skipping extern-Rust module gate: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust = temp.path().join("extern_rust_fixture.rs");
    std::fs::write(
        &rust,
        r#"
#![allow(dead_code, unsafe_code)]

unsafe extern "Rust" {
    pub fn platform_open(seed: i32) -> i32;
    pub fn platform_add(lhs: i32, rhs: i32) -> i32;
    fn private_hook(value: i32) -> i32;
}

pub fn invoke(seed: i32) -> i32 {
    unsafe { platform_add(platform_open(seed), 7_i32) }
}
"#,
    )
    .expect("write Rust fixture");

    let rust_metadata = temp.path().join("libextern_rust_fixture.rmeta");
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let rust_compile = Command::new(rustc)
        .arg("--crate-type=lib")
        .arg("--edition=2024")
        .arg("-Dwarnings")
        .arg("--emit=metadata")
        .arg("-o")
        .arg(&rust_metadata)
        .arg(&rust)
        .output()
        .expect("run rustc on fixture");
    assert_success(&rust_compile, "rustc fixture validation");

    let interface = temp.path().join("extern_rust_fixture.cppm");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .arg("-o")
        .arg(&interface)
        .arg("-m")
        .arg("extern_rust_fixture")
        .arg("--cxx-namespace")
        .arg("rrr")
        .output()
        .expect("run transpiler");
    assert_success(&transpile, "extern-Rust transpilation");

    let generated = std::fs::read_to_string(&interface).expect("read generated interface");
    assert!(
        !generated.contains("extern \"Rust\""),
        "C++ must never receive Rust's ABI string:\n{generated}"
    );
    assert!(
        generated.contains("export int32_t platform_open(int32_t seed);")
            && generated.contains("export int32_t platform_add(int32_t lhs, int32_t rhs);")
            && generated.contains("int32_t private_hook(int32_t value);")
            && !generated.contains("export int32_t private_hook"),
        "public/private declaration surface was not preserved:\n{generated}"
    );
    assert!(
        generated.contains("::rrr::platform_open") && generated.contains("::rrr::platform_add"),
        "calls must bind to declarations in the active namespace:\n{generated}"
    );

    let implementation = temp.path().join("extern_rust_fixture_impl.cpp");
    std::fs::write(
        &implementation,
        r#"module;
#include <cstdint>
module extern_rust_fixture;

namespace rrr {
int32_t platform_open(int32_t seed) { return seed * 2; }
int32_t platform_add(int32_t lhs, int32_t rhs) { return lhs + rhs; }
int32_t private_hook(int32_t value) { return value; }
}  // namespace rrr
"#,
    )
    .expect("write implementation unit");
    let importer = temp.path().join("importer.cpp");
    std::fs::write(
        &importer,
        r#"import extern_rust_fixture;
int main() { return rrr::invoke(5) == 17 ? 0 : 1; }
"#,
    )
    .expect("write importer");

    let include = project_include_dir();
    let pcm = temp.path().join("extern_rust_fixture.pcm");
    let interface_object = temp.path().join("extern_rust_fixture.o");
    let implementation_object = temp.path().join("extern_rust_fixture_impl.o");
    let importer_object = temp.path().join("importer.o");
    let binary = temp.path().join("extern_rust_fixture");
    let module_map = format!("-fmodule-file=extern_rust_fixture={}", pcm.display());

    let precompile = Command::new(&clang)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(&include)
        .arg("-x")
        .arg("c++-module")
        .arg("--precompile")
        .arg(&interface)
        .arg("-o")
        .arg(&pcm)
        .output()
        .expect("precompile module interface");
    assert_success(&precompile, "module interface precompile");

    for (context, language, source, object) in [
        (
            "module interface object",
            "c++-module",
            &interface,
            &interface_object,
        ),
        (
            "module implementation object",
            "c++",
            &implementation,
            &implementation_object,
        ),
        ("module importer object", "c++", &importer, &importer_object),
    ] {
        let compile = Command::new(&clang)
            .arg("-std=c++23")
            .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
            .arg("-I")
            .arg(&include)
            .arg("-x")
            .arg(language)
            .arg("-c")
            .arg(source)
            .arg(&module_map)
            .arg("-o")
            .arg(object)
            .output()
            .expect("compile named-module lane");
        assert_success(&compile, context);
    }

    if cfg!(target_os = "linux") {
        let nm = find_tool(&["llvm-nm", "nm"]).expect("nm is required for the ABI gate");
        let symbols = Command::new(nm)
            .arg("--defined-only")
            .arg(&implementation_object)
            .output()
            .expect("inspect implementation-unit symbols");
        assert_success(&symbols, "implementation-unit symbol inspection");
        let symbols = String::from_utf8_lossy(&symbols.stdout);
        assert!(
            symbols.contains("_ZN3rrrW19extern_rust_fixture13platform_openEi")
                && symbols.contains("_ZN3rrrW19extern_rust_fixture12platform_addEii"),
            "implementation definitions did not retain the exact namespace/module ABI:\n{symbols}"
        );
    }

    let link = Command::new(&clang)
        .arg(&interface_object)
        .arg(&implementation_object)
        .arg(&importer_object)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("link named-module runtime");
    assert_success(&link, "named-module link");
    let run = Command::new(&binary)
        .output()
        .expect("run named-module binary");
    assert_success(&run, "named-module runtime");
}

#[test]
fn extern_rust_fails_closed_without_named_module_output() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust = temp.path().join("direct.rs");
    std::fs::write(
        &rust,
        r#"unsafe extern "Rust" { pub fn platform_open() -> i32; }
pub fn invoke() -> i32 { unsafe { platform_open() } }
"#,
    )
    .expect("write direct-mode fixture");
    let output_path = temp.path().join("direct.cpp");
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("run direct-mode transpilation");
    assert!(!output.status.success(), "direct mode must fail closed");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("`extern \"Rust\"` declarations require named C++ module output"),
        "unexpected diagnostic:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output_path.exists(),
        "failed transpilation must not leave partial output"
    );
}

#[test]
fn extern_rust_rejects_non_function_foreign_items() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust = temp.path().join("foreign_static.rs");
    std::fs::write(
        &rust,
        "unsafe extern \"Rust\" { pub static PLATFORM_SLOT: i32; }\n",
    )
    .expect("write foreign-static fixture");
    let output_path = temp.path().join("foreign_static.cppm");
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .arg("-o")
        .arg(&output_path)
        .arg("-m")
        .arg("foreign_static")
        .output()
        .expect("run foreign-static transpilation");
    assert!(
        !output.status.success(),
        "unsupported item must fail closed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("named-module `extern \"Rust\"` supports function declarations only"),
        "unexpected diagnostic:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output_path.exists(),
        "failed transpilation must not leave partial output"
    );
}
