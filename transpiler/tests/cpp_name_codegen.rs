use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX")
        && !cxx.trim().is_empty()
    {
        return Some(cxx);
    }
    for candidate in ["clang++", "clang++-22", "clang++-21"] {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn run_transpiler(source: &Path, output: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
    command.arg(source).arg("-o").arg(output);
    command.args(extra).output().expect("run transpiler")
}

fn run_crate_transpiler(cargo_toml: &Path, output_dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(cargo_toml)
        .arg("--output-dir")
        .arg(output_dir)
        .output()
        .expect("run crate-mode transpiler")
}

#[test]
fn inert_cpp_name_overloads_compile_link_and_run_as_named_module() {
    let temp = tempfile::tempdir().expect("tempdir");
    let rust = temp.path().join("callbacks.rs");
    std::fs::write(
        &rust,
        r#"
#[cfg_attr(any(), cpp_name(invoke_callback_safely))]
pub fn invoke_i32(value: i32) -> i32 { value + 1 }

#[cfg_attr(any(), cpp_name(invoke_callback_safely))]
pub fn invoke_bool(value: bool) -> i32 { if value { 7 } else { 2 } }

#[cfg_attr(any(), cpp_name(invoke_callback_safely))]
pub fn invoke_pair(left: i32, right: i32) -> i32 { left + right }

pub fn route_i32(value: i32) -> i32 { invoke_i32(value) }
pub fn route_bool(value: bool) -> i32 { crate::invoke_bool(value) }
pub fn route_pair(left: i32, right: i32) -> i32 { self::invoke_pair(left, right) }
pub fn route_raw(value: i32) -> i32 { r#invoke_i32(value) }
pub mod nested {
    pub fn route_super(value: i32) -> i32 { super::invoke_i32(value) }
}
"#,
    )
    .expect("write Rust fixture");

    // The marker contract is deliberately inert for ordinary rustc.
    let rust_library = temp.path().join("libcallbacks.rlib");
    let rustc = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib"])
        .arg(&rust)
        .arg("-o")
        .arg(&rust_library)
        .output()
        .expect("run rustc");
    assert!(
        rustc.status.success(),
        "inert cpp_name fixture is not rustc-valid:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let cpp = temp.path().join("callbacks.cppm");
    let transpile = run_transpiler(
        &rust,
        &cpp,
        &["-m", "cpp_name_runtime", "--cxx-namespace", "rrr"],
    );
    assert!(
        transpile.status.success(),
        "transpilation failed:\n{}\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );

    let generated = std::fs::read_to_string(&cpp).expect("read generated C++ module");
    assert!(generated.contains("export module cpp_name_runtime;"));
    assert_eq!(
        generated
            .lines()
            .filter(|line| {
                line.trim_start()
                    .starts_with("export int32_t invoke_callback_safely(")
            })
            .count(),
        6,
        "three forward declarations and three definitions must share the exact C++ name:\n{generated}"
    );
    for rust_name in ["invoke_i32", "invoke_bool", "invoke_pair"] {
        assert!(
            !generated.contains(rust_name),
            "Rust-only identity leaked into generated C++: {rust_name}\n{generated}"
        );
    }
    assert_eq!(
        generated
            .matches("return ::rrr::invoke_callback_safely(")
            .count(),
        5,
        "bare, raw, crate, self, and super paths must deterministically use the ABI name:\n{generated}"
    );

    let Some(clang) = find_clang() else {
        eprintln!("skipping cpp_name C++ runtime gate: no clang++ in PATH or CXX");
        return;
    };
    let importer = temp.path().join("importer.cpp");
    std::fs::write(
        &importer,
        r#"
import cpp_name_runtime;
int main() {
    if (rrr::route_i32(4) != 5) return 1;
    if (rrr::route_bool(true) != 7) return 2;
    if (rrr::route_bool(false) != 2) return 3;
    if (rrr::route_pair(19, 23) != 42) return 4;
    if (rrr::nested::route_super(8) != 9) return 5;
    if (rrr::route_raw(10) != 11) return 6;
    return 0;
}
"#,
    )
    .expect("write C++ importer");

    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include");
    let pcm = temp.path().join("callbacks.pcm");
    let module_object = temp.path().join("callbacks.o");
    let importer_object = temp.path().join("importer.o");
    let binary = temp.path().join("cpp_name_runtime");

    let precompile = Command::new(&clang)
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"])
        .arg("-I")
        .arg(&include)
        .args(["-x", "c++-module", "--precompile"])
        .arg(&cpp)
        .arg("-o")
        .arg(&pcm)
        .output()
        .expect("precompile generated module");
    assert!(
        precompile.status.success(),
        "cpp_name module failed to precompile:\n{}",
        String::from_utf8_lossy(&precompile.stderr)
    );

    for (label, source, output, language) in [
        ("module", &cpp, &module_object, "c++-module"),
        ("importer", &importer, &importer_object, "c++"),
    ] {
        let compile = Command::new(&clang)
            .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"])
            .arg("-I")
            .arg(&include)
            .arg("-x")
            .arg(language)
            .arg("-c")
            .arg(source)
            .arg(format!("-fmodule-file=cpp_name_runtime={}", pcm.display()))
            .arg("-o")
            .arg(output)
            .output()
            .expect("compile cpp_name module lane");
        assert!(
            compile.status.success(),
            "{label} object failed to compile:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
    }

    let link = Command::new(&clang)
        .args([&module_object, &importer_object])
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("link cpp_name runtime lane");
    assert!(
        link.status.success(),
        "cpp_name runtime failed to link:\n{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let run = Command::new(&binary).output().expect("run cpp_name binary");
    assert!(run.status.success(), "cpp_name runtime returned {run:?}");
}

#[test]
fn generic_cpp_name_arity_overloads_are_direct_crate_and_clang_valid() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = r#"
#![deny(unsafe_code)]

#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
#[allow(unsafe_code)]
pub fn make_serializable_proxy_default<T: 'static>() -> i32 { 7 }

#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
#[allow(unsafe_code)]
pub fn make_serializable_proxy_copy<T: 'static>(_value: &T) -> i32 { 9 }

#[cfg_attr(any(), cpp_trait_member_dispatch)]
pub trait SinkBase { fn sink_kind(&self) -> i32; }

#[cfg_attr(any(), cpp_trait_member_dispatch)]
pub trait SourceBase { fn source_kind(&self) -> i32; }

pub struct BufferSink;
pub struct FdSink;
pub struct BufferSource;
pub struct FdSource;

#[cfg_attr(any(), cpp_name(make_sink_proxy))]
#[allow(unsafe_code)]
pub unsafe fn make_sink_proxy_buffer(_sink: *mut BufferSink) -> i32 { 11 }

#[cfg_attr(any(), cpp_name(make_sink_proxy))]
#[allow(unsafe_code)]
pub unsafe fn make_sink_proxy_fd(_sink: *mut FdSink) -> i32 { 12 }

#[cfg_attr(any(), cpp_name(make_source_proxy))]
#[allow(unsafe_code)]
pub unsafe fn make_source_proxy_buffer(_source: *mut BufferSource) -> i32 { 13 }

#[cfg_attr(any(), cpp_name(make_source_proxy))]
#[allow(unsafe_code)]
pub unsafe fn make_source_proxy_fd(_source: *mut FdSource) -> i32 { 14 }

pub fn route_default() -> i32 { make_serializable_proxy_default::<i32>() }
pub fn route_copy(value: i32) -> i32 {
    crate::make_serializable_proxy_copy::<i32>(&value)
}
pub fn route_copy_bare(value: i32) -> i32 {
    make_serializable_proxy_copy(&value)
}
#[allow(unsafe_code)]
pub unsafe fn route_sink_buffer(sink: *mut BufferSink) -> i32 {
    unsafe { make_sink_proxy_buffer(sink) }
}
#[allow(unsafe_code)]
pub unsafe fn route_sink_fd(sink: *mut FdSink) -> i32 {
    unsafe { crate::make_sink_proxy_fd(sink) }
}
#[allow(unsafe_code)]
pub unsafe fn route_source_buffer(source: *mut BufferSource) -> i32 {
    unsafe { make_source_proxy_buffer(source) }
}
#[allow(unsafe_code)]
pub unsafe fn route_source_fd(source: *mut FdSource) -> i32 {
    unsafe { crate::make_source_proxy_fd(source) }
}
"#;
    let rust = temp.path().join("generic.rs");
    std::fs::write(&rust, source).expect("write generic Rust fixture");

    let rust_library = temp.path().join("libgeneric.rlib");
    let rustc = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib"])
        .arg(&rust)
        .arg("-o")
        .arg(&rust_library)
        .output()
        .expect("run rustc for generic cpp_name fixture");
    assert!(
        rustc.status.success(),
        "generic cpp_name fixture is not rustc-valid:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let cpp = temp.path().join("generic.cppm");
    let transpile = run_transpiler(
        &rust,
        &cpp,
        &["-m", "cpp_name_generic_runtime", "--cxx-namespace", "rrr"],
    );
    assert!(
        transpile.status.success(),
        "direct generic cpp_name transpilation failed:\n{}\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );
    let generated = std::fs::read_to_string(&cpp).expect("read generic C++ module");
    for rust_name in [
        "make_serializable_proxy_default",
        "make_serializable_proxy_copy",
        "make_sink_proxy_buffer",
        "make_sink_proxy_fd",
        "make_source_proxy_buffer",
        "make_source_proxy_fd",
    ] {
        assert!(
            !generated.contains(rust_name),
            "Rust-only generic identity leaked into C++: {rust_name}\n{generated}"
        );
    }
    assert_eq!(
        generated.matches("make_serializable_proxy(").count(),
        5,
        "two declarations, two definitions, and the bare rewritten call must use make_serializable_proxy:\n{generated}"
    );
    assert_eq!(
        generated
            .matches("make_serializable_proxy<int32_t>")
            .count(),
        2,
        "both rewritten generic calls must preserve their explicit template argument:\n{generated}"
    );
    assert!(
        generated.matches("template<typename T>").count() >= 4,
        "generic declarations and definitions lost their exact template heads:\n{generated}"
    );
    for signature in [
        "make_serializable_proxy()",
        "make_serializable_proxy(const T& _value)",
        "make_sink_proxy(BufferSink* _sink)",
        "make_sink_proxy(FdSink* _sink)",
        "make_source_proxy(BufferSource* _source)",
        "make_source_proxy(FdSource* _source)",
    ] {
        assert!(
            generated.contains(signature),
            "cpp_name lost its exact historical signature `{signature}`:\n{generated}"
        );
    }
    assert_eq!(
        generated.matches("make_sink_proxy(").count(),
        6,
        "two declarations, two definitions, and two unsafe Rust calls must share the historical sink name:\n{generated}"
    );
    assert_eq!(
        generated.matches("make_source_proxy(").count(),
        6,
        "two declarations, two definitions, and two unsafe Rust calls must share the historical source name:\n{generated}"
    );

    let crate_root = temp.path().join("crate_lane");
    std::fs::create_dir_all(crate_root.join("src")).unwrap();
    let manifest = crate_root.join("Cargo.toml");
    std::fs::write(
        &manifest,
        "[package]\nname = \"cpp_name_generic_crate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(crate_root.join("src/lib.rs"), source).unwrap();
    let cargo_check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(crate_root.join("rust-target"))
        .output()
        .expect("cargo-check generic crate fixture");
    assert!(
        cargo_check.status.success(),
        "generic crate fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    let crate_output = crate_root.join("cpp_out");
    let crate_result = run_crate_transpiler(&manifest, &crate_output);
    assert!(
        crate_result.status.success(),
        "crate-mode generic cpp_name transpilation failed:\n{}\n{}",
        String::from_utf8_lossy(&crate_result.stdout),
        String::from_utf8_lossy(&crate_result.stderr)
    );
    let crate_cpp = crate_output.join("cpp_name_generic_crate.cppm");
    let crate_generated =
        std::fs::read_to_string(&crate_cpp).expect("read crate-mode generic output");
    assert!(crate_generated.contains("make_serializable_proxy("));
    assert_eq!(crate_generated.matches("make_sink_proxy(").count(), 6);
    assert_eq!(crate_generated.matches("make_source_proxy(").count(), 6);
    for signature in [
        "make_serializable_proxy()",
        "make_serializable_proxy(const T& _value)",
        "make_sink_proxy(BufferSink* _sink)",
        "make_sink_proxy(FdSink* _sink)",
        "make_source_proxy(BufferSource* _source)",
        "make_source_proxy(FdSource* _source)",
    ] {
        assert!(
            crate_generated.contains(signature),
            "crate mode lost exact historical signature `{signature}`:\n{crate_generated}"
        );
    }
    assert!(!crate_generated.contains("make_serializable_proxy_default"));
    assert!(!crate_generated.contains("make_serializable_proxy_copy"));

    let Some(clang) = find_clang() else {
        eprintln!("skipping generic cpp_name C++ runtime gate: no clang++ in PATH or CXX");
        return;
    };
    let importer = temp.path().join("generic_importer.cpp");
    std::fs::write(
        &importer,
        r#"
import cpp_name_generic_runtime;
int main() {
    int value = 5;
    if (rrr::make_serializable_proxy<int>() != 7) return 1;
    if (rrr::make_serializable_proxy<int>(value) != 9) return 2;
    if (rrr::route_default() != 7) return 3;
    if (rrr::route_copy(value) != 9) return 4;
    if (rrr::route_copy_bare(value) != 9) return 5;
    if (rrr::make_sink_proxy(static_cast<rrr::BufferSink*>(nullptr)) != 11) return 6;
    if (rrr::make_sink_proxy(static_cast<rrr::FdSink*>(nullptr)) != 12) return 7;
    if (rrr::make_source_proxy(static_cast<rrr::BufferSource*>(nullptr)) != 13) return 8;
    if (rrr::make_source_proxy(static_cast<rrr::FdSource*>(nullptr)) != 14) return 9;
    if (rrr::route_sink_buffer(nullptr) != 11) return 10;
    if (rrr::route_sink_fd(nullptr) != 12) return 11;
    if (rrr::route_source_buffer(nullptr) != 13) return 12;
    if (rrr::route_source_fd(nullptr) != 14) return 13;
    return 0;
}
"#,
    )
    .expect("write generic C++ importer");
    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include");
    let pcm = temp.path().join("generic.pcm");
    let module_object = temp.path().join("generic.o");
    let importer_object = temp.path().join("generic_importer.o");
    let binary = temp.path().join("generic_runtime");
    let precompile = Command::new(&clang)
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"])
        .arg("-I")
        .arg(&include)
        .args(["-x", "c++-module", "--precompile"])
        .arg(&cpp)
        .arg("-o")
        .arg(&pcm)
        .output()
        .expect("precompile generic cpp_name module");
    assert!(
        precompile.status.success(),
        "generic cpp_name module failed Clang precompile:\n{}",
        String::from_utf8_lossy(&precompile.stderr)
    );
    for (label, input, object, language) in [
        ("module", &cpp, &module_object, "c++-module"),
        ("importer", &importer, &importer_object, "c++"),
    ] {
        let compile = Command::new(&clang)
            .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"])
            .arg("-I")
            .arg(&include)
            .arg("-x")
            .arg(language)
            .arg("-c")
            .arg(input)
            .arg(format!(
                "-fmodule-file=cpp_name_generic_runtime={}",
                pcm.display()
            ))
            .arg("-o")
            .arg(object)
            .output()
            .expect("compile generic cpp_name lane");
        assert!(
            compile.status.success(),
            "generic {label} object failed to compile:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
    }
    let link = Command::new(&clang)
        .args([&module_object, &importer_object])
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("link generic cpp_name runtime");
    assert!(
        link.status.success(),
        "generic cpp_name runtime failed to link:\n{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let run = Command::new(&binary)
        .output()
        .expect("run generic cpp_name binary");
    assert!(
        run.status.success(),
        "generic cpp_name runtime returned {run:?}"
    );
}

#[test]
fn generic_cpp_name_same_arity_fails_closed_in_direct_and_crate_modes() {
    let source = r#"
#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
pub fn first<T: 'static>(_value: &T) -> i32 { 1 }
#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
pub fn second<U: 'static>(_value: &U) -> i32 { 2 }
"#;
    let direct_temp = tempfile::tempdir().expect("direct tempdir");
    let rust = direct_temp.path().join("same_arity.rs");
    let cpp = direct_temp.path().join("same_arity.cppm");
    std::fs::write(&rust, source).unwrap();
    let direct = run_transpiler(&rust, &cpp, &["-m", "same_arity"]);
    assert!(
        !direct.status.success(),
        "accepted same-arity generic overloads"
    );
    assert!(
        String::from_utf8_lossy(&direct.stderr).contains("generic cpp_name overload collision"),
        "unexpected direct same-arity diagnostic:\n{}",
        String::from_utf8_lossy(&direct.stderr)
    );
    assert!(!cpp.exists(), "direct rejection created partial output");

    let crate_temp = tempfile::tempdir().expect("crate tempdir");
    std::fs::create_dir(crate_temp.path().join("src")).unwrap();
    let manifest = crate_temp.path().join("Cargo.toml");
    std::fs::write(
        &manifest,
        "[package]\nname = \"same_arity\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(crate_temp.path().join("src/lib.rs"), source).unwrap();
    let output_dir = crate_temp.path().join("cpp_out");
    let crate_result = run_crate_transpiler(&manifest, &output_dir);
    assert!(
        !crate_result.status.success(),
        "crate mode accepted same-arity generic overloads"
    );
    assert!(
        String::from_utf8_lossy(&crate_result.stderr)
            .contains("generic cpp_name overload collision"),
        "unexpected crate same-arity diagnostic:\n{}",
        String::from_utf8_lossy(&crate_result.stderr)
    );
    assert!(
        !output_dir.exists(),
        "crate same-arity rejection created partial output"
    );

    let void_source = direct_temp.path().join("void_arity.rs");
    let void_map = direct_temp.path().join("void_arity.toml");
    let void_cpp = direct_temp.path().join("void_arity.cppm");
    std::fs::write(
        &void_source,
        r#"
pub struct Marker;
#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
pub fn default<T: 'static>() -> i32 { 1 }
#[cfg_attr(any(), cpp_name(make_serializable_proxy))]
pub fn mapped_void<T: 'static>(_value: Marker) -> i32 { 2 }
"#,
    )
    .unwrap();
    std::fs::write(&void_map, "Marker = \"void\"\n").unwrap();
    let void_result = run_transpiler(
        &void_source,
        &void_cpp,
        &[
            "-m",
            "void_arity",
            "--type-map",
            void_map.to_str().expect("UTF-8 temporary type map path"),
        ],
    );
    assert!(!void_result.status.success(), "accepted bare-void arity");
    assert!(
        String::from_utf8_lossy(&void_result.stderr).contains("bare `void`"),
        "unexpected bare-void arity diagnostic:\n{}",
        String::from_utf8_lossy(&void_result.stderr)
    );
    assert!(!void_cpp.exists(), "bare-void rejection created output");
}

#[test]
fn unsafe_cpp_name_rejects_unsupported_forms_atomically_in_direct_and_crate_modes() {
    for (label, source, expected) in [
        (
            "private-unsafe",
            "#[cfg_attr(any(), cpp_name(make_proxy))] unsafe fn make(value: *mut i32) -> i32 { *value }",
            "must be public",
        ),
        (
            "unsafe-method",
            "struct Host; impl Host { #[cfg_attr(any(), cpp_name(make_proxy))] pub unsafe fn make(value: *mut i32) -> i32 { *value } }",
            "supported only on a crate-file root free function",
        ),
        (
            "unsafe-trait-method",
            "trait Host { #[cfg_attr(any(), cpp_name(make_proxy))] unsafe fn make(value: *mut i32) -> i32; }",
            "supported only on a crate-file root free function",
        ),
        (
            "foreign-declaration",
            "unsafe extern \"C\" { #[cfg_attr(any(), cpp_name(make_proxy))] fn make(value: *mut i32) -> i32; }",
            "supported only on a crate-file root free function",
        ),
        (
            "unsafe-extern-definition",
            "#[cfg_attr(any(), cpp_name(make_proxy))] pub unsafe extern \"C\" fn make(value: *mut i32) -> i32 { *value }",
            "must be an ordinary Rust free function",
        ),
        (
            "unrelated-companion-allow",
            "#[cfg_attr(any(), cpp_name(make_proxy))] #[allow(dead_code)] pub fn make(value: i32) -> i32 { value }",
            "exact #[allow(unsafe_code)]",
        ),
        (
            "mixed-companion-allow",
            "#[cfg_attr(any(), cpp_name(make_proxy))] #[allow(unsafe_code, dead_code)] pub unsafe fn make(value: *mut i32) -> i32 { *value }",
            "exact #[allow(unsafe_code)]",
        ),
    ] {
        let direct = tempfile::tempdir().expect("direct unsafe-negative tempdir");
        let rust = direct.path().join("negative.rs");
        let cpp = direct.path().join("negative.cppm");
        std::fs::write(&rust, source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "unsafe_negative"]);
        assert!(
            !result.status.success(),
            "accepted direct unsafe form {label}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "direct unsafe form {label} failed for the wrong reason:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "direct unsafe form {label} created output");

        let crate_root = tempfile::tempdir().expect("crate unsafe-negative tempdir");
        std::fs::create_dir_all(crate_root.path().join("src")).unwrap();
        let manifest = crate_root.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!("[package]\nname='unsafe_negative_{label}'\nversion='0.1.0'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(crate_root.path().join("src/lib.rs"), source).unwrap();
        let output = crate_root.path().join("existing-output");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("sentinel.keep"), b"unsafe-form\n").unwrap();
        let result = run_crate_transpiler(&manifest, &output);
        assert!(
            !result.status.success(),
            "accepted crate unsafe form {label}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "crate unsafe form {label} failed for the wrong reason:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            std::fs::read(output.join("sentinel.keep")).unwrap(),
            b"unsafe-form\n"
        );
        let entries = std::fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![std::ffi::OsString::from("sentinel.keep")]);
    }
}

#[test]
fn cpp_name_trait_member_dispatch_marker_is_exact_and_atomic() {
    for (label, marker_surface, expected) in [
        (
            "active",
            "#[cpp_trait_member_dispatch] trait SinkBase {}",
            "unaudited attribute",
        ),
        (
            "wrong-predicate",
            "#[cfg_attr(all(), cpp_trait_member_dispatch)] trait SinkBase {}",
            "unaudited attribute",
        ),
        (
            "arguments",
            "#[cfg_attr(any(), cpp_trait_member_dispatch(extra))] trait SinkBase {}",
            "unaudited attribute",
        ),
        (
            "qualified",
            "#[cfg_attr(any(), maker::cpp_trait_member_dispatch)] trait SinkBase {}",
            "unaudited attribute",
        ),
        (
            "extra-payload",
            "#[cfg_attr(any(), cpp_trait_member_dispatch, allow(dead_code))] trait SinkBase {}",
            "unaudited attribute",
        ),
        (
            "import-shadow",
            "use maker::cpp_trait_member_dispatch;\n#[cfg_attr(any(), cpp_trait_member_dispatch)] trait SinkBase {}",
            "can shadow an audited compiler-owned macro",
        ),
        (
            "macro-shadow",
            "macro_rules! cpp_trait_member_dispatch { () => {} }\n#[cfg_attr(any(), cpp_trait_member_dispatch)] trait SinkBase {}",
            "shadows an audited compiler-owned macro",
        ),
    ] {
        let source = format!(
            "{marker_surface}\n#[cfg_attr(any(), cpp_name(make_proxy))]\npub fn make_proxy_i32(value: i32) -> i32 {{ value }}\n"
        );
        let direct = tempfile::tempdir().expect("direct trait-marker tempdir");
        let rust = direct.path().join("marker.rs");
        let cpp = direct.path().join("marker.cppm");
        std::fs::write(&rust, &source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "trait_marker_negative"]);
        assert!(
            !result.status.success(),
            "accepted direct marker case {label}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "direct marker case {label} failed for the wrong reason:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "direct marker case {label} created output");

        let crate_root = tempfile::tempdir().expect("crate trait-marker tempdir");
        std::fs::create_dir_all(crate_root.path().join("src")).unwrap();
        let manifest = crate_root.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!("[package]\nname='trait_marker_{label}'\nversion='0.1.0'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(crate_root.path().join("src/lib.rs"), &source).unwrap();
        let output = crate_root.path().join("existing-output");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("sentinel.keep"), b"trait-marker\n").unwrap();
        let result = run_crate_transpiler(&manifest, &output);
        assert!(
            !result.status.success(),
            "accepted crate marker case {label}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "crate marker case {label} failed for the wrong reason:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            std::fs::read(output.join("sentinel.keep")).unwrap(),
            b"trait-marker\n"
        );
        assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
    }
}

#[test]
fn cpp_name_preflight_rejects_moduleless_and_signature_collisions() {
    let temp = tempfile::tempdir().expect("tempdir");

    let moduleless = temp.path().join("moduleless.rs");
    std::fs::write(
        &moduleless,
        "#[cfg_attr(any(), cpp_name(overloaded))] pub fn first(value: i32) -> i32 { value }",
    )
    .unwrap();
    let moduleless_cpp = temp.path().join("moduleless.cpp");
    let result = run_transpiler(&moduleless, &moduleless_cpp, &[]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("requires named-module or crate-mode"),
        "unexpected moduleless diagnostic:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!moduleless_cpp.exists());

    let expanded_cpp = temp.path().join("expanded.cppm");
    let result = run_transpiler(&moduleless, &expanded_cpp, &["-m", "expanded", "--expand"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("does not support --expand"),
        "unexpected --expand diagnostic:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!expanded_cpp.exists());

    for (label, source) in [
        (
            "direct",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(other: i32) -> bool { other != 0 }
"#,
        ),
        (
            "alias-equivalent",
            r#"
type Left = i32;
type Right = i32;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: Left) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(other: Right) -> i32 { other }
"#,
        ),
        (
            "ordinary-target",
            r#"
pub fn overloaded(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn distinct(other: i32) -> i32 { other }
"#,
        ),
        (
            "compiler-owned-import-alias-equivalent",
            r#"
use std::sync::Arc as Left;
use std::sync::Arc as Right;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: &Left<i32>) -> i32 { **value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(other: &Right<i32>) -> i32 { **other }
"#,
        ),
        (
            "exact-definition-auto-alias",
            r#"
use std::sync::Arc;
type Left = Arc<i32>;
type Right = Arc<bool>;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: Left) -> i32 { **value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(other: Right) -> i32 { **other as i32 }
"#,
        ),
    ] {
        let rust = temp.path().join(format!("collision-{label}.rs"));
        let cpp = temp.path().join(format!("collision-{label}.cppm"));
        std::fs::write(&rust, source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "collision"]);
        assert!(!result.status.success(), "accepted {label} collision");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("cpp_name overload collision"),
            "unexpected {label} collision diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "collision left partial output: {label}");
    }

    let bare_fn_alias = temp.path().join("forward-definition-divergence.rs");
    let bare_fn_alias_cpp = temp.path().join("forward-definition-divergence.cppm");
    std::fs::write(
        &bare_fn_alias,
        r#"
type Left = fn(i32) -> i32;
type Right = fn(bool) -> i32;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: Left) -> i32 { value(1) }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: Right) -> i32 { value(true) }
"#,
    )
    .unwrap();
    let result = run_transpiler(
        &bare_fn_alias,
        &bare_fn_alias_cpp,
        &["-m", "forward_definition_divergence"],
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("cannot prove declaration/definition compatibility"),
        "unexpected declaration/definition diagnostic:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        !bare_fn_alias_cpp.exists(),
        "declaration/definition rejection left partial output"
    );
}

#[test]
fn cpp_name_proves_target_typedef_and_const_expression_identity_before_output() {
    let temp = tempfile::tempdir().expect("tempdir");

    for (label, source, expected) in [
        (
            "usize-u64",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn word(value: usize) -> usize { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn fixed(value: u64) -> u64 { value }
"#,
            "cpp_name overload collision",
        ),
        (
            "isize-i64",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn word(value: isize) -> isize { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn fixed(value: i64) -> i64 { value }
"#,
            "cpp_name overload collision",
        ),
        (
            "root-alias-target-typedef",
            r#"
type WordBase = usize;
type Word = WordBase;
type FixedBase = u64;
type Fixed = FixedBase;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn word(value: Word) -> usize { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn fixed(value: Fixed) -> u64 { value }
"#,
            "cpp_name overload collision",
        ),
        (
            "nested-target-typedef",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn word(value: &usize) -> usize { *value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn fixed(value: &u64) -> u64 { *value }
"#,
            "cpp_name overload collision",
        ),
        (
            "equal-root-consts",
            r#"
pub const LEFT: usize = 4;
pub const RIGHT: usize = 4;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(value: [i32; LEFT]) -> i32 { value[0] }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(value: [i32; RIGHT]) -> i32 { value[0] }
"#,
            "cpp_name overload collision",
        ),
        (
            "equal-root-const-arithmetic",
            r#"
pub const BASE: usize = 2;
pub const LEFT: usize = BASE * 2;
pub const RIGHT: usize = 1 << 2;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(value: [i32; crate::LEFT]) -> i32 { value[0] }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(value: [i32; self::RIGHT]) -> i32 { value[0] }
"#,
            "cpp_name overload collision",
        ),
        (
            "equal-root-const-generics",
            r#"
pub const LEFT: usize = 4;
pub const RIGHT: usize = 2 + 2;
pub struct Width<const N: usize>;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(value: Width<LEFT>) -> Width<LEFT> { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(value: Width<RIGHT>) -> Width<RIGHT> { value }
"#,
            "cpp_name overload collision",
        ),
        (
            "primitive-const-same-name",
            r#"
#![allow(non_upper_case_globals)]
pub const usize: usize = 4;
pub struct Wrap<T> { pub value: T }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn primitive_short(value: Wrap<usize>) -> usize { value.value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn primitive_qualified(value: Wrap<std::primitive::usize>) -> usize { value.value }
"#,
            "cpp_name overload collision",
        ),
        (
            "type-const-same-name",
            r#"
pub type SAME = i32;
pub const SAME: usize = 4;
pub struct Wrap<T> { pub value: T }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn alias_value(value: Wrap<SAME>) -> i32 { value.value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn primitive_value(value: Wrap<i32>) -> i32 { value.value }
"#,
            "competes with a source or compiler-owned type-namespace binding",
        ),
        (
            "struct-const-same-name",
            r#"
pub struct SAME { pub value: i32 }
pub const SAME: usize = 4;
pub struct Wrap<T> { pub value: T }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn named(value: Wrap<SAME>) -> i32 { value.value.value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn primitive(value: Wrap<i32>) -> i32 { value.value }
"#,
            "competes with a source or compiler-owned type-namespace binding",
        ),
        (
            "prelude-type-const-same-name",
            r#"
#![allow(non_upper_case_globals)]
pub const String: usize = 4;
pub struct Wrap<T> { pub value: T }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn prelude_value(value: Wrap<String>) -> String { value.value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn qualified_value(value: Wrap<std::string::String>) -> std::string::String { value.value }
"#,
            "competes with a source or compiler-owned type-namespace binding",
        ),
        (
            "unsupported-symbolic-const",
            r#"
pub const fn width() -> usize { 4 }
pub const LEFT: usize = width();
pub const RIGHT: usize = 5;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(value: [i32; LEFT]) -> i32 { value[0] }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(value: [i32; RIGHT]) -> i32 { value[0] }
"#,
            "cpp_name cannot prove overload identity from const expression",
        ),
    ] {
        let rust = temp.path().join(format!("identity-{label}.rs"));
        let rust_library = temp.path().join(format!("libidentity_{label}.rlib"));
        let cpp = temp.path().join(format!("identity-{label}.cppm"));
        std::fs::write(&rust, source).unwrap();

        let rustc = Command::new("rustc")
            .args(["--edition=2024", "--crate-type=lib"])
            .arg(&rust)
            .arg("-o")
            .arg(&rust_library)
            .output()
            .expect("run rustc identity fixture");
        assert!(
            rustc.status.success(),
            "identity fixture {label} is not rustc-valid:\n{}",
            String::from_utf8_lossy(&rustc.stderr)
        );

        let result = run_transpiler(&rust, &cpp, &["-m", "identity"]);
        assert!(
            !result.status.success(),
            "accepted identity fixture {label}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "{label} rejection left partial output");
    }

    let distinct = temp.path().join("distinct-const-arithmetic.rs");
    let distinct_cpp = temp.path().join("distinct-const-arithmetic.cppm");
    std::fs::write(
        &distinct,
        r#"
pub const BASE: usize = 2;
pub const LEFT: usize = BASE * 2;
pub const RIGHT: usize = (LEFT << 1) - 1;
pub struct Width<const N: usize>;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(value: [i32; LEFT]) -> i32 { value[0] }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(value: [i32; RIGHT]) -> i32 { value[0] }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn width_left(_value: Width<LEFT>) -> usize { LEFT }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn width_right(_value: Width<RIGHT>) -> usize { RIGHT }
#[cfg_attr(any(), cpp_name(explicit_overload))]
pub fn explicit_width_left(_value: Width<{ LEFT }>) -> usize { LEFT }
#[cfg_attr(any(), cpp_name(explicit_overload))]
pub fn explicit_width_right(_value: Width<{ RIGHT }>) -> usize { RIGHT }
#[cfg_attr(any(), cpp_name(target_overload))]
pub fn target_word(value: usize) -> usize { value }
#[cfg_attr(any(), cpp_name(target_overload))]
pub fn signed_fixed(value: i64) -> i64 { value }
"#,
    )
    .unwrap();
    let result = run_transpiler(
        &distinct,
        &distinct_cpp,
        &["-m", "distinct_const_arithmetic"],
    );
    assert!(
        result.status.success(),
        "rejected proven-distinct const expressions:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(distinct_cpp.exists());

    if let Some(clang) = find_clang() {
        let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("include");
        let pcm = temp.path().join("distinct-const-arithmetic.pcm");
        let precompile = Command::new(&clang)
            .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"])
            .arg("-I")
            .arg(&include)
            .args(["-x", "c++-module", "--precompile"])
            .arg(&distinct_cpp)
            .arg("-o")
            .arg(&pcm)
            .output()
            .expect("precompile proven-distinct const module");
        assert!(
            precompile.status.success(),
            "proven-distinct const module failed Clang precompile:\n{}",
            String::from_utf8_lossy(&precompile.stderr)
        );
    }
}

#[test]
fn cpp_name_direct_mode_rejects_opaque_item_and_attribute_expansion_before_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("hidden.rs"),
        "pub fn overloaded(value: bool) -> i32 { value as i32 }",
    )
    .unwrap();
    std::fs::write(temp.path().join("hidden_expr.rs"), "crate::renamed(41)").unwrap();
    for (label, source, expected) in [
        (
            "include",
            r#"
include!("hidden.rs");
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            "unexpanded item macro",
        ),
        (
            "expression-include",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
pub fn route_hidden() -> i32 { include!("hidden_expr.rs") }
"#,
            "unexpanded macro invocation",
        ),
        (
            "function-like-proc-macro",
            r#"
use maker::call_hidden;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
pub fn route_hidden() -> i32 { call_hidden!() }
"#,
            "unexpanded macro invocation",
        ),
        (
            "derive",
            r#"
#[derive(Clone)]
pub struct Host;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            "unaudited attribute",
        ),
        (
            "proc-attribute",
            r#"
#[make_overloaded]
pub struct Host;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            "unaudited attribute",
        ),
        (
            "cfg-proc-attribute",
            r#"
#[cfg_attr(not(any()), make_overloaded)]
pub struct Host;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            "unaudited attribute",
        ),
    ] {
        let rust = temp.path().join(format!("opaque-{label}.rs"));
        let cpp = temp.path().join(format!("opaque-{label}.cppm"));
        std::fs::write(&rust, source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "opaque_expansion"]);
        assert!(
            !result.status.success(),
            "accepted opaque {label} expansion"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "{label} rejection left partial output");
    }
}

#[test]
fn cpp_name_direct_mode_rejects_reexport_and_module_alias_parameter_bypasses() {
    let temp = tempfile::tempdir().expect("tempdir");
    for (label, source) in [
        (
            "pub-use",
            r#"
mod types { pub type A = i32; pub type B = i32; }
pub use types::{A, B};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: A) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: B) -> i32 { value }
"#,
        ),
        (
            "module-alias",
            r#"
mod types { pub type A = i32; pub type B = i32; }
use types as left;
use types as right;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: left::A) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: right::B) -> i32 { value }
"#,
        ),
        (
            "nested-function-pointer",
            r#"
mod types { pub type A = i32; pub type B = i32; }
pub use types::{A, B};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: fn(A) -> i32) -> i32 { value(1) }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: fn(B) -> i32) -> i32 { value(1) }
"#,
        ),
    ] {
        let rust = temp.path().join(format!("alias-bypass-{label}.rs"));
        let cpp = temp.path().join(format!("alias-bypass-{label}.cppm"));
        std::fs::write(&rust, source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "alias_bypass"]);
        assert!(!result.status.success(), "accepted {label} alias bypass");
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("cpp_name cannot prove overload identity"),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "{label} rejection left partial output");
    }

    for root in ["rusty", "std", "core", "alloc"] {
        let label = format!("local-{root}");
        let source = format!(
            r#"
#![no_std]
mod {root} {{ pub type A = i32; pub type B = i32; }}
use {root}::{{A, B}};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: A) -> i32 {{ value }}
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: B) -> i32 {{ value }}
"#
        );
        let rust = temp.path().join(format!("alias-bypass-{label}.rs"));
        let cpp = temp.path().join(format!("alias-bypass-{label}.cppm"));
        std::fs::write(&rust, source).unwrap();
        let result = run_transpiler(&rust, &cpp, &["-m", "alias_bypass"]);
        assert!(
            !result.status.success(),
            "accepted source-owned {root} alias bypass"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("cpp_name cannot prove overload identity"),
            "unexpected local-{root} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!cpp.exists(), "local-{root} rejection left partial output");
    }

    let allowed = temp.path().join("unrelated-aliases.rs");
    let allowed_cpp = temp.path().join("unrelated-aliases.cppm");
    std::fs::write(
        &allowed,
        r#"
use std::sync::Arc;
mod types { pub type A = i32; pub type B = i32; }
pub use types::{A, B};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: bool) -> i32 { value as i32 }
"#,
    )
    .unwrap();
    let result = run_transpiler(&allowed, &allowed_cpp, &["-m", "unrelated_aliases"]);
    assert!(
        result.status.success(),
        "unrelated imports were rejected:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(allowed_cpp.exists());
}

#[test]
fn cpp_name_crate_mode_rejects_alias_bypasses_before_output() {
    for (label, source, expected) in [
        (
            "pub-use",
            r#"
mod types { pub type A = i32; pub type B = i32; }
pub use types::{A, B};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: A) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: B) -> i32 { value }
"#,
            "cpp_name cannot prove overload identity",
        ),
        (
            "module-alias",
            r#"
mod types { pub type A = i32; pub type B = i32; }
use types as left;
use types as right;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: left::A) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: right::B) -> i32 { value }
"#,
            "cpp_name cannot prove overload identity",
        ),
        (
            "exact-definition-auto-alias",
            r#"
use std::sync::Arc;
type Left = Arc<i32>;
type Right = Arc<bool>;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: Left) -> i32 { **value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: Right) -> i32 { **value as i32 }
"#,
            "cpp_name overload collision",
        ),
        (
            "forward-definition-divergence",
            r#"
type Left = fn(i32) -> i32;
type Right = fn(bool) -> i32;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: Left) -> i32 { value(1) }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: Right) -> i32 { value(true) }
"#,
            "cannot prove declaration/definition compatibility",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"alias_bypass_{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        std::fs::write(src.join("lib.rs"), source).unwrap();
        let output_dir = temp.path().join("cpp_out");
        let result = run_crate_transpiler(&temp.path().join("Cargo.toml"), &output_dir);
        assert!(
            !result.status.success(),
            "crate mode accepted {label} bypass"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "unexpected crate-mode {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "crate-mode {label} rejection created output"
        );
    }

    for root in ["rusty", "std", "core", "alloc"] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"alias_bypass_{root}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        std::fs::write(
            src.join("lib.rs"),
            format!(
                r#"
#![no_std]
mod {root} {{ pub type A = i32; pub type B = i32; }}
use {root}::{{A, B}};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: A) -> i32 {{ value }}
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: B) -> i32 {{ value }}
"#
            ),
        )
        .unwrap();
        let output_dir = temp.path().join("cpp_out");
        let result = run_crate_transpiler(&temp.path().join("Cargo.toml"), &output_dir);
        assert!(
            !result.status.success(),
            "crate mode accepted source-owned {root} alias bypass"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr)
                .contains("cpp_name cannot prove overload identity"),
            "unexpected crate-mode local-{root} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "crate-mode local-{root} rejection created output"
        );
    }

    let temp = tempfile::tempdir().expect("tempdir");
    let src = temp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"unrelated_aliases\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        src.join("lib.rs"),
        r#"
use std::sync::Arc;
mod types { pub type A = i32; pub type B = i32; }
pub use types::{A, B};
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: bool) -> i32 { value as i32 }
"#,
    )
    .unwrap();
    let output_dir = temp.path().join("cpp_out");
    let result = run_crate_transpiler(&temp.path().join("Cargo.toml"), &output_dir);
    assert!(
        result.status.success(),
        "crate mode rejected unrelated aliases:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output_dir.join("unrelated_aliases.cppm").exists());
}

#[test]
fn cpp_name_crate_mode_rejects_opaque_expansion_before_output() {
    for (label, source, hidden, expected) in [
        (
            "include",
            r#"
include!("hidden.rs");
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            Some("pub fn overloaded(value: bool) -> i32 { value as i32 }"),
            "unexpanded item macro",
        ),
        (
            "expression-include",
            r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
pub fn route_hidden() -> i32 { include!("hidden.rs") }
"#,
            Some("crate::renamed(41)"),
            "unexpanded macro invocation",
        ),
        (
            "derive",
            r#"
#[derive(Clone)]
pub struct Host;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value }
"#,
            None,
            "unaudited attribute",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        std::fs::create_dir(&src).unwrap();
        let manifest = temp.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!(
                "[package]\nname = \"opaque_{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        )
        .unwrap();
        std::fs::write(src.join("lib.rs"), source).unwrap();
        if let Some(hidden) = hidden {
            std::fs::write(src.join("hidden.rs"), hidden).unwrap();
        }
        let cargo_check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join("rust-target"))
            .output()
            .expect("cargo-check opaque expansion fixture");
        assert!(
            cargo_check.status.success(),
            "{label} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let output_dir = temp.path().join("cpp_out");
        let result = run_crate_transpiler(&manifest, &output_dir);
        assert!(!result.status.success(), "accepted crate {label} expansion");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "unexpected crate {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "crate {label} rejection created partial output"
        );
    }

    for (label, host_attr) in [
        ("proc_attribute", "#[make_overloaded]"),
        (
            "cfg_proc_attribute",
            "#[cfg_attr(not(any()), make_overloaded)]",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        let maker_src = temp.path().join("maker/src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&maker_src).unwrap();
        let manifest = temp.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!(
                "[package]\nname = \"opaque_{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmaker = {{ path = \"maker\" }}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            temp.path().join("maker/Cargo.toml"),
            "[package]\nname = \"maker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nproc-macro = true\n",
        )
        .unwrap();
        std::fs::write(
            maker_src.join("lib.rs"),
            r#"
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn make_overloaded(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut output = item;
    output.extend(
        "pub fn overloaded(value: bool) -> i32 { value as i32 }"
            .parse::<TokenStream>()
            .unwrap(),
    );
    output
}
"#,
        )
        .unwrap();
        std::fs::write(
            src.join("lib.rs"),
            format!(
                r#"
use maker::make_overloaded;

{host_attr}
pub struct Host;

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 {{ value }}
"#
            ),
        )
        .unwrap();

        // This is a real Cargo-valid procedural-attribute expansion which
        // creates the hidden root overload the syntax-only preflight cannot
        // audit. Both direct and cfg_attr-mediated spellings must fail closed.
        let cargo_check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join("rust-target"))
            .output()
            .expect("cargo-check proc-attribute fixture");
        assert!(
            cargo_check.status.success(),
            "{label} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let output_dir = temp.path().join("cpp_out");
        let result = run_crate_transpiler(&manifest, &output_dir);
        assert!(
            !result.status.success(),
            "accepted real crate {label} expansion"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("unaudited attribute"),
            "unexpected real crate {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "real crate {label} rejection created partial output"
        );
    }

    for (label, route_source) in [
        (
            "function_proc_macro",
            "pub fn route_hidden() -> i32 { call_hidden!() }",
        ),
        (
            "local_wrapper_to_function_proc_macro",
            "macro_rules! wrapper { () => { call_hidden!() }; }\npub fn route_hidden() -> i32 { wrapper!() }",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        let maker_src = temp.path().join("maker/src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&maker_src).unwrap();
        let manifest = temp.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!(
                "[package]\nname = \"opaque_{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmaker = {{ path = \"maker\" }}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            temp.path().join("maker/Cargo.toml"),
            "[package]\nname = \"maker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nproc-macro = true\n",
        )
        .unwrap();
        std::fs::write(
            maker_src.join("lib.rs"),
            r#"
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn call_hidden(_input: TokenStream) -> TokenStream {
    "crate::renamed(41)".parse::<TokenStream>().unwrap()
}
"#,
        )
        .unwrap();
        let rust_source = format!(
            r#"
use maker::call_hidden;

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 {{ value + 1 }}

{route_source}
"#
        );
        let rust = src.join("lib.rs");
        std::fs::write(&rust, rust_source).unwrap();

        // Prove that this is a real Cargo-valid proc-macro expansion which
        // hides the source-owned function identity from syntax-only auditing.
        let cargo_check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join("rust-target"))
            .output()
            .expect("cargo-check function-like proc-macro fixture");
        assert!(
            cargo_check.status.success(),
            "{label} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let direct_output = temp.path().join("direct.cppm");
        let direct = run_transpiler(&rust, &direct_output, &["-m", "opaque_function_macro"]);
        assert!(!direct.status.success(), "direct mode accepted {label}");
        assert!(
            String::from_utf8_lossy(&direct.stderr).contains("unexpanded macro invocation"),
            "unexpected direct {label} diagnostic:\n{}",
            String::from_utf8_lossy(&direct.stderr)
        );
        assert!(
            !direct_output.exists(),
            "direct {label} rejection created partial output"
        );

        let output_dir = temp.path().join("cpp_out");
        let crate_result = run_crate_transpiler(&manifest, &output_dir);
        assert!(
            !crate_result.status.success(),
            "crate mode accepted {label}"
        );
        assert!(
            String::from_utf8_lossy(&crate_result.stderr).contains("unexpanded macro invocation"),
            "unexpected crate {label} diagnostic:\n{}",
            String::from_utf8_lossy(&crate_result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "crate {label} rejection created partial output"
        );
    }
}

#[test]
fn cpp_name_crate_mode_audits_marker_free_sibling_expansions_before_output() {
    for (label, root_prefix, child_source, hidden_source, maker_source, expected) in [
        (
            "sibling_expression_include",
            "",
            "pub fn route_hidden() -> i32 { include!(\"hidden.inc\") }",
            Some("crate::renamed(41)"),
            None,
            "unexpanded macro invocation",
        ),
        (
            "sibling_local_wrapper_to_proc",
            "macro_rules! wrapper { () => { maker::call_hidden!() }; }",
            "pub fn route_hidden() -> i32 { wrapper!() }",
            None,
            Some(
                r#"
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn call_hidden(_input: TokenStream) -> TokenStream {
    "crate::renamed(41)".parse::<TokenStream>().unwrap()
}
"#,
            ),
            "unexpanded macro invocation",
        ),
        (
            "sibling_proc_attribute",
            "",
            r#"
use maker::make_route;

#[make_route]
pub struct Host;
"#,
            None,
            Some(
                r#"
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn make_route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut output = item;
    output.extend(
        "pub fn route_hidden() -> i32 { crate::renamed(41) }"
            .parse::<TokenStream>()
            .unwrap(),
    );
    output
}
"#,
            ),
            "unaudited attribute",
        ),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let manifest = temp.path().join("Cargo.toml");
        let dependency = if maker_source.is_some() {
            "\n[dependencies]\nmaker = { path = \"maker\" }\n"
        } else {
            ""
        };
        std::fs::write(
            &manifest,
            format!(
                "[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{dependency}"
            ),
        )
        .unwrap();
        std::fs::write(
            src.join("lib.rs"),
            format!(
                r#"
{root_prefix}
mod child;
pub use child::route_hidden;

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 {{ value + 1 }}
"#
            ),
        )
        .unwrap();
        std::fs::write(src.join("child.rs"), child_source).unwrap();
        if let Some(hidden_source) = hidden_source {
            std::fs::write(src.join("hidden.inc"), hidden_source).unwrap();
        }
        if let Some(maker_source) = maker_source {
            let maker_src = temp.path().join("maker/src");
            std::fs::create_dir_all(&maker_src).unwrap();
            std::fs::write(
                temp.path().join("maker/Cargo.toml"),
                "[package]\nname = \"maker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nproc-macro = true\n",
            )
            .unwrap();
            std::fs::write(maker_src.join("lib.rs"), maker_source).unwrap();
        }

        // Each bypass is genuine Rust/Cargo input: the opaque expansion adds
        // or calls the renamed root function only after syntax preflight.
        let cargo_check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join("rust-target"))
            .output()
            .expect("cargo-check marker-free sibling expansion fixture");
        assert!(
            cargo_check.status.success(),
            "{label} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let output_dir = temp.path().join("cpp_out");
        let result = run_crate_transpiler(&manifest, &output_dir);
        assert!(!result.status.success(), "crate mode accepted {label}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "{label} rejection created partial crate output"
        );
    }
}

#[test]
fn cpp_name_transitive_dependency_failure_is_atomic_for_fresh_and_existing_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    let middle = temp.path().join("middle");
    let leaf = temp.path().join("leaf");
    let maker = temp.path().join("maker");
    for package in [&root, &middle, &leaf, &maker] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    // root -> middle -> leaf makes the bad cpp_name contract genuinely
    // transitive. middle also invokes a local proc-macro dependency carrying
    // its own bad marker: that host-only package must be pruned from the C++
    // graph while the ordinary transitive target library remains selected.
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"atomic_root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\natomic_middle = { path = \"../middle\" }\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn root() -> i32 { atomic_middle::middle() }\n",
    )
    .unwrap();
    std::fs::write(
        middle.join("Cargo.toml"),
        "[package]\nname = \"atomic_middle\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\natomic_leaf = { path = \"../leaf\" }\natomic_maker = { path = \"../maker\" }\n",
    )
    .unwrap();
    std::fs::write(
        middle.join("src/lib.rs"),
        "atomic_maker::make_value!();\npub fn middle() -> i32 { made() + atomic_leaf::first(1) }\n",
    )
    .unwrap();
    std::fs::write(
        leaf.join("Cargo.toml"),
        "[package]\nname = \"atomic_leaf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        leaf.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn first(value: i32) -> i32 { value }

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();
    std::fs::write(
        maker.join("Cargo.toml"),
        "[package]\nname = \"atomic_maker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nproc-macro = true\n",
    )
    .unwrap();
    std::fs::write(
        maker.join("src/lib.rs"),
        r#"
extern crate proc_macro;
use proc_macro::TokenStream;

#[cfg_attr(any(), cpp_name(macro_side_overload))]
fn macro_side_first(value: i32) -> i32 { value }

#[cfg_attr(any(), cpp_name(macro_side_overload))]
fn macro_side_second(value: i32) -> i32 { value + 1 }

#[proc_macro]
pub fn make_value(_input: TokenStream) -> TokenStream {
    "pub fn made() -> i32 { 40 }".parse().unwrap()
}
"#,
    )
    .unwrap();

    let manifest = root.join("Cargo.toml");
    let cargo_check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(temp.path().join("rust-target"))
        .output()
        .expect("cargo-check transitive cpp_name fixture");
    assert!(
        cargo_check.status.success(),
        "transitive dependency fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let fresh_output = temp.path().join("fresh-cpp-output");
    let fresh = run_crate_transpiler(&manifest, &fresh_output);
    assert!(
        !fresh.status.success(),
        "accepted transitive dependency collision"
    );
    let fresh_stderr = String::from_utf8_lossy(&fresh.stderr);
    assert!(
        fresh_stderr
            .contains("cpp_name whole local-dependency closure preflight failed before output")
            && fresh_stderr.contains("cpp_name overload collision")
            && fresh_stderr.contains("leaf/Cargo.toml")
            && !fresh_stderr.contains("maker/Cargo.toml")
            && !fresh_stderr.contains("macro_side_overload"),
        "unexpected transitive collision diagnostic:\n{fresh_stderr}"
    );
    assert!(
        !fresh_output.exists(),
        "transitive failure created fresh output at {}",
        fresh_output.display()
    );

    let existing_output = temp.path().join("existing-cpp-output");
    std::fs::create_dir_all(&existing_output).unwrap();
    let sentinel = existing_output.join("sentinel.keep");
    let sentinel_bytes = b"preexisting output must remain byte-for-byte intact\n";
    std::fs::write(&sentinel, sentinel_bytes).unwrap();
    let existing = run_crate_transpiler(&manifest, &existing_output);
    assert!(
        !existing.status.success(),
        "accepted transitive dependency collision with existing output"
    );
    assert_eq!(std::fs::read(&sentinel).unwrap(), sentinel_bytes);
    let mut entries = std::fs::read_dir(&existing_output)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    entries.sort();
    assert_eq!(entries, vec![std::ffi::OsString::from("sentinel.keep")]);
}

#[test]
fn cpp_name_uses_one_cargo_selected_target_dependency_graph_atomically() {
    let rustc = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    let rustc_stdout = String::from_utf8_lossy(&rustc.stdout);
    let host = rustc_stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap_or_default();
    if host != "x86_64-unknown-linux-gnu" {
        eprintln!("skipping Linux target-selection gate on test host {host}");
        return;
    }

    let temp = tempfile::tempdir().expect("target-selection tempdir");
    let bad = temp.path().join("bad");
    std::fs::create_dir_all(bad.join("src")).unwrap();
    std::fs::write(
        bad.join("Cargo.toml"),
        "[package]\nname='bad_dep'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        bad.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(selected_overload))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(selected_overload))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();

    let bridge = temp.path().join("bridge");
    std::fs::create_dir_all(bridge.join("src")).unwrap();
    std::fs::write(
        bridge.join("Cargo.toml"),
        "[package]\nname='target_bridge'\nversion='0.1.0'\nedition='2024'\n[target.'cfg(target_os = \"linux\")'.dependencies]\nchosen={package='bad_dep',path='../bad'}\n",
    )
    .unwrap();
    std::fs::write(
        bridge.join("src/lib.rs"),
        "#[cfg(target_os=\"linux\")] pub fn bridge() -> i32 { chosen::first(1) }\n#[cfg(not(target_os=\"linux\"))] pub fn bridge() -> i32 { 0 }\n",
    )
    .unwrap();

    let cases = temp.path().join("cases");
    let selected_cases = [
        (
            "target_unix",
            "[target.'cfg(unix)'.dependencies]\nchosen={package='bad_dep',path='../../bad'}\n",
            "pub fn root() -> i32 { chosen::first(1) }\n",
        ),
        (
            "target_os_linux",
            "[target.'cfg(target_os = \"linux\")'.dependencies]\nchosen={package='bad_dep',path='../../bad'}\n",
            "pub fn root() -> i32 { chosen::first(1) }\n",
        ),
        (
            "target_literal",
            "[target.x86_64-unknown-linux-gnu.dependencies]\nchosen={package='bad_dep',path='../../bad'}\n",
            "pub fn root() -> i32 { chosen::first(1) }\n",
        ),
        (
            "feature_selected_optional",
            "[dependencies]\nchosen={package='bad_dep',path='../../bad',optional=true}\n[features]\ndefault=['chosen']\n",
            "pub fn root() -> i32 { chosen::first(1) }\n",
        ),
        (
            "selected_transitive_target",
            "[dependencies]\ntarget_bridge={path='../../bridge'}\n",
            "pub fn root() -> i32 { target_bridge::bridge() }\n",
        ),
    ];

    for (name, dependency_tables, root_source) in selected_cases {
        let root = cases.join(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n{dependency_tables}[workspace]\n"
            ),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), root_source).unwrap();
        let manifest = root.join("Cargo.toml");
        let check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join("cargo-target").join(name))
            .output()
            .expect("cargo-check selected target fixture");
        assert!(
            check.status.success(),
            "selected target fixture {name} is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );

        for existing in [false, true] {
            let lane = if existing { "existing" } else { "fresh" };
            let output_dir = root.join(format!("{lane}-output"));
            if existing {
                std::fs::create_dir_all(&output_dir).unwrap();
                std::fs::write(output_dir.join("sentinel.keep"), b"preserve-v9\n").unwrap();
            }
            let failure = run_crate_transpiler(&manifest, &output_dir);
            assert!(
                !failure.status.success(),
                "selected target cpp_name collision {name}/{lane} was accepted"
            );
            let stderr = String::from_utf8_lossy(&failure.stderr);
            assert!(
                stderr.contains(
                    "cpp_name whole local-dependency closure preflight failed before output"
                ) && stderr.contains("cpp_name overload collision")
                    && stderr.contains("bad"),
                "selected target {name}/{lane} failed for the wrong reason:\n{stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output_dir.join("sentinel.keep")).unwrap(),
                    b"preserve-v9\n"
                );
                let entries = std::fs::read_dir(&output_dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name())
                    .collect::<Vec<_>>();
                assert_eq!(entries, vec![std::ffi::OsString::from("sentinel.keep")]);
            } else {
                assert!(
                    !output_dir.exists(),
                    "selected target failure {name} created fresh output"
                );
            }
        }
    }

    // A selected, dependency-owned contract must use the same graph during
    // generation: the target-qualified alias is emitted recursively.
    let good = temp.path().join("good");
    std::fs::create_dir_all(good.join("src")).unwrap();
    std::fs::write(
        good.join("Cargo.toml"),
        "[package]\nname='good_dep'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        good.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(good_overload))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(good_overload))]
pub fn second(value: bool) -> i32 { if value { 1 } else { 0 } }
"#,
    )
    .unwrap();
    let good_root = temp.path().join("good-root");
    std::fs::create_dir_all(good_root.join("src")).unwrap();
    std::fs::write(
        good_root.join("Cargo.toml"),
        "[package]\nname='good_root'\nversion='0.1.0'\nedition='2024'\n[target.'cfg(unix)'.dependencies]\ngood_alias={package='good_dep',path='../good'}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        good_root.join("src/lib.rs"),
        "pub fn root() -> i32 { good_alias::first(1) }\n",
    )
    .unwrap();
    let good_output = good_root.join("output");
    let good_result = run_crate_transpiler(&good_root.join("Cargo.toml"), &good_output);
    assert!(
        good_result.status.success(),
        "selected target recursive generation failed:\n{}",
        String::from_utf8_lossy(&good_result.stderr)
    );
    assert!(
        good_output.join("good_alias/good_dep.cppm").is_file(),
        "selected target alias was not recursively generated"
    );

    // Cargo workspace inheritance and `package = ...` renaming must preserve
    // the source-visible dependency key all the way through generation.
    let workspace = temp.path().join("workspace");
    for member in ["root", "leaf"] {
        std::fs::create_dir_all(workspace.join(member).join("src")).unwrap();
    }
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers=['root','leaf']\nresolver='2'\n[workspace.dependencies]\nrenamed={package='workspace_leaf_pkg',path='leaf'}\n",
    )
    .unwrap();
    std::fs::write(
        workspace.join("leaf/Cargo.toml"),
        "[package]\nname='workspace_leaf_pkg'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        workspace.join("leaf/src/lib.rs"),
        std::fs::read_to_string(good.join("src/lib.rs")).unwrap(),
    )
    .unwrap();
    std::fs::write(
        workspace.join("root/Cargo.toml"),
        "[package]\nname='workspace_target_root'\nversion='0.1.0'\nedition='2024'\n[target.'cfg(target_os = \"linux\")'.dependencies]\nrenamed.workspace=true\n",
    )
    .unwrap();
    std::fs::write(
        workspace.join("root/src/lib.rs"),
        "pub fn root() -> i32 { renamed::first(1) }\n",
    )
    .unwrap();
    let workspace_output = workspace.join("output");
    let workspace_result =
        run_crate_transpiler(&workspace.join("root/Cargo.toml"), &workspace_output);
    assert!(
        workspace_result.status.success(),
        "workspace-renamed target generation failed:\n{}",
        String::from_utf8_lossy(&workspace_result.stderr)
    );
    assert!(
        workspace_output
            .join("renamed/workspace_leaf_pkg.cppm")
            .is_file(),
        "workspace package alias was not retained for recursive generation"
    );
    assert!(!workspace_output.join("workspace_leaf_pkg").exists());

    // Target- and feature-disabled marker crates are excluded by the same
    // graph in both preflight and generation.
    for (name, dependency_tables) in [
        (
            "unselected_target",
            "[target.'cfg(windows)'.dependencies]\nchosen={package='bad_dep',path='../../bad'}\n",
        ),
        (
            "unselected_optional",
            "[dependencies]\nchosen={package='bad_dep',path='../../bad',optional=true}\n[features]\ndefault=[]\n",
        ),
    ] {
        let root = cases.join(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n{dependency_tables}[workspace]\n"
            ),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn root() -> i32 { 0 }\n").unwrap();
        let output_dir = root.join("output");
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("sentinel.keep"), b"unselected-v9\n").unwrap();
        let success = run_crate_transpiler(&root.join("Cargo.toml"), &output_dir);
        assert!(
            success.status.success(),
            "unselected dependency {name} was visited:\n{}",
            String::from_utf8_lossy(&success.stderr)
        );
        assert_eq!(
            std::fs::read(output_dir.join("sentinel.keep")).unwrap(),
            b"unselected-v9\n"
        );
        assert!(!output_dir.join("chosen").exists());
    }

    let unknown_output = temp.path().join("unknown-target-output");
    let unknown = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(cases.join("target_unix/Cargo.toml"))
        .arg("--output-dir")
        .arg(&unknown_output)
        .env("CARGO_BUILD_TARGET", "rusty-cpp-definitely-unknown-target")
        .output()
        .expect("run unknown target fail-closed fixture");
    assert!(!unknown.status.success());
    assert!(
        String::from_utf8_lossy(&unknown.stderr)
            .contains("requires an exact Cargo target-selected normal local-dependency graph"),
        "unknown target failed for the wrong reason:\n{}",
        String::from_utf8_lossy(&unknown.stderr)
    );
    assert!(!unknown_output.exists());
}

#[test]
fn cpp_name_dependency_graph_is_package_specific_inside_a_workspace() {
    let rustc = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    if !String::from_utf8_lossy(&rustc.stdout)
        .lines()
        .any(|line| line == "host: x86_64-unknown-linux-gnu")
    {
        eprintln!("skipping package-specific workspace graph gate on non-Linux test host");
        return;
    }

    let temp = tempfile::tempdir().expect("package-specific graph tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".cargo")).unwrap();
    std::fs::write(
        workspace.join(".cargo/config.toml"),
        "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
    )
    .unwrap();
    std::fs::write(
        workspace.join("Cargo.toml"),
        r#"[workspace]
members=['root-control','root-selected','chooser','bridge','poison']
resolver='2'

[workspace.dependencies]
control={package='chooser',path='chooser',default-features=false}
selected={package='chooser',path='chooser',default-features=false,features=['activate']}
"#,
    )
    .unwrap();
    let write_member = |directory: &str, manifest: &str, source: &str| {
        let member = workspace.join(directory);
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(member.join("Cargo.toml"), manifest).unwrap();
        std::fs::write(member.join("src/lib.rs"), source).unwrap();
    };
    write_member(
        "poison",
        "[package]\nname='poison'\nversion='0.1.0'\nedition='2024'\n",
        r#"
#[cfg_attr(any(), cpp_name(poison_overload))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(poison_overload))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    );
    write_member(
        "bridge",
        "[package]\nname='feature-bridge'\nversion='0.1.0'\nedition='2024'\n[dependencies]\npoison={path='../poison',optional=true}\n[features]\ndefault=['dep:poison']\n",
        "pub fn value() -> i32 { poison::first(1) }\n",
    );
    write_member(
        "chooser",
        "[package]\nname='chooser'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nfeature_bridge={package='feature-bridge',path='../bridge',optional=true}\n[features]\ndefault=[]\nactivate=['dep:feature_bridge']\n",
        "#[cfg(feature=\"activate\")] pub fn value() -> i32 { feature_bridge::value() }\n#[cfg(not(feature=\"activate\"))] pub fn value() -> i32 { 3 }\n",
    );
    write_member(
        "root-control",
        "[package]\nname='root_control'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ncontrol.workspace=true\n",
        "pub fn root() -> i32 { control::value() }\n",
    );
    write_member(
        "root-selected",
        "[package]\nname='root_selected'\nversion='0.1.0'\nedition='2024'\n[target.'cfg(all(unix, target_arch = \"x86_64\", not(target_os = \"macos\")))'.dependencies]\nselected.workspace=true\n",
        "pub fn root() -> i32 { selected::value() }\n",
    );

    let control_output = workspace.join("control-output");
    let control = run_crate_transpiler(&workspace.join("root-control/Cargo.toml"), &control_output);
    assert!(
        control.status.success(),
        "package-specific control was polluted by another workspace root:\n{}",
        String::from_utf8_lossy(&control.stderr)
    );
    assert!(control_output.join("control/chooser.cppm").is_file());
    assert!(
        !control_output.join("control/feature_bridge").exists(),
        "control graph followed an optional edge activated only by root-selected"
    );
    assert!(!control_output.join("control/poison").exists());

    for existing in [false, true] {
        let lane = if existing { "existing" } else { "fresh" };
        let selected_output = workspace.join(format!("selected-{lane}"));
        if existing {
            std::fs::create_dir_all(&selected_output).unwrap();
            std::fs::write(selected_output.join("sentinel.keep"), b"package-specific\n").unwrap();
        }
        let selected = run_crate_transpiler(
            &workspace.join("root-selected/Cargo.toml"),
            &selected_output,
        );
        assert!(!selected.status.success());
        let stderr = String::from_utf8_lossy(&selected.stderr);
        assert!(
            stderr
                .contains("cpp_name whole local-dependency closure preflight failed before output")
                && stderr.contains("cpp_name overload collision")
                && stderr.contains("poison"),
            "package-specific selected graph failed for the wrong reason:\n{stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read(selected_output.join("sentinel.keep")).unwrap(),
                b"package-specific\n"
            );
            assert_eq!(std::fs::read_dir(&selected_output).unwrap().count(), 1);
        } else {
            assert!(!selected_output.exists());
        }
    }
}

#[test]
fn cpp_name_resolver_v2_keeps_normal_features_separate_from_build_and_dev() {
    let temp = tempfile::tempdir().expect("resolver-context tempdir");

    for dependency_kind in ["build-dependencies", "dev-dependencies"] {
        let lane = dependency_kind.trim_end_matches("-dependencies");
        let fixture = temp.path().join(lane);
        let root = fixture.join("root");
        let chooser = fixture.join("chooser");
        let poison = fixture.join("poison");
        for directory in [&root, &chooser, &poison] {
            std::fs::create_dir_all(directory.join("src")).unwrap();
        }
        std::fs::write(
            poison.join("Cargo.toml"),
            "[package]\nname='context_poison'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(
            poison.join("src/lib.rs"),
            r#"
#[cfg_attr(any(), cpp_name(context_collision))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(context_collision))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
        )
        .unwrap();
        std::fs::write(
            chooser.join("Cargo.toml"),
            "[package]\nname='context_chooser'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ncontext_poison={path='../poison',optional=true}\n[features]\ndefault=[]\nactivate=['dep:context_poison']\n[workspace]\nresolver='2'\n",
        )
        .unwrap();
        std::fs::write(
            chooser.join("src/lib.rs"),
            "#[cfg(feature=\"activate\")] pub fn value() -> i32 { context_poison::first(1) }\n#[cfg(not(feature=\"activate\"))] pub fn value() -> i32 { 3 }\n",
        )
        .unwrap();

        let overlap = format!(
            "[package]\nname='context_root_{lane}'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ncontext_chooser={{path='../chooser',default-features=false}}\n[{dependency_kind}]\ncontext_chooser={{path='../chooser',default-features=false,features=['activate']}}\n[workspace]\nresolver='2'\n"
        );
        std::fs::write(root.join("Cargo.toml"), overlap).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            r#"
#[cfg_attr(any(), cpp_name(context_root_value))]
pub fn root_value() -> i32 { context_chooser::value() }
"#,
        )
        .unwrap();
        if dependency_kind == "build-dependencies" {
            std::fs::write(
                root.join("build.rs"),
                "fn main() { let _ = context_chooser::value(); }\n",
            )
            .unwrap();
        }

        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(fixture.join("cargo-target"))
            .output()
            .expect("cargo-check resolver-context fixture");
        assert!(
            cargo_check.status.success(),
            "resolver-context {lane} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let normal_tree = Command::new("cargo")
            .arg("tree")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .args(["--edges", "normal", "--prefix", "depth", "--no-dedupe"])
            .output()
            .expect("query Cargo normal context");
        assert!(normal_tree.status.success());
        assert!(
            !String::from_utf8_lossy(&normal_tree.stdout).contains("context_poison"),
            "Cargo normal {lane} context unexpectedly selected poison:\n{}",
            String::from_utf8_lossy(&normal_tree.stdout)
        );

        for existing in [false, true] {
            let output = fixture.join(if existing {
                "normal-existing"
            } else {
                "normal-fresh"
            });
            if existing {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(output.join("sentinel.keep"), b"resolver-context\n").unwrap();
            }
            let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
            assert!(
                result.status.success(),
                "normal {lane} graph was polluted by its {lane} feature context:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(output.join(format!("context_root_{lane}.cppm")).is_file());
            assert!(
                output
                    .join("context_chooser/context_chooser.cppm")
                    .is_file()
            );
            assert!(
                !output.join("context_chooser/context_poison").exists(),
                "normal {lane} generation followed a {lane}-only optional edge"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output.join("sentinel.keep")).unwrap(),
                    b"resolver-context\n"
                );
            }
        }

        // The same optional edge enabled in the normal context must remain a
        // selected source-owned contract and fail before fresh or existing
        // output is mutated.
        let selected = fixture.join("selected");
        std::fs::create_dir_all(selected.join("src")).unwrap();
        std::fs::write(
            selected.join("Cargo.toml"),
            format!(
                "[package]\nname='context_selected_{lane}'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ncontext_chooser={{path='../chooser',default-features=false,features=['activate']}}\n[workspace]\nresolver='2'\n"
            ),
        )
        .unwrap();
        std::fs::write(
            selected.join("src/lib.rs"),
            "#[cfg_attr(any(), cpp_name(context_selected_value))]\npub fn value() -> i32 { context_chooser::value() }\n",
        )
        .unwrap();
        for existing in [false, true] {
            let output = fixture.join(if existing {
                "selected-existing"
            } else {
                "selected-fresh"
            });
            if existing {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(output.join("sentinel.keep"), b"selected-context\n").unwrap();
            }
            let result = run_crate_transpiler(&selected.join("Cargo.toml"), &output);
            assert!(
                !result.status.success(),
                "selected normal {lane} edge was lost"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains(
                    "cpp_name whole local-dependency closure preflight failed before output"
                ) && stderr.contains("cpp_name overload collision")
                    && stderr.contains("context_collision")
                    && stderr.contains("poison/Cargo.toml"),
                "selected normal {lane} edge failed for the wrong reason:\n{stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output.join("sentinel.keep")).unwrap(),
                    b"selected-context\n"
                );
                assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
            } else {
                assert!(!output.exists());
            }
        }
    }
}

#[test]
fn cpp_name_target_normal_graph_prunes_proc_macro_feature_context() {
    let temp = tempfile::tempdir().expect("proc-macro feature-context tempdir");
    let root = temp.path().join("root");
    let selected = temp.path().join("selected");
    let maker = temp.path().join("maker");
    let shared = temp.path().join("shared");
    let poison = temp.path().join("poison");
    for package in [&root, &selected, &maker, &shared, &poison] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        poison.join("Cargo.toml"),
        "[package]\nname='proc_context_poison'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        poison.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(proc_context_collision))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(proc_context_collision))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();
    std::fs::write(
        shared.join("Cargo.toml"),
        "[package]\nname='proc_context_shared'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nproc_context_poison={path='../poison',optional=true}\n[features]\ndefault=[]\nactivate=['dep:proc_context_poison']\n",
    )
    .unwrap();
    std::fs::write(
        shared.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(proc_context_shared_value))]
pub fn value() -> i32 { 7 }
"#,
    )
    .unwrap();
    std::fs::write(
        maker.join("Cargo.toml"),
        "[package]\nname='proc_context_maker'\nversion='0.1.0'\nedition='2024'\n[lib]\nproc-macro=true\n[dependencies]\nproc_context_shared={path='../shared',default-features=false,features=['activate']}\n",
    )
    .unwrap();
    std::fs::write(
        maker.join("src/lib.rs"),
        r#"
extern crate proc_macro;
use proc_macro::TokenStream;
#[proc_macro]
pub fn make_item(_input: TokenStream) -> TokenStream {
    let _ = proc_context_shared::value();
    "pub fn made() -> i32 { 7 }".parse().unwrap()
}
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='proc_context_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nproc_context_shared={path='../shared',default-features=false}\nproc_context_maker={path='../maker'}\n[workspace]\nresolver='2'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(proc_context_root_value))]
pub fn root_value() -> i32 { proc_context_shared::value() }
"#,
    )
    .unwrap();

    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(temp.path().join("cargo-target"))
        .output()
        .expect("cargo-check proc-macro feature-context fixture");
    assert!(
        cargo_check.status.success(),
        "proc-macro feature-context fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    let unpruned_tree = Command::new("cargo")
        .arg("tree")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .args(["--edges", "normal", "--prefix", "depth", "--no-dedupe"])
        .output()
        .expect("query unpruned Cargo normal graph");
    assert!(unpruned_tree.status.success());
    assert!(
        String::from_utf8_lossy(&unpruned_tree.stdout).contains("proc_context_poison"),
        "counterexample no longer demonstrates host feature pollution"
    );
    let target_tree = Command::new("cargo")
        .arg("tree")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .args([
            "--edges",
            "normal,no-proc-macro",
            "--prefix",
            "depth",
            "--no-dedupe",
        ])
        .output()
        .expect("query target-normal Cargo graph");
    assert!(target_tree.status.success());
    let target_tree = String::from_utf8_lossy(&target_tree.stdout);
    assert!(target_tree.contains("proc_context_shared"));
    assert!(!target_tree.contains("proc_context_maker"));
    assert!(!target_tree.contains("proc_context_poison"));

    for existing in [false, true] {
        let output = temp.path().join(if existing {
            "target-normal-existing"
        } else {
            "target-normal-fresh"
        });
        if existing {
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("sentinel.keep"), b"target-normal\n").unwrap();
        }
        let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
        assert!(
            result.status.success(),
            "target-normal graph was polluted by its proc-macro host context:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.join("proc_context_root.cppm").is_file());
        assert!(
            output
                .join("proc_context_shared/proc_context_shared.cppm")
                .is_file()
        );
        assert!(!output.join("proc_context_maker").exists());
        assert!(
            !output
                .join("proc_context_shared/proc_context_poison")
                .exists()
        );
        if existing {
            assert_eq!(
                std::fs::read(output.join("sentinel.keep")).unwrap(),
                b"target-normal\n"
            );
        }
    }

    // A target-normal activation of the same optional dependency must still
    // be selected and fail before either fresh or sentinel output is changed.
    std::fs::write(
        selected.join("Cargo.toml"),
        "[package]\nname='proc_context_selected'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nproc_context_shared={path='../shared',default-features=false,features=['activate']}\n[workspace]\nresolver='2'\n",
    )
    .unwrap();
    std::fs::write(
        selected.join("src/lib.rs"),
        "#[cfg_attr(any(), cpp_name(proc_context_selected_value))]\npub fn value() -> i32 { proc_context_shared::value() }\n",
    )
    .unwrap();
    for existing in [false, true] {
        let output = temp.path().join(if existing {
            "selected-existing"
        } else {
            "selected-fresh"
        });
        if existing {
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("sentinel.keep"), b"selected-target\n").unwrap();
        }
        let result = run_crate_transpiler(&selected.join("Cargo.toml"), &output);
        assert!(!result.status.success(), "lost target-normal activation");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr
                .contains("cpp_name whole local-dependency closure preflight failed before output")
                && stderr.contains("proc_context_collision")
                && stderr.contains("poison/Cargo.toml"),
            "target-normal activation failed for the wrong reason:\n{stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read(output.join("sentinel.keep")).unwrap(),
                b"selected-target\n"
            );
            assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
        } else {
            assert!(!output.exists());
        }
    }
}

#[test]
fn cpp_name_cargo_path_overrides_feed_exact_graph_atomically() {
    fn initialize_git_package(directory: &Path, package_name: &str, version: &str) -> String {
        std::fs::create_dir_all(directory.join("src")).unwrap();
        std::fs::write(
            directory.join("Cargo.toml"),
            format!("[package]\nname='{package_name}'\nversion='{version}'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(
            directory.join("src/lib.rs"),
            "pub fn value() -> i32 { 7 }\n",
        )
        .unwrap();
        for arguments in [
            vec!["init", "--quiet"],
            vec!["config", "user.email", "cpp-name-test@example.invalid"],
            vec!["config", "user.name", "cpp-name-test"],
            vec!["add", "Cargo.toml", "src/lib.rs"],
            vec!["commit", "--quiet", "-m", "fixture"],
        ] {
            let result = Command::new("git")
                .args(arguments)
                .current_dir(directory)
                .output()
                .expect("run git for local override fixture");
            assert!(
                result.status.success(),
                "could not initialize local override git source:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        format!(
            "file://{}",
            std::fs::canonicalize(directory).unwrap().display()
        )
    }

    fn write_collision_package(directory: &Path, package_name: &str, version: &str) {
        std::fs::create_dir_all(directory.join("src")).unwrap();
        std::fs::write(
            directory.join("Cargo.toml"),
            format!("[package]\nname='{package_name}'\nversion='{version}'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(
            directory.join("src/lib.rs"),
            r#"
#[cfg_attr(any(), cpp_name(override_collision))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(override_collision))]
pub fn second(value: i32) -> i32 { value + 1 }
pub fn value() -> i32 { 7 }
"#,
        )
        .unwrap();
    }

    let temp = tempfile::tempdir().expect("workspace path override tempdir");
    for override_kind in ["patch", "replace"] {
        let lane = temp.path().join(format!("selected-{override_kind}"));
        let original = lane.join("original");
        let patched = lane.join("patched");
        let root = lane.join("root");
        let package_name = format!("selected_{override_kind}_leaf");
        let git_url = initialize_git_package(&original, &package_name, "0.1.0");
        write_collision_package(&patched, &package_name, "0.1.0");
        std::fs::create_dir_all(root.join("src")).unwrap();
        let override_table = if override_kind == "patch" {
            format!("[patch.{git_url:?}]\n{package_name}={{path='../patched'}}\n")
        } else {
            format!(
                "[replace]\n{replacement:?}={{path='../patched'}}\n",
                replacement = format!("git+{git_url}#{package_name}@0.1.0")
            )
        };
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='selected_{override_kind}_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\n{package_name}={{git={git_url:?},version='=0.1.0'}}\n{override_table}[workspace]\nresolver='2'\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            format!("pub fn root() -> i32 {{ {package_name}::value() }}\n"),
        )
        .unwrap();

        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(lane.join("cargo-target"))
            .output()
            .expect("cargo-check selected local override fixture");
        assert!(
            cargo_check.status.success(),
            "selected {override_kind} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        for existing in [false, true] {
            let output = lane.join(if existing {
                "selected-existing"
            } else {
                "selected-fresh"
            });
            if existing {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(output.join("sentinel.keep"), b"selected-override\n").unwrap();
            }
            let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
            assert!(
                !result.status.success(),
                "selected local {override_kind} contract was skipped"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains(
                    "cpp_name whole local-dependency closure preflight failed before output"
                ) && stderr.contains("cpp_name overload collision")
                    && stderr.contains("override_collision")
                    && stderr.contains("patched/Cargo.toml"),
                "selected local {override_kind} failed for the wrong reason:\n{stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output.join("sentinel.keep")).unwrap(),
                    b"selected-override\n"
                );
                assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
            } else {
                assert!(!output.exists());
            }
        }
    }

    // A path patch with a version outside the dependency requirement is only
    // an over-approximation witness. Cargo's exact graph must discard it in
    // both preflight and recursive generation.
    let lane = temp.path().join("unselected-version");
    let original = lane.join("original");
    let patched = lane.join("patched");
    let root = lane.join("root");
    let package_name = "unselected_version_leaf";
    let git_url = initialize_git_package(&original, package_name, "0.1.0");
    write_collision_package(&patched, package_name, "9.9.9");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname='unselected_version_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\n{package_name}={{git={git_url:?},version='=0.1.0'}}\n[patch.{git_url:?}]\n{package_name}={{path='../patched'}}\n[workspace]\nresolver='2'\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        format!("pub fn root() -> i32 {{ {package_name}::value() }}\n"),
    )
    .unwrap();
    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(lane.join("cargo-target"))
        .output()
        .expect("cargo-check unselected patch version fixture");
    assert!(
        cargo_check.status.success(),
        "unselected patch version fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    for existing in [false, true] {
        let output = lane.join(if existing {
            "unselected-existing"
        } else {
            "unselected-fresh"
        });
        if existing {
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("sentinel.keep"), b"unselected-override\n").unwrap();
        }
        let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
        assert!(
            result.status.success(),
            "unselected patch version polluted the exact graph:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.join("unselected_version_root.cppm").is_file());
        assert!(!output.join(package_name).exists());
        if existing {
            assert_eq!(
                std::fs::read(output.join("sentinel.keep")).unwrap(),
                b"unselected-override\n"
            );
        }
    }

    // Cargo configuration is an independent override surface. Cover modern
    // and legacy discovery, recursive include, absolute and relative home
    // discovery, CARGO_HOME, and the legacy top-level `paths` override. Every
    // selected local contract must take the same exact preflight path as a
    // manifest-owned patch.
    let inherited_rustup_home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup")));
    for config_kind in [
        "modern",
        "legacy",
        "legacy-precedence",
        "included",
        "cargo-home",
        "relative-home",
        "paths",
    ] {
        let lane = temp.path().join(format!("config-selected-{config_kind}"));
        let original = lane.join("original");
        let patched = lane.join("patched");
        let root = lane.join("root");
        let package_name = format!("config_selected_{config_kind}_leaf").replace('-', "_");
        let git_url = initialize_git_package(&original, &package_name, "0.1.0");
        write_collision_package(&patched, &package_name, "0.1.0");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='config_selected_{config_kind}_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\n{package_name}={{git={git_url:?},version='=0.1.0'}}\n[workspace]\nresolver='2'\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            format!("pub fn root() -> i32 {{ {package_name}::value() }}\n"),
        )
        .unwrap();

        let patch_config = format!(
            "[patch.{git_url:?}]\n{package_name}={{path={patched:?}}}\n",
            patched = std::fs::canonicalize(&patched).unwrap()
        );
        let config_home = match config_kind {
            "cargo-home" => lane.join("cargo-home"),
            "relative-home" => root.join("relative-home/.cargo"),
            _ => root.join(".cargo"),
        };
        std::fs::create_dir_all(&config_home).unwrap();
        match config_kind {
            "legacy" => std::fs::write(config_home.join("config"), &patch_config).unwrap(),
            "legacy-precedence" => {
                std::fs::write(config_home.join("config"), &patch_config).unwrap();
                std::fs::write(
                    config_home.join("config.toml"),
                    "# Cargo must ignore this modern spelling when legacy config exists.\n",
                )
                .unwrap();
            }
            "included" => {
                std::fs::write(
                    config_home.join("config.toml"),
                    "include = [{path='optional-absent.toml',optional=true},'include-level-one.toml']\n",
                )
                .unwrap();
                std::fs::write(
                    config_home.join("include-level-one.toml"),
                    "include = [{path='local-overrides.toml'}]\n",
                )
                .unwrap();
                std::fs::write(config_home.join("local-overrides.toml"), &patch_config).unwrap();
            }
            "paths" => std::fs::write(
                config_home.join("config.toml"),
                format!(
                    "paths = [{patched:?}]\n",
                    patched = std::fs::canonicalize(&patched).unwrap()
                ),
            )
            .unwrap(),
            "modern" | "cargo-home" | "relative-home" => {
                std::fs::write(config_home.join("config.toml"), &patch_config).unwrap()
            }
            _ => unreachable!(),
        }

        let mut cargo_check = Command::new("cargo");
        cargo_check
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(lane.join("cargo-target"));
        if config_kind == "cargo-home" {
            cargo_check.env("CARGO_HOME", &config_home);
        } else if config_kind == "relative-home" {
            cargo_check
                .current_dir(&root)
                .env_remove("CARGO_HOME")
                .env("HOME", "relative-home");
            if let Some(rustup_home) = &inherited_rustup_home {
                cargo_check.env("RUSTUP_HOME", rustup_home);
            }
        }
        let cargo_check = cargo_check
            .output()
            .expect("cargo-check selected Cargo configuration override fixture");
        assert!(
            cargo_check.status.success(),
            "selected Cargo configuration {config_kind} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        for existing in [false, true] {
            let output = lane.join(if existing {
                "selected-existing"
            } else {
                "selected-fresh"
            });
            if existing {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(output.join("sentinel.keep"), b"selected-config-override\n")
                    .unwrap();
            }
            let mut command = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
            command.arg("--crate");
            if config_kind == "modern" && !existing {
                // Also prove the exact resolver accepts a one-component
                // relative manifest spelling from its Cargo working directory.
                command.arg("Cargo.toml").current_dir(&root);
            } else {
                command.arg(root.join("Cargo.toml"));
            }
            command.arg("--output-dir").arg(&output);
            if config_kind == "cargo-home" {
                command.env("CARGO_HOME", &config_home);
            } else if config_kind == "relative-home" {
                command
                    .env_remove("CARGO_HOME")
                    .env("HOME", "relative-home");
                if let Some(rustup_home) = &inherited_rustup_home {
                    command.env("RUSTUP_HOME", rustup_home);
                }
            }
            let result = command
                .output()
                .expect("run selected Cargo configuration override fixture");
            assert!(
                !result.status.success(),
                "selected Cargo configuration {config_kind} contract was skipped"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains(
                    "cpp_name whole local-dependency closure preflight failed before output"
                ) && stderr.contains("cpp_name overload collision")
                    && stderr.contains("override_collision")
                    && stderr.contains("patched/Cargo.toml"),
                "selected Cargo configuration {config_kind} failed for the wrong reason:\n{stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output.join("sentinel.keep")).unwrap(),
                    b"selected-config-override\n"
                );
                assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
            } else {
                assert!(!output.exists());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // With HOME absent/empty and CARGO_HOME absent/empty, Cargo falls back
        // to the account database for its home directory. Interpose that
        // lookup in subprocesses so this regression never mutates the real
        // user home.
        let lane = temp.path().join("config-passwd-home-selected");
        let original = lane.join("original");
        let patched = lane.join("patched");
        let root = lane.join("root");
        let fake_home = lane.join("fake-home");
        let package_name = "config_passwd_home_leaf";
        let git_url = initialize_git_package(&original, package_name, "0.1.0");
        write_collision_package(&patched, package_name, "0.1.0");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(fake_home.join(".cargo")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='config_passwd_home_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\n{package_name}={{git={git_url:?},version='=0.1.0'}}\n[workspace]\nresolver='2'\n"
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            format!("pub fn root() -> i32 {{ {package_name}::value() }}\n"),
        )
        .unwrap();
        std::fs::write(
            fake_home.join(".cargo/config.toml"),
            format!(
                "[patch.{git_url:?}]\n{package_name}={{path={patched:?}}}\n",
                patched = std::fs::canonicalize(&patched).unwrap()
            ),
        )
        .unwrap();

        let preload_source = lane.join("fake_home.c");
        let preload_library = lane.join("fake_home.so");
        std::fs::write(
            &preload_source,
            format!(
                r#"#define _GNU_SOURCE
#include <pwd.h>
#include <string.h>
#include <sys/types.h>
static char fake_name[] = "rusty_cpp_probe";
static char fake_passwd[] = "x";
static char fake_gecos[] = "";
static char fake_dir[] = "{fake_home}";
static char fake_shell[] = "/bin/sh";
static struct passwd fake;
static void fill(struct passwd *entry, uid_t uid) {{
    memset(entry, 0, sizeof(*entry));
    entry->pw_name = fake_name;
    entry->pw_passwd = fake_passwd;
    entry->pw_uid = uid;
    entry->pw_gid = 0;
    entry->pw_gecos = fake_gecos;
    entry->pw_dir = fake_dir;
    entry->pw_shell = fake_shell;
}}
struct passwd *getpwuid(uid_t uid) {{ fill(&fake, uid); return &fake; }}
int getpwuid_r(uid_t uid, struct passwd *entry, char *buffer, size_t length,
               struct passwd **result) {{
    (void)buffer;
    (void)length;
    fill(entry, uid);
    *result = entry;
    return 0;
}}
"#,
                fake_home = fake_home.display()
            ),
        )
        .unwrap();
        let compile_preload = Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(&preload_source)
            .arg("-o")
            .arg(&preload_library)
            .output()
            .expect("compile account-home interposer");
        assert!(
            compile_preload.status.success(),
            "account-home interposer failed to compile:\n{}",
            String::from_utf8_lossy(&compile_preload.stderr)
        );
        let rustup_home = std::env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup")));

        let configure = |command: &mut Command, home_mode: &str| {
            command
                .env_remove("HOME")
                .env_remove("CARGO_HOME")
                .env("LD_PRELOAD", &preload_library);
            match home_mode {
                "absent" => {}
                "empty-cargo-home" => {
                    command.env("CARGO_HOME", "");
                }
                "empty-home" => {
                    command.env("HOME", "");
                }
                _ => unreachable!(),
            }
            if let Some(rustup_home) = &rustup_home {
                command.env("RUSTUP_HOME", rustup_home);
            }
        };
        for home_mode in ["absent", "empty-cargo-home", "empty-home"] {
            // Keep the three Cargo proofs independent: no mode may inherit a
            // prior mode's selected path package through a lockfile or target
            // directory.
            let _ = std::fs::remove_file(root.join("Cargo.lock"));
            let mut cargo_check = Command::new("cargo");
            cargo_check
                .arg("check")
                .arg("--manifest-path")
                .arg(root.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(lane.join(format!("cargo-target-{home_mode}")));
            configure(&mut cargo_check, home_mode);
            let cargo_check = cargo_check
                .output()
                .expect("cargo-check account-home configuration fixture");
            assert!(
                cargo_check.status.success(),
                "account-home ({home_mode}) configuration fixture is not Cargo-valid:\n{}",
                String::from_utf8_lossy(&cargo_check.stderr)
            );
            let cargo_stderr = String::from_utf8_lossy(&cargo_check.stderr);
            let patched_path = patched.to_string_lossy();
            assert!(
                cargo_stderr.contains(patched_path.as_ref()),
                "cargo check did not prove that account-home ({home_mode}) selected the patched local package:\n{cargo_stderr}"
            );

            for existing in [false, true] {
                let output = lane.join(format!(
                    "{home_mode}-{}",
                    if existing { "existing" } else { "fresh" }
                ));
                if existing {
                    std::fs::create_dir_all(&output).unwrap();
                    std::fs::write(output.join("sentinel.keep"), b"account-home-config\n")
                        .unwrap();
                }
                let mut command = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
                command
                    .arg("--crate")
                    .arg(root.join("Cargo.toml"))
                    .arg("--output-dir")
                    .arg(&output);
                configure(&mut command, home_mode);
                let result = command
                    .output()
                    .expect("run account-home configuration fixture");
                assert!(
                    !result.status.success(),
                    "selected account-home ({home_mode}) configuration contract was skipped"
                );
                let stderr = String::from_utf8_lossy(&result.stderr);
                assert!(
                    stderr.contains(
                        "cpp_name whole local-dependency closure preflight failed before output"
                    ) && stderr.contains("cpp_name overload collision")
                        && stderr.contains("override_collision")
                        && stderr.contains("patched/Cargo.toml"),
                    "account-home ({home_mode}) configuration failed for the wrong reason:\n{stderr}"
                );
                if existing {
                    assert_eq!(
                        std::fs::read(output.join("sentinel.keep")).unwrap(),
                        b"account-home-config\n"
                    );
                    assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
                } else {
                    assert!(!output.exists());
                }
            }
        }
    }

    // A configuration patch is still only an over-approximation witness. An
    // unselected local version must not leak back into preflight or generation.
    let lane = temp.path().join("config-unselected-version");
    let original = lane.join("original");
    let patched = lane.join("patched");
    let root = lane.join("root");
    let package_name = "config_unselected_version_leaf";
    let git_url = initialize_git_package(&original, package_name, "0.1.0");
    write_collision_package(&patched, package_name, "9.9.9");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join(".cargo")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname='config_unselected_version_root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\n{package_name}={{git={git_url:?},version='=0.1.0'}}\n[workspace]\nresolver='2'\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        format!("pub fn root() -> i32 {{ {package_name}::value() }}\n"),
    )
    .unwrap();
    std::fs::write(
        root.join(".cargo/config.toml"),
        format!(
            "[patch.{git_url:?}]\n{package_name}={{path={patched:?}}}\n",
            patched = std::fs::canonicalize(&patched).unwrap()
        ),
    )
    .unwrap();
    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(lane.join("cargo-target"))
        .output()
        .expect("cargo-check unselected Cargo configuration patch fixture");
    assert!(
        cargo_check.status.success(),
        "unselected Cargo configuration patch fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    for existing in [false, true] {
        let output = lane.join(if existing {
            "unselected-existing"
        } else {
            "unselected-fresh"
        });
        if existing {
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(
                output.join("sentinel.keep"),
                b"unselected-config-override\n",
            )
            .unwrap();
        }
        let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
        assert!(
            result.status.success(),
            "unselected Cargo configuration patch polluted the exact graph:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.join("config_unselected_version_root.cppm").is_file());
        assert!(!output.join(package_name).exists());
        if existing {
            assert_eq!(
                std::fs::read(output.join("sentinel.keep")).unwrap(),
                b"unselected-config-override\n"
            );
        }
    }

    // Invalid required includes and include cycles are Cargo-invalid, but the
    // cheap scan must still force the exact path and preserve output atomically
    // instead of treating uncertainty as proof that no local override exists.
    for config_error in ["missing-required-include", "include-cycle"] {
        let lane = temp.path().join(config_error);
        let root = lane.join("root");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".cargo")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{config_error}'\nversion='0.1.0'\nedition='2024'\n[workspace]\nresolver='2'\n"
            ),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn root() -> i32 { 7 }\n").unwrap();
        if config_error == "missing-required-include" {
            std::fs::write(
                root.join(".cargo/config.toml"),
                "include=['required-but-absent.toml']\n",
            )
            .unwrap();
        } else {
            std::fs::write(root.join(".cargo/config.toml"), "include=['cycle.toml']\n").unwrap();
            std::fs::write(root.join(".cargo/cycle.toml"), "include=['config.toml']\n").unwrap();
        }
        for existing in [false, true] {
            let output = lane.join(if existing { "existing" } else { "fresh" });
            if existing {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::write(output.join("sentinel.keep"), b"invalid-config\n").unwrap();
            }
            let result = run_crate_transpiler(&root.join("Cargo.toml"), &output);
            assert!(
                !result.status.success(),
                "Cargo configuration error {config_error} was ignored"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains("exact Cargo target-selected normal local-dependency graph")
                    || stderr.contains("Cargo configuration include cycle"),
                "Cargo configuration error {config_error} failed for the wrong reason:\n{stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read(output.join("sentinel.keep")).unwrap(),
                    b"invalid-config\n"
                );
                assert_eq!(std::fs::read_dir(&output).unwrap().count(), 1);
            } else {
                assert!(!output.exists());
            }
        }
    }
}

#[test]
fn cpp_name_uses_canonical_cpp_parameter_identity_for_type_maps_before_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source = r#"
pub struct Left;
pub struct Right;

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn left(_value: Left) -> i32 { 1 }

#[cfg_attr(any(), cpp_name(overloaded))]
pub fn right(_value: Right) -> i32 { 2 }
"#;
    let direct_source = temp.path().join("identity.rs");
    std::fs::write(&direct_source, source).unwrap();
    let rustc = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib"])
        .arg(&direct_source)
        .arg("-o")
        .arg(temp.path().join("libidentity.rlib"))
        .output()
        .expect("rustc type-map identity fixture");
    assert!(
        rustc.status.success(),
        "type-map fixture is not rustc-valid:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let collisions = [
        ("fundamental", "unsigned long", "unsigned long int"),
        ("specifier-order", "long unsigned", "int unsigned long"),
        ("top-cv", "const int", "int"),
        ("top-pointer-cv", "int* const", "int*"),
        (
            "reference-fundamental",
            "unsigned long const&",
            "const unsigned long int&",
        ),
        (
            "pointer-fundamental",
            "unsigned long const*",
            "const unsigned long int*",
        ),
        (
            "template-fundamental",
            "Wrapper<unsigned long>",
            "Wrapper<unsigned long int>",
        ),
        (
            "function-template-fundamental",
            "Callable<int(unsigned long) const>",
            "Callable<int(unsigned long int) const>",
        ),
        ("opaque-typedef-names", "AliasA", "AliasB"),
        (
            "opaque-alias-templates",
            "AliasTemplate<int>",
            "AliasTemplate<bool>",
        ),
    ];
    for (label, left, right) in collisions {
        let type_map = temp.path().join(format!("{label}.toml"));
        std::fs::write(
            &type_map,
            format!("Left = {left:?}\nRight = {right:?}\n"),
        )
        .unwrap();
        let output = temp.path().join(format!("{label}.cppm"));
        let result = run_transpiler(
            &direct_source,
            &output,
            &[
                "-m",
                "canonical_identity",
                "--type-map",
                type_map.to_str().unwrap(),
            ],
        );
        assert!(!result.status.success(), "accepted {label} collision");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("cpp_name overload collision"),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!output.exists(), "{label} collision created output");
    }

    // Crate mode must run the same proof in memory before it creates the
    // output directory. The fixture itself remains ordinary Cargo-valid Rust.
    let crate_dir = temp.path().join("identity-crate");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"identity_crate\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(crate_dir.join("src/lib.rs"), source).unwrap();
    let cargo_check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(crate_dir.join("rust-target"))
        .output()
        .expect("cargo-check type-map crate fixture");
    assert!(
        cargo_check.status.success(),
        "type-map crate fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    for (label, left, right) in [
        ("crate-fundamental", "unsigned long", "unsigned long int"),
        ("crate-top-cv", "const int", "int"),
    ] {
        let type_map = crate_dir.join(format!("{label}.toml"));
        std::fs::write(
            &type_map,
            format!("Left = {left:?}\nRight = {right:?}\n"),
        )
        .unwrap();
        let output_dir = crate_dir.join(format!("cpp-out-{label}"));
        let result = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .output()
            .expect("run crate-mode canonical identity fixture");
        assert!(!result.status.success(), "accepted {label} collision");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("cpp_name overload collision"),
            "unexpected {label} diagnostic:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !output_dir.exists(),
            "{label} collision created partial crate output"
        );
    }

    // These cv/rank differences are genuine overload distinctions and must
    // not be collapsed by the conservative parser.
    for (label, left, right) in [
        ("referent-cv-distinct", "const int&", "int&"),
        ("pointee-cv-distinct", "const int*", "int*"),
        ("fundamental-rank-distinct", "long", "long long"),
    ] {
        let type_map = temp.path().join(format!("{label}.toml"));
        std::fs::write(
            &type_map,
            format!("Left = {left:?}\nRight = {right:?}\n"),
        )
        .unwrap();
        let output = temp.path().join(format!("{label}.cppm"));
        let result = run_transpiler(
            &direct_source,
            &output,
            &[
                "-m",
                label,
                "--type-map",
                type_map.to_str().unwrap(),
            ],
        );
        assert!(
            result.status.success(),
            "rejected proven-distinct {label}:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(output.exists());
    }

    if let Some(clang) = find_clang() {
        for (label, left, right) in [
            ("clang-fundamental", "unsigned long", "unsigned long int"),
            ("clang-top-cv", "const int", "int"),
        ] {
            let source = temp.path().join(format!("{label}.cpp"));
            std::fs::write(
                &source,
                format!("int overloaded({left}) {{ return 1; }}\nint overloaded({right}) {{ return 2; }}\n"),
            )
            .unwrap();
            let clang_result = Command::new(&clang)
                .args(["-std=c++23", "-fsyntax-only"])
                .arg(&source)
                .output()
                .expect("run Clang identity oracle");
            assert!(
                !clang_result.status.success(),
                "Clang did not diagnose {label} as one parameter identity"
            );
        }
    }
}

#[test]
fn cpp_name_cpp_inherit_requires_exact_inert_runtime_marker_provenance() {
    #[derive(Clone, Copy)]
    enum MarkerCase {
        Injecting,
        RenamedInert,
        WrongPackageInert,
        ExactInert,
    }

    for (label, case, should_pass) in [
        ("injecting", MarkerCase::Injecting, false),
        ("renamed-inert", MarkerCase::RenamedInert, false),
        ("wrong-package-inert", MarkerCase::WrongPackageInert, false),
        ("exact-inert", MarkerCase::ExactInert, true),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker_package = match case {
            MarkerCase::WrongPackageInert => "alternate-markers",
            _ => "rusty-cpp-markers",
        };
        let marker_dependency_key = match case {
            MarkerCase::RenamedInert => "markers",
            _ => "rusty-cpp-markers",
        };
        let marker_dir = temp.path().join("markers");
        let runtime_dir = temp.path().join("runtime");
        let app_dir = temp.path().join("app");
        std::fs::create_dir_all(marker_dir.join("src")).unwrap();
        std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
        std::fs::create_dir_all(app_dir.join("src")).unwrap();

        std::fs::write(
            marker_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = {marker_package:?}\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\nproc-macro = true\n"
            ),
        )
        .unwrap();
        let marker_source = match case {
            MarkerCase::Injecting => r#"
extern crate proc_macro;
use proc_macro::TokenStream;
#[proc_macro_attribute]
pub fn cpp_inherit(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut output = item;
    output.extend("pub fn route_hidden() -> i32 { crate::renamed(41) }".parse::<TokenStream>().unwrap());
    output
}
"#,
            _ => r#"
//! Rustc-visible inert attributes consumed by rusty-cpp code generation.
use proc_macro::TokenStream;
/// Request direct C++ inheritance.
#[proc_macro_attribute]
pub fn cpp_inherit(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
"#,
        };
        std::fs::write(marker_dir.join("src/lib.rs"), marker_source).unwrap();

        let marker_dependency = if marker_dependency_key == marker_package {
            format!("{marker_dependency_key} = {{ path = \"../markers\" }}")
        } else {
            format!(
                "{marker_dependency_key} = {{ package = {marker_package:?}, path = \"../markers\" }}"
            )
        };
        std::fs::write(
            runtime_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"rusty\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{marker_dependency}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            runtime_dir.join("src/lib.rs"),
            format!("pub use {}::cpp_inherit;\n", marker_dependency_key.replace('-', "_")),
        )
        .unwrap();

        std::fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"cpp_inherit_host\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrusty = { path = \"../runtime\" }\n\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            app_dir.join("src/lib.rs"),
            r#"
mod child;
#[cfg_attr(any(), cpp_name(overloaded))]
pub fn renamed(value: i32) -> i32 { value + 1 }
"#,
        )
        .unwrap();
        std::fs::write(
            app_dir.join("src/child.rs"),
            "use rusty::cpp_inherit;\n#[cpp_inherit]\npub struct Host;\n",
        )
        .unwrap();

        let cargo_check = Command::new("cargo")
            .args(["check", "--offline", "--manifest-path"])
            .arg(app_dir.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(temp.path().join("rust-target"))
            .output()
            .expect("cargo-check cpp_inherit provenance fixture");
        assert!(
            cargo_check.status.success(),
            "{label} fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&cargo_check.stderr)
        );

        let output_dir = temp.path().join("cpp-out");
        let result = run_crate_transpiler(&app_dir.join("Cargo.toml"), &output_dir);
        assert_eq!(
            result.status.success(),
            should_pass,
            "unexpected {label} result:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if should_pass {
            assert!(output_dir.join("cpp_inherit_host.cppm").exists());
        } else {
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("unaudited attribute `cpp_inherit`"),
                "unexpected {label} diagnostic:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(
                !output_dir.exists(),
                "{label} provenance rejection created partial output"
            );
        }
    }
}

#[test]
fn durable_serializable_nested_cpp_inherit_uses_authenticated_facade_preflight() {
    let temp = tempfile::tempdir().expect("durable Serializable provenance tempdir");
    let markers = temp.path().join("markers");
    let runtime = temp.path().join("runtime");
    let app = temp.path().join("app");
    for package in [&markers, &runtime, &app] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        markers.join("Cargo.toml"),
        "[package]\nname='rusty-cpp-markers'\nversion='0.0.0'\nedition='2021'\n[lib]\nproc-macro=true\n",
    )
    .unwrap();
    std::fs::write(
        markers.join("src/lib.rs"),
        r#"//! Rustc-visible inert attributes consumed by rusty-cpp code generation.
use proc_macro::TokenStream;
/// Request direct C++ inheritance.
#[proc_macro_attribute]
pub fn cpp_inherit(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
"#,
    )
    .unwrap();
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.0.0'\nedition='2021'\n[dependencies]\nrusty-cpp-markers={path='../markers'}\n",
    )
    .unwrap();
    std::fs::write(
        runtime.join("src/lib.rs"),
        "pub use rusty_cpp_markers::cpp_inherit;\n",
    )
    .unwrap();
    std::fs::write(
        app.join("Cargo.toml"),
        "[package]\nname='durable_serializable'\nversion='0.0.0'\nedition='2021'\n[dependencies]\nrusty={path='../runtime'}\n[workspace]\nresolver='2'\n",
    )
    .unwrap();
    std::fs::write(app.join("src/lib.rs"), "pub mod serializable;\n").unwrap();
    std::fs::write(
        app.join("src/serializable.rs"),
        r#"#[cfg_attr(any(), cpp_name(make_sink_proxy))]
pub fn make_sink_proxy_value(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(make_sink_proxy))]
pub fn make_sink_proxy_flag(value: bool) -> i32 { if value { 1 } else { 0 } }

pub mod details {
    use rusty::cpp_inherit;
    pub trait SerializableBase {}
    pub struct SerializableSharedPtrHolder;
    #[cpp_inherit]
    impl SerializableBase for SerializableSharedPtrHolder {}
}
"#,
    )
    .unwrap();

    let manifest = app.join("Cargo.toml");
    let cargo_check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(&manifest)
        .arg("--target-dir")
        .arg(temp.path().join("cargo-target"))
        .output()
        .expect("cargo-check durable Serializable provenance fixture");
    assert!(
        cargo_check.status.success(),
        "durable Serializable provenance fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let output_dir = temp.path().join("cpp-output");
    let result = run_crate_transpiler(&manifest, &output_dir);
    assert!(
        result.status.success(),
        "authenticated nested cpp_inherit was rejected before Serializable output:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated =
        std::fs::read_to_string(output_dir.join("durable_serializable.serializable.cppm"))
            .expect("read durable Serializable module");
    assert!(generated.contains("make_sink_proxy("), "{generated}");
    assert!(
        generated.contains("SerializableSharedPtrHolder")
            && generated.contains("SerializableBase"),
        "{generated}"
    );
}

#[test]
fn cpp_name_root_feature_graph_stays_exact_with_fake_std_provenance() {
    let temp = tempfile::tempdir().expect("root feature-graph tempdir");
    let fake_std = temp.path().join("fake-std");
    let bad = temp.path().join("bad");
    let root = temp.path().join("root");
    for package in [&fake_std, &bad, &root] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        fake_std.join("Cargo.toml"),
        "[package]\nname='innocent_package'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='std'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        fake_std.join("src/lib.rs"),
        "#![no_std]\npub fn lookalike() -> i32 { 7 }\n",
    )
    .unwrap();

    std::fs::write(
        bad.join("Cargo.toml"),
        "[package]\nname='bad_dep'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        bad.join("src/lib.rs"),
        r#"#![no_std]
#[cfg_attr(any(), cpp_name(cross_feature_overload))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(cross_feature_overload))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='cross_feature_root'\nversion='0.1.0'\nedition='2024'\n\
         [features]\ndefault=[]\nfake-std=['dep:innocent_package']\ncpp-name=['dep:bad_dep']\n\
         [dependencies]\ninnocent_package={path='../fake-std',optional=true}\nbad_dep={path='../bad',optional=true}\n\
         [workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        r#"#![no_std]
#[cfg(feature = "fake-std")]
extern crate std;

#[cfg(feature = "cpp-name")]
pub fn root() -> i32 { bad_dep::first(1) }
#[cfg(not(feature = "cpp-name"))]
pub fn root() -> i32 { 0 }
"#,
    )
    .unwrap();

    let manifest = root.join("Cargo.toml");
    for (label, features, should_pass) in [
        ("neither", None, true),
        ("fake-std", Some("fake-std"), true),
        ("cpp-name", Some("cpp-name"), false),
        (
            "qualified-cpp-name",
            Some("cross_feature_root/cpp-name"),
            false,
        ),
        ("both", Some("fake-std,cpp-name"), false),
    ] {
        let cargo_target = temp.path().join(format!("cargo-target-{label}"));
        let mut cargo_check = Command::new("cargo");
        cargo_check
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(&cargo_target)
            .arg("--no-default-features");
        if let Some(features) = features {
            cargo_check.arg("--features").arg(features);
        }
        let check = cargo_check.output().expect("cargo-check feature fixture");
        assert!(
            check.status.success(),
            "{label} feature fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );

        let output_dir = temp.path().join(format!("cpp-output-{label}"));
        let mut transpiler = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
        transpiler
            .arg("--crate")
            .arg(&manifest)
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--offline")
            .arg("--no-default-features");
        if let Some(features) = features {
            transpiler.arg("--features").arg(features);
        }
        let result = transpiler.output().expect("run feature-context transpiler");
        assert_eq!(
            result.status.success(),
            should_pass,
            "unexpected {label} feature result:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if should_pass {
            assert!(output_dir.join("cross_feature_root.cppm").is_file());
            assert!(
                !output_dir.join("bad_dep").exists(),
                "inactive cpp_name dependency was recursively generated in {label} lane"
            );
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains(
                    "cpp_name whole local-dependency closure preflight failed before output"
                ) && stderr.contains("cpp_name overload collision")
                    && stderr.contains("bad"),
                "{label} failed for the wrong reason:\n{stderr}"
            );
            assert!(
                !output_dir.exists(),
                "{label} collision created partial crate output"
            );
        }
    }
}

#[test]
fn cpp_name_dependency_features_drive_exact_child_sysroot_provenance() {
    let temp = tempfile::tempdir().expect("dependency provenance feature tempdir");
    let fake_std = temp.path().join("fake-std");
    let poison = temp.path().join("poison");
    let bridge = temp.path().join("bridge");
    let root = temp.path().join("root");
    for package in [&fake_std, &poison, &bridge, &root] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        fake_std.join("Cargo.toml"),
        "[package]\nname='innocent_package'\nversion='0.1.0'\nedition='2024'\n\
         [lib]\nname='std'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        fake_std.join("src/lib.rs"),
        "#![no_std]\npub mod sync { pub struct Arc<T>(pub T); impl<T> Arc<T> { pub fn new(value: T) -> Self { Self(value) } } }\n",
    )
    .unwrap();

    std::fs::write(
        poison.join("Cargo.toml"),
        "[package]\nname='cpp_name_poison'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        poison.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(child_feature_collision))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(child_feature_collision))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();

    std::fs::write(
        bridge.join("Cargo.toml"),
        "[package]\nname='feature_bridge'\nversion='0.1.0'\nedition='2024'\n\
         [features]\ndefault=[]\nfake-sysroot=['dep:innocent_package']\nname-contract=['dep:cpp_name_poison']\n\
         [dependencies]\ninnocent_package={path='../fake-std',optional=true}\ncpp_name_poison={path='../poison',optional=true}\n\
         [workspace]\n",
    )
    .unwrap();
    std::fs::write(
        bridge.join("src/lib.rs"),
        r#"#![no_std]
extern crate std;
use std::sync::Arc;

pub struct Owner { pub value: i32 }
impl Owner {
    #[cfg_attr(any(), cpp_ctor)]
    pub unsafe fn new(value: i32) -> Owner { Owner { value } }
}
pub fn make_owner(value: i32) -> Arc<Owner> {
    Arc::new(unsafe { Owner::new(value) })
}
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='dependency_context_root'\nversion='0.1.0'\nedition='2024'\n\
         [features]\ndefault=[]\nroot-fake=['feature_bridge/fake-sysroot']\nroot-name=['feature_bridge/name-contract']\n\
         [dependencies]\nfeature_bridge={path='../bridge',default-features=false}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        r#"
#[cfg_attr(any(), cpp_name(dependency_context_root_value))]
pub fn root_value() -> i32 { 7 }
"#,
    )
    .unwrap();

    let manifest = root.join("Cargo.toml");
    for (label, features, should_pass, should_fuse) in [
        ("baseline", None, true, true),
        ("fake", Some("root-fake"), true, false),
        ("name", Some("root-name"), false, false),
        ("both", Some("root-fake,root-name"), false, false),
    ] {
        let mut cargo_check = Command::new("cargo");
        cargo_check
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .arg("--target-dir")
            .arg(temp.path().join(format!("cargo-target-{label}")))
            .arg("--no-default-features");
        if let Some(features) = features {
            cargo_check.arg("--features").arg(features);
        }
        let check = cargo_check.output().expect("cargo-check child feature fixture");
        assert!(
            check.status.success(),
            "{label} child feature fixture is not Cargo-valid:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );

        let output_dir = temp.path().join(format!("cpp-output-{label}"));
        let mut transpiler = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
        transpiler
            .arg("--crate")
            .arg(&manifest)
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--offline")
            .arg("--no-default-features");
        if let Some(features) = features {
            transpiler.arg("--features").arg(features);
        }
        let result = transpiler.output().expect("run child feature transpiler");
        assert_eq!(
            result.status.success(),
            should_pass,
            "unexpected {label} child feature result:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if should_pass {
            let bridge_cpp =
                std::fs::read_to_string(output_dir.join("feature_bridge/feature_bridge.cppm"))
                    .unwrap();
            assert_eq!(
                bridge_cpp.contains("rusty::Arc<Owner>::make("),
                should_fuse,
                "{label} used the wrong child sysroot provenance:\n{bridge_cpp}"
            );
            assert!(!output_dir.join("feature_bridge/cpp_name_poison").exists());
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                stderr.contains("cpp_name whole local-dependency closure preflight failed before output")
                    && stderr.contains("child_feature_collision")
                    && stderr.contains("poison"),
                "{label} failed for the wrong reason:\n{stderr}"
            );
            assert!(!output_dir.exists(), "{label} created partial output");
        }
    }
}

#[test]
fn cpp_name_dependency_feature_selector_fails_closed_before_output() {
    let temp = tempfile::tempdir().expect("dependency feature-selector tempdir");
    let poison = temp.path().join("poison");
    let chooser = temp.path().join("chooser");
    let root = temp.path().join("root");
    for package in [&poison, &chooser, &root] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        poison.join("Cargo.toml"),
        "[package]\nname='feature_poison'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        poison.join("src/lib.rs"),
        r#"#[cfg_attr(any(), cpp_name(dependency_feature_collision))]
pub fn first(value: i32) -> i32 { value }
#[cfg_attr(any(), cpp_name(dependency_feature_collision))]
pub fn second(value: i32) -> i32 { value + 1 }
"#,
    )
    .unwrap();

    std::fs::write(
        chooser.join("Cargo.toml"),
        "[package]\nname='feature_chooser'\nversion='0.1.0'\nedition='2024'\n\
         [features]\ndefault=[]\nactivate=['dep:feature_poison']\n\
         [dependencies]\nfeature_poison={path='../poison',optional=true}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        chooser.join("src/lib.rs"),
        "#[cfg(feature=\"activate\")] pub fn value() -> i32 { feature_poison::first(1) }\n\
         #[cfg(not(feature=\"activate\"))] pub fn value() -> i32 { 0 }\n",
    )
    .unwrap();

    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='dependency_feature_root'\nversion='0.1.0'\nedition='2024'\n\
         [dependencies]\nfeature_chooser={path='../chooser',default-features=false}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn root() -> i32 { feature_chooser::value() }\n",
    )
    .unwrap();

    let manifest = root.join("Cargo.toml");
    let selector = "feature_chooser/activate";
    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(&manifest)
        .arg("--features")
        .arg(selector)
        .arg("--target-dir")
        .arg(temp.path().join("cargo-target"))
        .output()
        .expect("cargo-check dependency feature-selector fixture");
    assert!(
        check.status.success(),
        "dependency feature selector is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let output_dir = temp.path().join("cpp-output");
    let result = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(&manifest)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--offline")
        .arg("--features")
        .arg(selector)
        .output()
        .expect("run dependency feature-selector transpiler");
    assert!(!result.status.success(), "dependency feature selector was guessed");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("source-owned C++ contract requires an exact Cargo target-selected normal local-dependency graph before output")
            && stderr.contains("unsupported Cargo dependency feature selector")
            && stderr.contains(selector)
            && stderr.contains("cannot exactly project"),
        "dependency feature selector failed for the wrong reason:\n{stderr}"
    );
    assert!(
        !output_dir.exists(),
        "unsupported dependency feature selector created partial output"
    );
}
