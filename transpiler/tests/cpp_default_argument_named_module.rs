use sha2::{Digest, Sha256};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn find_tool(candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
            .then(|| (*candidate).to_string())
    })
}

fn find_clang() -> Option<String> {
    env::var("CXX")
        .ok()
        .filter(|cxx| !cxx.trim().is_empty())
        .or_else(|| find_tool(&["clang++", "clang++-22", "clang++-21", "clang++-20"]))
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
fn typed_cpp_defaults_compile_and_capture_the_importer_call_site() {
    let Some(clang) = find_clang() else {
        eprintln!("skipping default-argument module gate: no clang++ available");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let rust = temp.path().join("default_argument_fixture.rs");
    std::fs::write(
        &rust,
        r#"
pub fn observed_line(
    #[cfg_attr(any(), cpp_default_argument(source_location))]
    location: &::rusty::SourceLocation,
) -> u32 {
    location.line()
}

pub unsafe fn is_selected_stream_stderr(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) -> bool {
    unsafe { stream == srpc_stderr() }
}

pub unsafe fn canonical_primitive_const_paths(
    bare: [u8; char::MIN as usize],
    std_path: [u8; std::primitive::char::MIN as usize],
    core_absolute_path: [u8; ::core::primitive::char::MIN as usize],
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) -> bool {
    let _ = (bare, std_path, core_absolute_path);
    unsafe { stream == srpc_stderr() }
}

mod core {}

#[allow(unsafe_code)]
unsafe extern "C" { fn srpc_stderr() -> *mut rusty::CFile; }
"#,
    )
    .expect("write Rust fixture");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(
        &type_map,
        r#"[rusty]
SourceLocation = "std::source_location"
CFile = "FILE"
"#,
    )
    .expect("write type map");
    let interface = temp.path().join("default_argument_fixture.cppm");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        r#"version = 1
[[module]]
name = "default_argument_fixture"
includes = [
  { path = "cstdio", form = "angle" },
  { path = "source_location", form = "angle" },
]
"#,
    )
    .expect("write module preamble");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .arg("-o")
        .arg(&interface)
        .arg("-m")
        .arg("default_argument_fixture")
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run transpiler");
    assert_success(&transpile, "default-argument transpilation");

    let generated = std::fs::read_to_string(&interface).expect("read interface");
    assert_eq!(
        generated
            .matches(" = std::source_location::current()")
            .count(),
        1,
        "default must appear on exactly one declaration:\n{generated}"
    );
    assert_eq!(generated.matches(" = stderr").count(), 2);

    let importer = temp.path().join("importer.cpp");
    std::fs::write(
        &importer,
        r#"#include <cstdio>
import default_argument_fixture;
int main() {
  const auto expected = __LINE__ + 1;
  const auto observed = observed_line();
  if (observed != expected) return 1;
  if (!is_selected_stream_stderr()) return 2;
  return canonical_primitive_const_paths({}, {}, {}) ? 0 : 3;
}
"#,
    )
    .expect("write importer");

    let pcm = temp.path().join("default_argument_fixture.pcm");
    let interface_object = temp.path().join("interface.o");
    let seam = temp.path().join("seam.cpp");
    std::fs::write(
        &seam,
        "#include <cstdio>\nextern \"C\" FILE* srpc_stderr(){ return stderr; }\n",
    )
    .expect("write C seam");
    let seam_object = temp.path().join("seam.o");
    let importer_object = temp.path().join("importer.o");
    let binary = temp.path().join("default_argument_fixture");
    let include = project_include_dir();
    let module_map = format!("-fmodule-file=default_argument_fixture={}", pcm.display());
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
        .expect("precompile interface");
    assert_success(&precompile, "module interface precompile");
    for (source, language, object) in [
        (&interface, "c++-module", &interface_object),
        (&importer, "c++", &importer_object),
        (&seam, "c++", &seam_object),
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
        assert_success(&compile, "named-module compile");
    }
    let link = Command::new(&clang)
        .arg(&interface_object)
        .arg(&importer_object)
        .arg(&seam_object)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("link runtime");
    assert_success(&link, "named-module link");
    let run = Command::new(&binary).output().expect("run binary");
    assert_success(&run, "named-module runtime");
}

#[test]
fn crate_mode_rejects_invalid_default_before_creating_output() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");
    let crate_dir = temp.path().join("fixture");
    std::fs::create_dir_all(crate_dir.join("src")).expect("create fixture src");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={path='../rusty'}\n",
    )
    .expect("write manifest");
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"pub fn bad(
    #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *const ::rusty::CFile,
) {}"#,
    )
    .expect("write invalid source");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let output_dir = temp.path().join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run invalid crate");
    assert!(
        !output.status.success(),
        "invalid contract must fail closed"
    );
    assert!(
        !output_dir.exists(),
        "crate preflight must not leave a partial output directory"
    );

    // This contract is valid Rust, but the associated type cannot be rendered
    // as an exact standalone C++ forward-declaration type. The exact codegen
    // dry run must reject it before even crate metadata is written.
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = i32; }
pub fn unresolved_forward_type(
    value: <u32 as ValueType>::Output,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) {
    let _ = (value, stream);
}
"#,
    )
    .expect("write rustc-valid unresolved source");
    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", temp.path().join("cargo-target"))
        .output()
        .expect("cargo check unresolved fixture");
    assert_success(&cargo_check, "rustc-valid unresolved fixture");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write exact preamble");
    let unresolved_output_dir = temp.path().join("unresolved-generated");
    let unresolved = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&unresolved_output_dir)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run unresolved crate");
    assert!(
        !unresolved.status.success(),
        "unresolvable forward declaration must fail closed"
    );
    assert!(
        !unresolved_output_dir.exists(),
        "late exact-codegen rejection left partial crate output"
    );
}

#[test]
fn crate_mode_accepts_const_arithmetic_and_primitive_constants() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");

    let crate_dir = temp.path().join("fixture");
    std::fs::create_dir_all(crate_dir.join("src")).expect("create fixture src");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={path='../rusty'}\n",
    )
    .expect("write fixture manifest");
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"pub fn accepted(
    arithmetic: [u8; (2 + 1) * 2],
    primitive: [u8; char::MAX as usize],
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (arithmetic, primitive, stream); }
"#,
    )
    .expect("write positive source");
    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", temp.path().join("cargo-target"))
        .output()
        .expect("cargo check positive fixture");
    assert_success(&cargo_check, "rustc-valid const-expression positive");

    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write module preamble");
    let generated = temp.path().join("generated");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&generated)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run const-expression positive");
    assert_success(&transpile, "const-expression positive transpilation");
    let output =
        std::fs::read_to_string(generated.join("fixture.cppm")).expect("read positive module");
    assert_eq!(
        output.matches("FILE* stream = stderr").count(),
        1,
        "{output}"
    );
    assert!(
        output.contains("std::array<uint8_t, ((2 + 1)) * 2>"),
        "{output}"
    );
    assert!(
        output.contains("static_cast<char32_t>(0x10FFFF)"),
        "{output}"
    );
    assert!(!output.contains("std::array<auto"), "{output}");
}

#[test]
fn crate_mode_rejects_projected_alias_closures_without_mutating_output() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write module preamble");

    let cases = [
        (
            "direct-parameter",
            r#"
pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = i32; }
pub fn rejected(
    value: <u32 as ValueType>::Output,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            None,
        ),
        (
            "local-alias-chain",
            r#"
pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = i32; }
pub type Projected = <u32 as ValueType>::Output;
pub type First = Projected;
pub type Second = First;
pub fn rejected(
    value: Second,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            None,
        ),
        (
            "nested-parameter-return",
            r#"
pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = i32; }
pub type Projected = <u32 as ValueType>::Output;
pub fn rejected(
    value: Option<(u8, Projected)>,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) -> Result<Projected, ()> { let _ = (value, stream); unreachable!() }
"#,
            None,
        ),
        (
            "cross-file-alias-chain",
            r#"
pub mod aliases;
use crate::aliases::Second as Imported;
pub fn rejected(
    value: Option<Imported>,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            Some(
                r#"
pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = i32; }
pub type Projected = <u32 as ValueType>::Output;
pub type First = Projected;
pub type Second = First;
"#,
            ),
        ),
        (
            "named-const-associated-projection",
            r#"
pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
pub const PROJECTED: usize = <u32 as Width>::VALUE;
pub type Payload = [u8; PROJECTED];
pub fn rejected(
    value: Payload,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            None,
        ),
        (
            "cross-file-named-const-reexports",
            r#"
pub mod aliases;
pub use crate::aliases::SECOND as MID;
pub use self::MID as WIDTH;
pub type Payload = [u8; WIDTH];
pub fn rejected(
    value: Option<Payload>,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            Some(
                r#"
pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
pub const PROJECTED: usize = <u32 as Width>::VALUE;
pub use self::PROJECTED as FIRST;
pub use self::FIRST as SECOND;
"#,
            ),
        ),
        (
            "named-const-macro-closure",
            r#"
macro_rules! width { () => { 4 } }
pub const EXPANDED: usize = width!();
pub type Payload = [u8; EXPANDED];
pub fn rejected(
    value: Payload,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            None,
        ),
        (
            "primitive-name-shadow",
            r#"
#[allow(non_camel_case_types)]
pub struct u8;
pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
impl u8 { pub const MAX: usize = <u32 as Width>::VALUE; }
pub const SHADOWED: usize = u8::MAX;
pub type Payload = [u8; SHADOWED];
pub fn rejected(
    value: Payload,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
            None,
        ),
    ];

    for (name, root_source, aliases_source) in cases {
        let crate_dir = temp.path().join("cases").join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create case src");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={path='../../rusty'}\n",
        )
        .expect("write case manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), root_source).expect("write case root");
        if let Some(aliases_source) = aliases_source {
            std::fs::write(crate_dir.join("src/aliases.rs"), aliases_source)
                .expect("write case aliases");
        }
        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                temp.path().join("cargo-target").join(name),
            )
            .output()
            .expect("cargo check projected fixture");
        assert_success(
            &cargo_check,
            &format!("rustc-valid projected fixture {name}"),
        );

        let absent_output = temp.path().join("absent-output").join(name);
        let absent = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&absent_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run projected absent-output negative");
        assert!(
            !absent.status.success(),
            "projected case {name} was accepted"
        );
        assert!(
            !absent_output.exists(),
            "projected case {name} created an output directory"
        );

        let existing_output = temp.path().join("existing-output").join(name);
        std::fs::create_dir_all(&existing_output).expect("create existing output");
        let sentinel = existing_output.join("keep.txt");
        std::fs::write(&sentinel, format!("preserve-{name}\n")).expect("write sentinel");
        let existing = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&existing_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run projected preexisting-output negative");
        assert!(
            !existing.status.success(),
            "projected case {name} was accepted with preexisting output"
        );
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("read preserved sentinel"),
            format!("preserve-{name}\n")
        );
        let entries = std::fs::read_dir(&existing_output)
            .expect("read existing output")
            .map(|entry| entry.expect("read output entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from("keep.txt")],
            "projected case {name} mutated preexisting output"
        );
    }
}

#[test]
fn crate_mode_rejects_external_origin_and_primitive_spoofs_atomically() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");

    let dependency = temp.path().join("dep");
    std::fs::create_dir_all(dependency.join("src")).expect("create dependency src");
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='dep'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='src/lib.rs'\n",
    )
    .expect("write dependency manifest");
    std::fs::write(
        dependency.join("src/lib.rs"),
        r#"pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 5; }
pub const BAD: usize = <u32 as Width>::VALUE;
"#,
    )
    .expect("write dependency source");

    let libc = temp.path().join("libc");
    std::fs::create_dir_all(libc.join("src")).expect("create libc spoof src");
    std::fs::write(
        libc.join("Cargo.toml"),
        "[package]\nname='libc'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='src/lib.rs'\n",
    )
    .expect("write libc spoof manifest");
    std::fs::write(
        libc.join("src/lib.rs"),
        r#"pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
#[allow(non_camel_case_types)]
pub struct u8;
impl u8 { pub const MAX: usize = <u32 as Width>::VALUE; }
"#,
    )
    .expect("write libc spoof source");

    let cases = [
        (
            "libc-package-lookalike",
            "libc={path='../../libc'}\n",
            r#"pub type Payload = [u8; libc::u8::MAX];
const _: [u8; 4] = [0; libc::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "absolute-use-beats-local-module",
            "dep={path='../../dep'}\n",
            r#"mod dep { pub const BAD: usize = 4; }
use ::dep::BAD as WIDTH;
const _: [u8; 5] = [0; WIDTH];
pub type Payload = [u8; WIDTH];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "absolute-grouped-use",
            "dep={path='../../dep'}\n",
            r#"use ::dep::{BAD as WIDTH};
const _: [u8; 5] = [0; WIDTH];
pub type Payload = [u8; WIDTH];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "absolute-named-reexport-chain",
            "dep={path='../../dep'}\n",
            r#"pub use ::dep::BAD as FIRST;
pub use self::FIRST as WIDTH;
const _: [u8; 5] = [0; WIDTH];
pub type Payload = [u8; WIDTH];
pub fn rejected(value: Option<Payload>, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "absolute-glob",
            "dep={path='../../dep'}\n",
            r#"use ::dep::*;
const _: [u8; 5] = [0; BAD];
pub type Payload = [u8; BAD];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "absolute-glob-reexport-chain",
            "dep={path='../../dep'}\n",
            r#"mod bridge { pub use ::dep::*; }
use crate::bridge::*;
const _: [u8; 5] = [0; BAD];
pub type Payload = [u8; BAD];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "renamed-external-crate",
            "renamed={package='dep',path='../../dep'}\n",
            r#"use ::renamed::BAD as WIDTH;
const _: [u8; 5] = [0; WIDTH];
pub type Payload = [u8; WIDTH];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "extern-crate-alias",
            "dep={path='../../dep'}\n",
            r#"extern crate dep as alias;
use alias::BAD as WIDTH;
const _: [u8; 5] = [0; WIDTH];
pub type Payload = [u8; WIDTH];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "local-std-shadow",
            "",
            r#"trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
mod std { pub mod primitive { #[allow(non_camel_case_types)] pub struct u8; impl u8 { pub const MAX: usize = <u32 as super::super::Width>::VALUE; } } }
pub type Payload = [u8; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "local-core-shadow",
            "",
            r#"trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
mod core { #[allow(non_camel_case_types)] pub struct u8; impl u8 { pub const MAX: usize = <u32 as super::Width>::VALUE; } }
pub type Payload = [u8; core::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "local-libc-shadow",
            "",
            r#"trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
mod libc { #[allow(non_camel_case_types)] pub struct u8; impl u8 { pub const MAX: usize = <u32 as super::Width>::VALUE; } }
pub type Payload = [u8; libc::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
        (
            "local-primitive-glob-shadow",
            "",
            r#"trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
mod primitive { #[allow(non_camel_case_types)] pub struct u8; impl u8 { pub const MAX: usize = <u32 as super::Width>::VALUE; } }
use primitive::*;
pub type Payload = [::core::primitive::u8; u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
        ),
    ];

    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write module preamble");

    for (name, extra_dependencies, source) in cases {
        let crate_dir = temp.path().join("cases").join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create case src");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={{path='../../rusty'}}\n{extra_dependencies}"
            ),
        )
        .expect("write case manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), source).expect("write case source");

        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                temp.path().join("cargo-target").join(name),
            )
            .output()
            .expect("cargo check origin fixture");
        assert_success(&cargo_check, &format!("rustc-valid origin fixture {name}"));

        let absent_output = temp.path().join("absent-origin-output").join(name);
        let absent = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&absent_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run absent-output origin negative");
        assert!(!absent.status.success(), "origin case {name} was accepted");
        assert!(
            !absent_output.exists(),
            "origin case {name} created an output directory"
        );

        let existing_output = temp.path().join("existing-origin-output").join(name);
        std::fs::create_dir_all(&existing_output).expect("create existing output");
        let sentinel = existing_output.join("keep.txt");
        std::fs::write(&sentinel, format!("preserve-{name}\n")).expect("write sentinel");
        let existing = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&existing_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run preexisting-output origin negative");
        assert!(
            !existing.status.success(),
            "origin case {name} was accepted with preexisting output"
        );
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("read preserved sentinel"),
            format!("preserve-{name}\n")
        );
        let entries = std::fs::read_dir(&existing_output)
            .expect("read existing output")
            .map(|entry| entry.expect("read output entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from("keep.txt")],
            "origin case {name} mutated preexisting output"
        );
    }
}

#[test]
fn crate_mode_rejects_macro_generated_binding_surfaces_atomically() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n[dependencies]\nbinding_macros={package='binding-macros',path='../binding_macros'}\n",
    )
    .expect("write runtime manifest");
    std::fs::write(
        runtime.join("src/lib.rs"),
        "pub use binding_macros::bind_std as cpp_inherit;\npub struct CFile;\n",
    )
    .expect("write runtime source");

    let fake_std = temp.path().join("fake_std");
    std::fs::create_dir_all(fake_std.join("src")).expect("create fake std src");
    std::fs::write(
        fake_std.join("Cargo.toml"),
        "[package]\nname='fake-std'\nversion='0.1.0'\nedition='2024'\n[lib]\npath='src/lib.rs'\n",
    )
    .expect("write fake std manifest");
    std::fs::write(
        fake_std.join("src/lib.rs"),
        r#"pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
pub mod primitive {
    #[allow(non_camel_case_types)] pub struct u8;
    impl u8 { pub const MAX: usize = <u32 as crate::Width>::VALUE; }
}
"#,
    )
    .expect("write fake std source");

    let binding_macros = temp.path().join("binding_macros");
    std::fs::create_dir_all(binding_macros.join("src")).expect("create proc macro src");
    std::fs::write(
        binding_macros.join("Cargo.toml"),
        "[package]\nname='binding-macros'\nversion='0.1.0'\nedition='2024'\n[lib]\nproc-macro=true\npath='src/lib.rs'\n",
    )
    .expect("write proc macro manifest");
    std::fs::write(
        binding_macros.join("src/lib.rs"),
        r#"use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn bind_std(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    format!("{item} use fake_std as std;").parse().unwrap()
}

#[proc_macro_attribute]
pub fn test(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    format!("{item} use fake_std as std;").parse().unwrap()
}

#[proc_macro_derive(BindStd)]
pub fn derive_bind_std(_item: TokenStream) -> TokenStream {
    "use fake_std as std;".parse().unwrap()
}

#[proc_macro]
pub fn bind_std_item(_input: TokenStream) -> TokenStream {
    "use fake_std as std;".parse().unwrap()
}
"#,
    )
    .expect("write proc macro source");

    let tool_macros = temp.path().join("tool_macros");
    std::fs::create_dir_all(tool_macros.join("src")).expect("create tool proc macro src");
    std::fs::write(
        tool_macros.join("Cargo.toml"),
        "[package]\nname='tool-macros'\nversion='0.1.0'\nedition='2024'\n[lib]\nproc-macro=true\npath='src/lib.rs'\n",
    )
    .expect("write tool proc macro manifest");
    std::fs::write(
        tool_macros.join("src/lib.rs"),
        r#"use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn bind_std(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    format!("{item} use fake_std as std;").parse().unwrap()
}
"#,
    )
    .expect("write tool proc macro source");

    let cases: Vec<(&str, &str, Vec<(&str, &str)>, &str)> = vec![
        (
            "macro-use-shadow",
            r#"macro_rules! bind_std { () => { use fake_std as std; }; }
bind_std!();
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "macro-extern-crate-shadow",
            r#"macro_rules! bind_external { () => { extern crate fake_std as alternate; }; }
bind_external!();
pub type Payload = [u8; alternate::primitive::u8::MAX];
const _: [u8; 4] = [0; alternate::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "macro-type-alias",
            r#"pub trait ValueType { type Output; }
impl ValueType for u32 { type Output = u32; }
macro_rules! make_alias { () => { pub type Hidden = <u32 as ValueType>::Output; }; }
make_alias!();
pub fn rejected(value: Hidden, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "macro-named-const",
            r#"pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
macro_rules! make_const { () => { pub const HIDDEN: usize = <u32 as Width>::VALUE; }; }
make_const!();
pub type Payload = [u8; HIDDEN];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "macro-module-shadow",
            r#"pub trait Width { const VALUE: usize; }
impl Width for u32 { const VALUE: usize = 4; }
macro_rules! make_std { () => { mod std { pub mod primitive { #[allow(non_camel_case_types)] pub struct u8; impl u8 { pub const MAX: usize = <u32 as crate::Width>::VALUE; } } } }; }
make_std!();
pub type Payload = [u8; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "wrapper-macro",
            r#"macro_rules! bind_std { () => { use fake_std as std; }; }
macro_rules! wrapper { () => { bind_std!(); }; }
wrapper!();
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "include-item-macro",
            r#"include!("bindings.rs");
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![("bindings.rs", "use fake_std as std;\n")],
            "fixture",
        ),
        (
            "proc-attribute",
            r#"use binding_macros::bind_std;
#[bind_std]
pub struct Anchor;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "tool-namespace-spoof-proc-attribute",
            r#"#[clippy::bind_std]
pub struct Anchor;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "builtin-attribute-spelling-spoof",
            r#"use binding_macros::test;
#[test]
pub struct Anchor;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "proc-derive",
            r#"use binding_macros::BindStd;
#[derive(BindStd)]
pub struct Anchor;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "function-like-proc-macro",
            r#"binding_macros::bind_std_item!();
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "active-cfg-attribute-proc-macro",
            r#"#[cfg_attr(not(any()), binding_macros::bind_std)]
pub struct Anchor;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "trusted-attribute-spelling-alias",
            r#"use rusty::cpp_inherit;
pub trait Trait {}
pub struct Anchor;
#[cpp_inherit]
impl Trait for Anchor {}
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![],
            "fixture",
        ),
        (
            "ancestor-trusted-attribute-spoof",
            r#"use rusty::cpp_inherit;
pub trait Trait {}
pub struct Anchor;
#[cpp_inherit]
impl Trait for Anchor {}
pub mod child;
"#,
            vec![(
                "child.rs",
                r#"use super::std;
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            )],
            "fixture.child",
        ),
        (
            "non-marker-sibling-macro-closure",
            r#"pub mod generated;
use crate::generated::Payload;
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            vec![(
                "generated.rs",
                r#"macro_rules! bind_std { () => { use fake_std as std; }; }
bind_std!();
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
"#,
            )],
            "fixture",
        ),
        (
            "out-of-line-marker-module",
            "pub mod child;\n",
            vec![(
                "child.rs",
                r#"macro_rules! bind_std { () => { use fake_std as std; }; }
bind_std!();
pub type Payload = [u8; std::primitive::u8::MAX];
const _: [u8; 4] = [0; std::primitive::u8::MAX];
pub fn rejected(value: Payload, #[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut ::rusty::CFile) { let _ = (value, stream); }
"#,
            )],
            "fixture.child",
        ),
    ];

    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    for (name, root_source, extra_sources, preamble_module) in cases {
        let crate_dir = temp.path().join("cases").join(name);
        std::fs::create_dir_all(crate_dir.join("src")).expect("create case src");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={path='../../rusty'}\nfake_std={package='fake-std',path='../../fake_std'}\nbinding_macros={package='binding-macros',path='../../binding_macros'}\nclippy={package='tool-macros',path='../../tool_macros'}\n",
        )
        .expect("write case manifest");
        std::fs::write(crate_dir.join("src/lib.rs"), root_source).expect("write case root");
        for (path, source) in extra_sources {
            std::fs::write(crate_dir.join("src").join(path), source).expect("write extra source");
        }

        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(crate_dir.join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                temp.path().join("cargo-target").join(name),
            )
            .output()
            .expect("cargo check macro-binding fixture");
        assert_success(
            &cargo_check,
            &format!("rustc-valid macro-binding fixture {name}"),
        );

        let preamble = temp.path().join(format!("{name}-module-preamble.toml"));
        std::fs::write(
            &preamble,
            format!(
                "version=1\n[[module]]\nname='{preamble_module}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write case module preamble");
        let absent_output = temp.path().join("absent-macro-output").join(name);
        let absent = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&absent_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run absent-output macro negative");
        assert!(!absent.status.success(), "macro case {name} was accepted");
        assert!(
            String::from_utf8_lossy(&absent.stderr).contains("macro-generated bindings"),
            "macro case {name} failed for the wrong reason: {}",
            String::from_utf8_lossy(&absent.stderr)
        );
        assert!(
            !absent_output.exists(),
            "macro case {name} created an output directory"
        );

        let existing_output = temp.path().join("existing-macro-output").join(name);
        std::fs::create_dir_all(&existing_output).expect("create existing output");
        let sentinel = existing_output.join("keep.txt");
        std::fs::write(&sentinel, format!("preserve-{name}\n")).expect("write sentinel");
        let existing = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(crate_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&existing_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run preexisting-output macro negative");
        assert!(
            !existing.status.success(),
            "macro case {name} was accepted with preexisting output"
        );
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("read preserved sentinel"),
            format!("preserve-{name}\n")
        );
        let entries = std::fs::read_dir(&existing_output)
            .expect("read existing output")
            .map(|entry| entry.expect("read output entry").file_name())
            .collect::<Vec<_>>();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from("keep.txt")],
            "macro case {name} mutated preexisting output"
        );
    }
}

#[test]
fn crate_mode_rejects_local_dependency_codegen_failures_atomically() {
    let temp = tempfile::tempdir().expect("create temp dir");

    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");

    let leaf = temp.path().join("leaf");
    std::fs::create_dir_all(leaf.join("src")).expect("create failing leaf src");
    std::fs::write(
        leaf.join("Cargo.toml"),
        "[package]\nname='leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='leaf'\npath='src/lib.rs'\n",
    )
    .expect("write failing leaf manifest");
    std::fs::write(
        leaf.join("src/lib.rs"),
        r#"pub mod cpp { pub mod missing { pub fn value() -> i32 { 7 } } }
use cpp::missing;
pub fn leaf_value() -> i32 { missing::value() }
"#,
    )
    .expect("write Rust-valid, transpiler-invalid leaf");

    let bridge = temp.path().join("bridge");
    std::fs::create_dir_all(bridge.join("src")).expect("create bridge src");
    std::fs::write(
        bridge.join("Cargo.toml"),
        "[package]\nname='bridge'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='bridge'\npath='src/lib.rs'\n[dependencies]\nleaf={path='../leaf'}\n",
    )
    .expect("write bridge manifest");
    std::fs::write(
        bridge.join("src/lib.rs"),
        "pub fn bridge_value() -> i32 { leaf::leaf_value() }\n",
    )
    .expect("write bridge source");

    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    for (name, dependency) in [("direct", "leaf"), ("transitive", "bridge")] {
        let crate_name = format!("{name}_root");
        let root = temp.path().join("cases").join(name);
        std::fs::create_dir_all(root.join("src")).expect("create root src");
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{crate_name}'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='{crate_name}'\npath='src/lib.rs'\n[dependencies]\nrusty={{path='../../rusty'}}\n{dependency}={{path='../../{dependency}'}}\n"
            ),
        )
        .expect("write root manifest");
        std::fs::write(
            root.join("src/lib.rs"),
            r#"pub fn api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#,
        )
        .expect("write default-bearing root");
        let cargo_check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                temp.path().join("cargo-target").join(name),
            )
            .output()
            .expect("cargo check dependency-atomic fixture");
        assert_success(
            &cargo_check,
            &format!("rustc-valid dependency-atomic fixture {name}"),
        );

        let preamble = temp.path().join(format!("{name}-preamble.toml"));
        std::fs::write(
            &preamble,
            format!(
                "version=1\n[[module]]\nname='{crate_name}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write dependency-atomic preamble");
        let absent_output = temp.path().join("absent-dependency-output").join(name);
        let absent = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(root.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&absent_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run absent-output dependency negative");
        assert!(
            !absent.status.success(),
            "dependency case {name} was accepted"
        );
        let absent_stderr = String::from_utf8_lossy(&absent.stderr);
        assert!(
            absent_stderr.contains(
                "source-owned C++ contract dependency codegen preflight failed before output"
            ) && absent_stderr.contains("no C++ module symbol index is configured"),
            "dependency case {name} failed for the wrong reason: {absent_stderr}"
        );
        assert!(
            !absent_output.exists(),
            "dependency case {name} created an output directory"
        );

        let existing_output = temp.path().join("existing-dependency-output").join(name);
        std::fs::create_dir_all(&existing_output).expect("create existing output");
        let sentinel = existing_output.join("keep.txt");
        std::fs::write(&sentinel, format!("preserve-{name}\n")).expect("write sentinel");
        let existing = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(root.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&existing_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .output()
            .expect("run preexisting-output dependency negative");
        assert!(
            !existing.status.success(),
            "dependency case {name} was accepted with preexisting output"
        );
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("read preserved sentinel"),
            format!("preserve-{name}\n")
        );
        let mut entries = std::fs::read_dir(&existing_output)
            .expect("read existing output")
            .map(|entry| entry.expect("read output entry").file_name())
            .collect::<Vec<_>>();
        entries.sort();
        assert_eq!(
            entries,
            vec![std::ffi::OsString::from("keep.txt")],
            "dependency case {name} mutated preexisting output"
        );
    }

    let workspace = temp.path().join("workspace");
    for member in ["root", "rusty", "leaf"] {
        std::fs::create_dir_all(workspace.join(member).join("src"))
            .expect("create workspace member src");
    }
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers=['root','rusty','leaf']\nresolver='2'\n[workspace.dependencies]\nrusty={path='rusty'}\nleaf={path='leaf'}\n",
    )
    .expect("write workspace manifest");
    std::fs::write(
        workspace.join("rusty/Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write workspace runtime manifest");
    std::fs::write(workspace.join("rusty/src/lib.rs"), "pub struct CFile;\n")
        .expect("write workspace runtime source");
    std::fs::write(
        workspace.join("leaf/Cargo.toml"),
        "[package]\nname='leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='leaf'\npath='src/lib.rs'\n",
    )
    .expect("write workspace leaf manifest");
    std::fs::write(
        workspace.join("leaf/src/lib.rs"),
        r#"pub mod cpp { pub mod missing { pub fn value() -> i32 { 7 } } }
use cpp::missing;
pub fn leaf_value() -> i32 { missing::value() }
"#,
    )
    .expect("write workspace leaf source");
    std::fs::write(
        workspace.join("root/Cargo.toml"),
        "[package]\nname='workspace_root'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='workspace_root'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\nleaf.workspace=true\n",
    )
    .expect("write workspace root manifest");
    std::fs::write(
        workspace.join("root/src/lib.rs"),
        r#"pub fn api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#,
    )
    .expect("write workspace root source");
    let workspace_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(workspace.join("root/Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            temp.path().join("cargo-target/workspace"),
        )
        .output()
        .expect("cargo check workspace-inherited fixture");
    assert_success(&workspace_check, "workspace-inherited dependency fixture");
    let workspace_preamble = workspace.join("preamble.toml");
    std::fs::write(
        &workspace_preamble,
        "version=1\n[[module]]\nname='workspace_root'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write workspace preamble");
    let workspace_output = workspace.join("absent-output");
    let workspace_failure = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(workspace.join("root/Cargo.toml"))
        .arg("--output-dir")
        .arg(&workspace_output)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&workspace_preamble)
        .output()
        .expect("run workspace-inherited dependency negative");
    assert!(
        !workspace_failure.status.success(),
        "workspace-inherited dependency failure was accepted"
    );
    assert!(
        String::from_utf8_lossy(&workspace_failure.stderr)
            .contains("dependency codegen preflight failed before output"),
        "workspace dependency failed for wrong reason: {}",
        String::from_utf8_lossy(&workspace_failure.stderr)
    );
    assert!(
        !workspace_output.exists(),
        "workspace dependency failure created output"
    );

    let marker_free = temp.path().join("marker-free");
    std::fs::create_dir_all(marker_free.join("src")).expect("create marker-free src");
    std::fs::write(
        marker_free.join("Cargo.toml"),
        "[package]\nname='marker_free'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='marker_free'\npath='src/lib.rs'\n[dependencies]\nleaf={path='../leaf'}\n",
    )
    .expect("write marker-free manifest");
    std::fs::write(marker_free.join("src/lib.rs"), "pub fn api() {}\n")
        .expect("write marker-free source");
    let legacy_output = temp.path().join("marker-free-output");
    let legacy = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(marker_free.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&legacy_output)
        .output()
        .expect("run marker-free dependency fixture");
    assert_success(&legacy, "marker-free dependency behavior");
    assert!(legacy_output.join("marker_free.cppm").is_file());
}

#[test]
fn crate_mode_uses_one_cargo_selected_target_dependency_graph_atomically() {
    let rustc_version = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    assert_success(&rustc_version, "rustc host query");
    let rustc_version = String::from_utf8_lossy(&rustc_version.stdout);
    if !rustc_version
        .lines()
        .any(|line| line == "host: x86_64-unknown-linux-gnu")
    {
        eprintln!("skipping Linux target-selection gate on non-x86_64-unknown-linux-gnu test host");
        return;
    }

    let temp = tempfile::tempdir().expect("create target-selection temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime source");

    let leaf = temp.path().join("leaf");
    std::fs::create_dir_all(leaf.join("src")).expect("create selected leaf src");
    std::fs::write(
        leaf.join("Cargo.toml"),
        "[package]\nname='leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='leaf'\npath='src/lib.rs'\n",
    )
    .expect("write selected leaf manifest");
    std::fs::write(
        leaf.join("src/lib.rs"),
        r#"pub mod cpp { pub mod missing { pub fn value() -> i32 { 7 } } }
use cpp::missing;
pub fn leaf_value() -> i32 { missing::value() }
"#,
    )
    .expect("write Rust-valid, transpiler-invalid selected leaf");

    let bridge = temp.path().join("bridge");
    std::fs::create_dir_all(bridge.join("src")).expect("create transitive bridge src");
    std::fs::write(
        bridge.join("Cargo.toml"),
        "[package]\nname='bridge'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='bridge'\npath='src/lib.rs'\n[target.'cfg(target_os = \"linux\")'.dependencies]\nleaf={path='../leaf'}\n",
    )
    .expect("write transitive target dependency manifest");
    std::fs::write(
        bridge.join("src/lib.rs"),
        "#[cfg(target_os=\"linux\")] pub fn bridge_value() -> i32 { leaf::leaf_value() }\n#[cfg(not(target_os=\"linux\"))] pub fn bridge_value() -> i32 { 0 }\n",
    )
    .expect("write transitive bridge source");

    let contract_leaf = temp.path().join("contract-leaf");
    std::fs::create_dir_all(contract_leaf.join("src")).expect("create target contract leaf src");
    std::fs::write(
        contract_leaf.join("Cargo.toml"),
        "[package]\nname='contract_leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='contract_leaf'\npath='src/lib.rs'\n[dependencies]\nrusty={path='../rusty'}\n",
    )
    .expect("write target contract leaf manifest");
    std::fs::write(
        contract_leaf.join("src/lib.rs"),
        r#"pub fn dependency_api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#,
    )
    .expect("write target dependency-owned contract");

    let good_leaf = temp.path().join("good-leaf");
    std::fs::create_dir_all(good_leaf.join("src")).expect("create good target leaf src");
    std::fs::write(
        good_leaf.join("Cargo.toml"),
        "[package]\nname='good_leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='good_leaf'\npath='src/lib.rs'\n",
    )
    .expect("write good target leaf manifest");
    std::fs::write(
        good_leaf.join("src/lib.rs"),
        "pub fn good_value() -> i32 { 11 }\n",
    )
    .expect("write good target leaf source");

    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let root_source = r#"pub fn api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#;
    let selected_cases = [
        (
            "target_unix",
            "[dependencies]\nrusty={path='../../rusty'}\n[target.'cfg(unix)'.dependencies]\nleaf={path='../../leaf'}\n",
        ),
        (
            "target_os_linux",
            "[dependencies]\nrusty={path='../../rusty'}\n[target.'cfg(target_os = \"linux\")'.dependencies]\nleaf={path='../../leaf'}\n",
        ),
        (
            "target_literal",
            "[dependencies]\nrusty={path='../../rusty'}\n[target.x86_64-unknown-linux-gnu.dependencies]\nleaf={path='../../leaf'}\n",
        ),
        (
            "feature_selected_optional",
            "[dependencies]\nrusty={path='../../rusty'}\nleaf={path='../../leaf',optional=true}\n[features]\ndefault=['leaf']\n",
        ),
        (
            "selected_transitive_target",
            "[dependencies]\nrusty={path='../../rusty'}\nbridge={path='../../bridge'}\n",
        ),
    ];

    let cargo_target = temp.path().join("cargo-target");
    for (name, dependency_tables) in selected_cases {
        let root = temp.path().join("cases").join(name);
        std::fs::create_dir_all(root.join("src")).expect("create selected root src");
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='{name}'\npath='src/lib.rs'\n{dependency_tables}"
            ),
        )
        .expect("write selected root manifest");
        std::fs::write(root.join("src/lib.rs"), root_source).expect("write selected root source");
        let check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("cargo check selected target fixture");
        assert_success(&check, &format!("Cargo-valid selected fixture {name}"));

        let preamble = root.join("preamble.toml");
        std::fs::write(
            &preamble,
            format!(
                "version=1\n[[module]]\nname='{name}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write selected fixture preamble");
        for existing in [false, true] {
            let lane = if existing { "existing" } else { "absent" };
            let output_dir = temp.path().join("selected-output").join(name).join(lane);
            if existing {
                std::fs::create_dir_all(&output_dir).expect("create sentinel output");
                std::fs::write(output_dir.join("keep.txt"), format!("keep-{name}\n"))
                    .expect("write sentinel");
            }
            let failure = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
                .arg("--crate")
                .arg(root.join("Cargo.toml"))
                .arg("--output-dir")
                .arg(&output_dir)
                .arg("--type-map")
                .arg(&type_map)
                .arg("--module-preamble")
                .arg(&preamble)
                .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
                .output()
                .expect("run selected target dependency negative");
            assert!(
                !failure.status.success(),
                "selected dependency case {name}/{lane} was accepted"
            );
            let stderr = String::from_utf8_lossy(&failure.stderr);
            assert!(
                stderr.contains("dependency codegen preflight failed before output")
                    && stderr.contains("no C++ module symbol index is configured"),
                "selected dependency case {name}/{lane} failed for the wrong reason: {stderr}"
            );
            if existing {
                assert_eq!(
                    std::fs::read_to_string(output_dir.join("keep.txt"))
                        .expect("read preserved target sentinel"),
                    format!("keep-{name}\n")
                );
                let entries = std::fs::read_dir(&output_dir)
                    .expect("read preserved target output")
                    .map(|entry| entry.expect("read preserved entry").file_name())
                    .collect::<Vec<_>>();
                assert_eq!(entries, vec![std::ffi::OsString::from("keep.txt")]);
            } else {
                assert!(
                    !output_dir.exists(),
                    "selected dependency case {name} created absent output"
                );
            }
        }
    }

    let dependency_contract_root = temp.path().join("dependency-contract-root");
    std::fs::create_dir_all(dependency_contract_root.join("src"))
        .expect("create dependency-contract root src");
    std::fs::write(
        dependency_contract_root.join("Cargo.toml"),
        "[package]\nname='dependency_contract_root'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='dependency_contract_root'\npath='src/lib.rs'\n[target.'cfg(unix)'.dependencies]\ncontract_leaf={package='contract_leaf',path='../contract-leaf'}\n",
    )
    .expect("write dependency-contract root manifest");
    std::fs::write(
        dependency_contract_root.join("src/lib.rs"),
        "pub fn root_value() -> i32 { 1 }\n",
    )
    .expect("write marker-free dependency-contract root source");
    let dependency_contract_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(dependency_contract_root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &cargo_target)
        .output()
        .expect("cargo check selected dependency-contract fixture");
    assert_success(
        &dependency_contract_check,
        "Cargo-valid selected dependency-contract fixture",
    );
    for existing in [false, true] {
        let lane = if existing { "existing" } else { "absent" };
        let output_dir = temp.path().join(format!("dependency-contract-{lane}"));
        if existing {
            std::fs::create_dir_all(&output_dir).expect("create dependency-contract sentinel");
            std::fs::write(output_dir.join("keep.txt"), "contract-keep\n")
                .expect("write dependency-contract sentinel");
        }
        let failure = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(dependency_contract_root.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
            .output()
            .expect("run selected dependency-contract negative");
        assert!(!failure.status.success());
        let stderr = String::from_utf8_lossy(&failure.stderr);
        assert!(
            stderr.contains("local dependency")
                && stderr.contains("contains source-owned C++ contracts"),
            "selected dependency contract was not found by closure scan: {stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read_to_string(output_dir.join("keep.txt"))
                    .expect("read dependency-contract sentinel"),
                "contract-keep\n"
            );
            let entries = std::fs::read_dir(&output_dir)
                .expect("read dependency-contract output")
                .map(|entry| entry.expect("read dependency-contract entry").file_name())
                .collect::<Vec<_>>();
            assert_eq!(entries, vec![std::ffi::OsString::from("keep.txt")]);
        } else {
            assert!(!output_dir.exists());
        }
    }

    let successful_target_root = temp.path().join("successful-target-root");
    std::fs::create_dir_all(successful_target_root.join("src"))
        .expect("create successful target root src");
    std::fs::write(
        successful_target_root.join("Cargo.toml"),
        "[package]\nname='successful_target_root'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='successful_target_root'\npath='src/lib.rs'\n[dependencies]\nrusty={path='../rusty'}\n[target.'cfg(unix)'.dependencies]\ngood_leaf={package='good_leaf',path='../good-leaf'}\n",
    )
    .expect("write successful target root manifest");
    std::fs::write(successful_target_root.join("src/lib.rs"), root_source)
        .expect("write successful target root source");
    let successful_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(successful_target_root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &cargo_target)
        .output()
        .expect("cargo check successful target generation fixture");
    assert_success(
        &successful_check,
        "Cargo-valid successful target generation fixture",
    );
    let successful_preamble = successful_target_root.join("preamble.toml");
    std::fs::write(
        &successful_preamble,
        "version=1\n[[module]]\nname='successful_target_root'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write successful target preamble");
    let successful_output = successful_target_root.join("output");
    let successful = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(successful_target_root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&successful_output)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&successful_preamble)
        .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
        .output()
        .expect("run successful target recursive generation");
    assert_success(&successful, "selected target recursive generation");
    assert!(
        successful_output
            .join("successful_target_root.cppm")
            .is_file()
    );
    assert!(
        successful_output.join("good_leaf/good_leaf.cppm").is_file(),
        "selected target dependency was not recursively generated"
    );

    let workspace = temp.path().join("workspace");
    for member in ["root", "rusty", "leaf"] {
        std::fs::create_dir_all(workspace.join(member).join("src"))
            .expect("create target workspace member");
    }
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers=['root','rusty','leaf']\nresolver='2'\n[workspace.dependencies]\nrusty={path='rusty'}\nrenamed={package='leaf',path='leaf'}\n",
    )
    .expect("write target workspace manifest");
    std::fs::write(
        workspace.join("rusty/Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write target workspace runtime manifest");
    std::fs::write(workspace.join("rusty/src/lib.rs"), "pub struct CFile;\n")
        .expect("write target workspace runtime source");
    std::fs::write(
        workspace.join("leaf/Cargo.toml"),
        "[package]\nname='leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='leaf'\npath='src/lib.rs'\n",
    )
    .expect("write target workspace leaf manifest");
    std::fs::write(
        workspace.join("leaf/src/lib.rs"),
        r#"pub mod cpp { pub mod missing { pub fn value() -> i32 { 7 } } }
use cpp::missing;
pub fn leaf_value() -> i32 { missing::value() }
"#,
    )
    .expect("write target workspace invalid leaf");
    std::fs::write(
        workspace.join("root/Cargo.toml"),
        "[package]\nname='workspace_target_root'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='workspace_target_root'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\n[target.'cfg(target_os = \"linux\")'.dependencies]\nrenamed.workspace=true\n",
    )
    .expect("write workspace-inherited renamed target manifest");
    std::fs::write(workspace.join("root/src/lib.rs"), root_source)
        .expect("write workspace target root source");
    let workspace_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(workspace.join("root/Cargo.toml"))
        .env("CARGO_TARGET_DIR", &cargo_target)
        .output()
        .expect("cargo check workspace target fixture");
    assert_success(
        &workspace_check,
        "Cargo-valid workspace-inherited renamed target fixture",
    );
    let workspace_preamble = workspace.join("preamble.toml");
    std::fs::write(
        &workspace_preamble,
        "version=1\n[[module]]\nname='workspace_target_root'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write workspace target preamble");
    for existing in [false, true] {
        let lane = if existing { "existing" } else { "absent" };
        let output_dir = workspace.join(format!("{lane}-output"));
        if existing {
            std::fs::create_dir_all(&output_dir).expect("create workspace sentinel output");
            std::fs::write(output_dir.join("keep.txt"), "workspace-keep\n")
                .expect("write workspace sentinel");
        }
        let failure = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(workspace.join("root/Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&workspace_preamble)
            .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
            .output()
            .expect("run workspace target dependency negative");
        assert!(!failure.status.success());
        let stderr = String::from_utf8_lossy(&failure.stderr);
        assert!(
            stderr.contains("dependency 'renamed' codegen failed before output"),
            "workspace alias identity was not preserved: {stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read_to_string(output_dir.join("keep.txt"))
                    .expect("read workspace sentinel"),
                "workspace-keep\n"
            );
            let entries = std::fs::read_dir(&output_dir)
                .expect("read workspace output")
                .map(|entry| entry.expect("read workspace entry").file_name())
                .collect::<Vec<_>>();
            assert_eq!(entries, vec![std::ffi::OsString::from("keep.txt")]);
        } else {
            assert!(!output_dir.exists());
        }
    }

    for (name, dependency_tables) in [
        (
            "unselected_target",
            "[dependencies]\nrusty={path='../../rusty'}\n[target.'cfg(windows)'.dependencies]\nleaf={path='../../leaf'}\n",
        ),
        (
            "unselected_optional",
            "[dependencies]\nrusty={path='../../rusty'}\nleaf={path='../../leaf',optional=true}\n[features]\ndefault=[]\n",
        ),
    ] {
        let root = temp.path().join("controls").join(name);
        std::fs::create_dir_all(root.join("src")).expect("create unselected control src");
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='{name}'\npath='src/lib.rs'\n{dependency_tables}"
            ),
        )
        .expect("write unselected control manifest");
        std::fs::write(root.join("src/lib.rs"), root_source)
            .expect("write unselected control source");
        let check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("cargo check unselected control");
        assert_success(&check, &format!("Cargo-valid unselected control {name}"));
        let preamble = root.join("preamble.toml");
        std::fs::write(
            &preamble,
            format!(
                "version=1\n[[module]]\nname='{name}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write unselected control preamble");
        let output_dir = root.join("output");
        std::fs::create_dir_all(&output_dir).expect("create unselected sentinel output");
        std::fs::write(output_dir.join("keep.txt"), format!("keep-{name}\n"))
            .expect("write unselected sentinel");
        let success = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(root.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(&preamble)
            .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
            .output()
            .expect("run unselected target control");
        assert_success(&success, &format!("unselected dependency control {name}"));
        assert_eq!(
            std::fs::read_to_string(output_dir.join("keep.txt")).expect("read unselected sentinel"),
            format!("keep-{name}\n")
        );
        assert!(
            !output_dir.join("leaf").exists(),
            "unselected dependency {name} was generated"
        );
    }

    let unknown_root = temp.path().join("cases/target_unix");
    let unknown_output = temp.path().join("unknown-target-output");
    let unknown = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(unknown_root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&unknown_output)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(unknown_root.join("preamble.toml"))
        .env_remove("CARGO_BUILD_TARGET")
        .env("CARGO_HOME", temp.path().join("isolated-cargo-home"))
        .env("RUSTC", temp.path().join("missing-rustc"))
        .output()
        .expect("run unknown-target fail-closed fixture");
    assert!(!unknown.status.success());
    assert!(
        String::from_utf8_lossy(&unknown.stderr)
            .contains("requires an exact Cargo target-selected normal local-dependency graph"),
        "unknown target failed for the wrong reason: {}",
        String::from_utf8_lossy(&unknown.stderr)
    );
    assert!(!unknown_output.exists());
}

#[test]
fn crate_mode_ignores_contracts_in_unselected_optional_and_target_dependencies() {
    let rustc_version = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    assert_success(&rustc_version, "rustc host query");
    if !String::from_utf8_lossy(&rustc_version.stdout)
        .lines()
        .any(|line| line == "host: x86_64-unknown-linux-gnu")
    {
        eprintln!("skipping Linux unselected-contract gate on non-Linux test host");
        return;
    }

    let temp = tempfile::tempdir().expect("create unselected-contract temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime facade src");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime facade manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime facade source");

    let contract_leaf = temp.path().join("contract-leaf");
    std::fs::create_dir_all(contract_leaf.join("src")).expect("create contract leaf src");
    std::fs::write(
        contract_leaf.join("Cargo.toml"),
        "[package]\nname='contract_leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='contract_leaf'\npath='src/lib.rs'\n[dependencies]\nrusty={path='../rusty'}\n",
    )
    .expect("write contract leaf manifest");
    std::fs::write(
        contract_leaf.join("src/lib.rs"),
        r#"pub fn dependency_api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#,
    )
    .expect("write valid dependency-owned contract");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");

    for (name, dependency_tables) in [
        (
            "unselected_contract_optional",
            "[dependencies]\ncontract_leaf={path='../../contract-leaf',optional=true}\n[features]\ndefault=[]\n",
        ),
        (
            "unselected_contract_target",
            "[target.'cfg(windows)'.dependencies]\ncontract_leaf={path='../../contract-leaf'}\n",
        ),
    ] {
        let root = temp.path().join("roots").join(name);
        std::fs::create_dir_all(root.join("src")).expect("create marker-free root src");
        std::fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname='marker_free_root'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='marker_free_root'\npath='src/lib.rs'\n{dependency_tables}[workspace]\n"
            ),
        )
        .expect("write marker-free root manifest");
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn root_value() -> i32 { 7 }\n",
        )
        .expect("write marker-free root source");
        let check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", temp.path().join("cargo-target"))
            .output()
            .expect("cargo check unselected-contract root");
        assert_success(&check, &format!("Cargo-valid unselected contract {name}"));

        for existing in [false, true] {
            let lane = if existing { "existing" } else { "absent" };
            let output_dir = temp.path().join("output").join(name).join(lane);
            if existing {
                std::fs::create_dir_all(&output_dir)
                    .expect("create unselected-contract sentinel output");
                std::fs::write(output_dir.join("keep.txt"), format!("keep-{name}\n"))
                    .expect("write unselected-contract sentinel");
            }
            let success = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
                .arg("--crate")
                .arg(root.join("Cargo.toml"))
                .arg("--output-dir")
                .arg(&output_dir)
                .arg("--type-map")
                .arg(&type_map)
                .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
                .output()
                .expect("run unselected-contract root");
            assert_success(
                &success,
                &format!("unselected dependency contract {name}/{lane}"),
            );
            let stderr = String::from_utf8_lossy(&success.stderr);
            assert!(
                output_dir.join("marker_free_root.cppm").is_file(),
                "unselected contract {name}/{lane} omitted root output"
            );
            assert!(
                !stderr.contains("whole local-dependency closure preflight failed before output"),
                "unselected contract {name}/{lane} activated closure rejection: {stderr}"
            );
            if name == "unselected_contract_optional" {
                assert!(
                    stderr.contains("Warning: failed to transpile dependency 'contract_leaf'")
                        && stderr.contains(
                            "cpp_default_argument(stderr) requires structured angle include"
                        ),
                    "legacy optional-dependency warning changed for {lane}: {stderr}"
                );
            } else {
                assert!(
                    stderr.trim().is_empty(),
                    "unselected target dependency emitted an unexpected warning for {lane}: {stderr}"
                );
            }
            let root_bytes = std::fs::read(output_dir.join("marker_free_root.cppm"))
                .expect("read marker-free root output");
            assert_eq!(
                format!("{:x}", Sha256::digest(&root_bytes)),
                "826518f8cf2d006b54f6ca0d77e2205f638bdf4a387f6dc7aca61bf6571ce093",
                "marker-free root bytes diverged from the f910d3e baseline for {name}/{lane}"
            );
            if existing {
                assert_eq!(
                    std::fs::read_to_string(output_dir.join("keep.txt"))
                        .expect("read preserved unselected-contract sentinel"),
                    format!("keep-{name}\n")
                );
            }
        }
    }
}

#[test]
fn crate_mode_dependency_graph_is_package_specific_inside_a_workspace() {
    let rustc_version = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    assert_success(&rustc_version, "rustc host query");
    if !String::from_utf8_lossy(&rustc_version.stdout)
        .lines()
        .any(|line| line == "host: x86_64-unknown-linux-gnu")
    {
        eprintln!("skipping package-specific workspace graph gate on non-Linux test host");
        return;
    }

    let temp = tempfile::tempdir().expect("create package-specific graph temp dir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".cargo")).expect("create Cargo config dir");
    std::fs::write(
        workspace.join(".cargo/config.toml"),
        "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
    )
    .expect("write Cargo target config");
    std::fs::write(
        workspace.join("Cargo.toml"),
        r#"[workspace]
members=['root-selected','root-control','root-dep-contract','rusty','chooser','feature-bridge','poison','windows-poison','contract-leaf']
resolver='2'

[workspace.dependencies]
rusty={path='rusty'}
selected={package='chooser',path='chooser',default-features=false,features=['activate']}
control={package='chooser',path='chooser',default-features=false}
windows_bad={package='windows-poison',path='windows-poison'}
contract_alias={package='contract-leaf',path='contract-leaf'}
"#,
    )
    .expect("write adversarial workspace manifest");

    let write_member = |directory: &str, manifest: &str, source: &str| {
        let member = workspace.join(directory);
        std::fs::create_dir_all(member.join("src")).expect("create workspace member src");
        std::fs::write(member.join("Cargo.toml"), manifest)
            .expect("write workspace member manifest");
        std::fs::write(member.join("src/lib.rs"), source).expect("write workspace member source");
    };
    write_member(
        "rusty",
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
        "pub struct CFile;\n",
    );
    write_member(
        "poison",
        "[package]\nname='poison'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='poison'\npath='src/lib.rs'\n",
        "pub mod cpp { pub mod absent { pub fn value() -> i32 { 19 } } }\nuse cpp::absent;\npub fn poison_value() -> i32 { absent::value() }\n",
    );
    write_member(
        "windows-poison",
        "[package]\nname='windows-poison'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='windows_poison'\npath='src/lib.rs'\n",
        "pub mod cpp { pub mod absent { pub fn value() -> i32 { 23 } } }\nuse cpp::absent;\npub fn value() -> i32 { absent::value() }\n",
    );
    write_member(
        "chooser",
        "[package]\nname='chooser'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='chooser'\npath='src/lib.rs'\n[dependencies]\nfeature_bridge={package='feature-bridge',path='../feature-bridge',optional=true}\n[features]\ndefault=[]\nactivate=['dep:feature_bridge']\n",
        "#[cfg(feature=\"activate\")] pub fn value() -> i32 { feature_bridge::value() }\n#[cfg(not(feature=\"activate\"))] pub fn value() -> i32 { 3 }\n",
    );
    write_member(
        "feature-bridge",
        "[package]\nname='feature-bridge'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='feature_bridge'\npath='src/lib.rs'\n[dependencies]\npoison={path='../poison',optional=true}\n[features]\ndefault=['dep:poison']\n",
        "pub fn value() -> i32 { poison::poison_value() }\n",
    );
    let root_contract = r#"pub fn api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#;
    write_member(
        "root-control",
        "[package]\nname='root_control'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_control'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\ncontrol.workspace=true\n[target.'cfg(any(windows, target_arch = \"aarch64\"))'.dependencies]\nwindows_bad.workspace=true\n",
        root_contract,
    );
    write_member(
        "root-selected",
        "[package]\nname='root_selected'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_selected'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\n[target.'cfg(all(unix, target_arch = \"x86_64\", not(target_os = \"macos\")))'.dependencies]\nselected.workspace=true\n[target.'cfg(any(windows, target_arch = \"aarch64\"))'.dependencies]\nwindows_bad.workspace=true\n",
        root_contract,
    );
    write_member(
        "contract-leaf",
        "[package]\nname='contract-leaf'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='contract_leaf'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\n",
        r#"pub fn dependency_api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#,
    );
    write_member(
        "root-dep-contract",
        "[package]\nname='root_dep_contract'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_dep_contract'\npath='src/lib.rs'\n[target.'x86_64-unknown-linux-gnu'.dependencies]\ncontract_alias.workspace=true\n",
        "pub fn root_value() -> i32 { 1 }\n",
    );

    let cargo_target = temp.path().join("cargo-target");
    for root in ["root-control", "root-selected", "root-dep-contract"] {
        let check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(workspace.join(root).join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("cargo check adversarial workspace root");
        assert_success(&check, &format!("Cargo-valid adversarial root {root}"));
    }

    let control_tree = Command::new("cargo")
        .arg("tree")
        .arg("--manifest-path")
        .arg(workspace.join("root-control/Cargo.toml"))
        .arg("-p")
        .arg("root_control")
        .arg("--target")
        .arg("x86_64-unknown-linux-gnu")
        .arg("--edges")
        .arg("normal")
        .output()
        .expect("query Cargo control graph");
    assert_success(&control_tree, "Cargo control dependency tree");
    assert!(
        !String::from_utf8_lossy(&control_tree.stdout).contains("poison v"),
        "Cargo control graph unexpectedly selected poison"
    );
    let selected_tree = Command::new("cargo")
        .arg("tree")
        .arg("--manifest-path")
        .arg(workspace.join("root-selected/Cargo.toml"))
        .arg("-p")
        .arg("root_selected")
        .arg("--target")
        .arg("x86_64-unknown-linux-gnu")
        .arg("--edges")
        .arg("normal")
        .output()
        .expect("query Cargo selected graph");
    assert_success(&selected_tree, "Cargo selected dependency tree");
    assert!(
        String::from_utf8_lossy(&selected_tree.stdout).contains("poison v"),
        "Cargo selected graph omitted poison"
    );

    let type_map = workspace.join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    for module in ["root_control", "root_selected"] {
        std::fs::write(
            workspace.join(format!("{module}-preamble.toml")),
            format!(
                "version=1\n[[module]]\nname='{module}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write root preamble");
    }

    let control_output = workspace.join("control-existing");
    std::fs::create_dir_all(&control_output).expect("create control sentinel output");
    std::fs::write(control_output.join("keep.txt"), "control-keep\n")
        .expect("write control sentinel");
    let control = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(workspace.join("root-control/Cargo.toml"))
        .arg("--output-dir")
        .arg(&control_output)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(workspace.join("root_control-preamble.toml"))
        .env("CARGO_TARGET_DIR", &cargo_target)
        .output()
        .expect("transpile package-specific control root");
    assert_success(&control, "package-specific control generation");
    assert!(control_output.join("root_control.cppm").is_file());
    assert!(control_output.join("control/chooser.cppm").is_file());
    assert!(
        !control_output.join("control/poison").exists(),
        "control generation followed an optional edge activated only by another workspace root"
    );

    for existing in [false, true] {
        let lane = if existing { "existing" } else { "absent" };
        let selected_output = workspace.join(format!("selected-{lane}"));
        if existing {
            std::fs::create_dir_all(&selected_output).expect("create selected sentinel output");
            std::fs::write(selected_output.join("keep.txt"), "selected-keep\n")
                .expect("write selected sentinel");
        }
        let selected = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(workspace.join("root-selected/Cargo.toml"))
            .arg("--output-dir")
            .arg(&selected_output)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(workspace.join("root_selected-preamble.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("transpile package-specific selected root");
        assert!(!selected.status.success());
        let selected_stderr = String::from_utf8_lossy(&selected.stderr);
        assert!(
            selected_stderr.contains("dependency 'selected' codegen failed before output")
                && selected_stderr
                    .contains("dependency 'feature_bridge' codegen failed before output")
                && selected_stderr.contains("dependency 'poison' codegen failed before output"),
            "selected graph {lane} failed for the wrong reason: {selected_stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read_dir(&selected_output)
                    .expect("read selected sentinel output")
                    .count(),
                1,
                "selected failure mutated preexisting output"
            );
            assert_eq!(
                std::fs::read_to_string(selected_output.join("keep.txt"))
                    .expect("read selected sentinel"),
                "selected-keep\n"
            );
        } else {
            assert!(
                !selected_output.exists(),
                "selected failure created a fresh output directory"
            );
        }
    }

    for existing in [false, true] {
        let lane = if existing { "existing" } else { "absent" };
        let contract_output = workspace.join(format!("contract-{lane}"));
        if existing {
            std::fs::create_dir_all(&contract_output).expect("create contract sentinel output");
            std::fs::write(contract_output.join("keep.txt"), "contract-keep\n")
                .expect("write contract sentinel");
        }
        let contract = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(workspace.join("root-dep-contract/Cargo.toml"))
            .arg("--output-dir")
            .arg(&contract_output)
            .arg("--type-map")
            .arg(&type_map)
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("transpile dependency-owned contract root");
        assert!(!contract.status.success());
        assert!(
            String::from_utf8_lossy(&contract.stderr)
                .contains("contains source-owned C++ contracts"),
            "selected dependency-owned contract {lane} was not rejected: {}",
            String::from_utf8_lossy(&contract.stderr)
        );
        if existing {
            assert_eq!(
                std::fs::read_dir(&contract_output)
                    .expect("read contract sentinel output")
                    .count(),
                1,
                "dependency-contract failure mutated preexisting output"
            );
        } else {
            assert!(
                !contract_output.exists(),
                "dependency-contract failure created a fresh output directory"
            );
        }
    }
}

#[test]
fn crate_mode_target_normal_features_stay_separate_from_host_and_non_normal_units() {
    let rustc_version = Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("query rustc host target");
    assert_success(&rustc_version, "rustc host query");
    if !String::from_utf8_lossy(&rustc_version.stdout)
        .lines()
        .any(|line| line == "host: x86_64-unknown-linux-gnu")
    {
        eprintln!("skipping resolver-context graph gate on non-Linux test host");
        return;
    }

    let temp = tempfile::tempdir().expect("create resolver-context temp dir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".cargo")).expect("create Cargo config dir");
    std::fs::write(
        workspace.join(".cargo/config.toml"),
        "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
    )
    .expect("write Cargo target config");
    std::fs::write(
        workspace.join("Cargo.toml"),
        r#"[workspace]
members=['root-build','root-dev','root-proc','root-target','rusty','chooser','poison','pm']
resolver='2'

[workspace.dependencies]
rusty={path='rusty'}
chooser_control={package='chooser',path='chooser',default-features=false}
chooser_active={package='chooser',path='chooser',default-features=false,features=['activate']}
pm={path='pm'}
"#,
    )
    .expect("write resolver-context workspace manifest");

    let write_member = |directory: &str, manifest: &str, source: &str| {
        let member = workspace.join(directory);
        std::fs::create_dir_all(member.join("src")).expect("create workspace member src");
        std::fs::write(member.join("Cargo.toml"), manifest)
            .expect("write workspace member manifest");
        std::fs::write(member.join("src/lib.rs"), source).expect("write workspace member source");
    };
    write_member(
        "rusty",
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
        "pub struct CFile;\n",
    );
    write_member(
        "poison",
        "[package]\nname='poison'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='poison'\npath='src/lib.rs'\n",
        "pub mod cpp { pub mod absent { pub fn value() -> i32 { 29 } } }\nuse cpp::absent;\npub fn poison_value() -> i32 { absent::value() }\n",
    );
    write_member(
        "chooser",
        "[package]\nname='chooser'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='chooser'\npath='src/lib.rs'\n[dependencies]\npoison={path='../poison',optional=true}\n[features]\ndefault=[]\nactivate=['dep:poison']\n",
        "#[cfg(feature=\"activate\")] pub fn value() -> i32 { poison::poison_value() }\n#[cfg(not(feature=\"activate\"))] pub fn value() -> i32 { 5 }\n",
    );
    write_member(
        "pm",
        "[package]\nname='pm'\nversion='0.1.0'\nedition='2024'\n[lib]\nproc-macro=true\n[dependencies]\nchooser_active.workspace=true\n",
        r#"use proc_macro::TokenStream;
#[proc_macro_attribute]
pub fn identity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = chooser_active::value();
    item
}
"#,
    );
    let root_contract = r#"pub fn api(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = stream; }
"#;
    write_member(
        "root-build",
        "[package]\nname='root_build'\nversion='0.1.0'\nedition='2024'\nbuild='build.rs'\n[lib]\nname='root_build'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\nchooser={path='../chooser',default-features=false}\n[build-dependencies]\nchooser={path='../chooser',default-features=false,features=['activate']}\n",
        root_contract,
    );
    std::fs::write(
        workspace.join("root-build/build.rs"),
        "fn main() { let _ = chooser::value(); }\n",
    )
    .expect("write build-context feature activator");
    write_member(
        "root-dev",
        "[package]\nname='root_dev'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_dev'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\nchooser={path='../chooser',default-features=false}\n[dev-dependencies]\nchooser={path='../chooser',default-features=false,features=['activate']}\n",
        root_contract,
    );
    write_member(
        "root-proc",
        "[package]\nname='root_proc'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_proc'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\nchooser_control.workspace=true\npm.workspace=true\n",
        root_contract,
    );
    write_member(
        "root-target",
        "[package]\nname='root_target'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='root_target'\npath='src/lib.rs'\n[dependencies]\nrusty.workspace=true\nchooser_active.workspace=true\n",
        root_contract,
    );

    let cargo_target = temp.path().join("cargo-target");
    for root in ["root-build", "root-dev", "root-proc", "root-target"] {
        let check = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(workspace.join(root).join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("cargo check resolver-context root");
        assert_success(&check, &format!("Cargo-valid resolver-context root {root}"));
    }

    for root in ["root-build", "root-dev"] {
        let package = root.replace('-', "_");
        for (edges, poison_expected) in [("normal", false), ("all", true)] {
            let tree = Command::new("cargo")
                .arg("tree")
                .arg("--manifest-path")
                .arg(workspace.join(root).join("Cargo.toml"))
                .arg("-p")
                .arg(&package)
                .arg("--target")
                .arg("x86_64-unknown-linux-gnu")
                .arg("--edges")
                .arg(edges)
                .output()
                .expect("query Cargo non-normal feature context");
            assert_success(&tree, &format!("Cargo {root} {edges} dependency tree"));
            assert_eq!(
                String::from_utf8_lossy(&tree.stdout).contains("poison v"),
                poison_expected,
                "Cargo {root} {edges} tree had the wrong poison selection:\n{}",
                String::from_utf8_lossy(&tree.stdout)
            );
        }
    }
    for (edges, pm_expected, poison_expected) in [
        ("normal", true, true),
        ("normal,no-proc-macro", false, false),
    ] {
        let tree = Command::new("cargo")
            .arg("tree")
            .arg("--manifest-path")
            .arg(workspace.join("root-proc/Cargo.toml"))
            .arg("-p")
            .arg("root_proc")
            .arg("--target")
            .arg("x86_64-unknown-linux-gnu")
            .arg("--edges")
            .arg(edges)
            .output()
            .expect("query Cargo procedural-macro feature context");
        assert_success(&tree, &format!("Cargo procedural-macro {edges} tree"));
        let tree = String::from_utf8_lossy(&tree.stdout);
        assert_eq!(
            tree.contains("pm v"),
            pm_expected,
            "wrong pm selection: {tree}"
        );
        assert_eq!(
            tree.contains("poison v"),
            poison_expected,
            "wrong poison selection: {tree}"
        );
        assert!(
            tree.contains("chooser v"),
            "target chooser was lost: {tree}"
        );
    }

    let type_map = workspace.join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    for module in ["root_build", "root_dev", "root_proc", "root_target"] {
        std::fs::write(
            workspace.join(format!("{module}-preamble.toml")),
            format!(
                "version=1\n[[module]]\nname='{module}'\nincludes=[{{path='cstdio',form='angle'}}]\n"
            ),
        )
        .expect("write resolver-context preamble");
    }

    for (root, module, chooser_alias) in [
        ("root-build", "root_build", "chooser"),
        ("root-dev", "root_dev", "chooser"),
        ("root-proc", "root_proc", "chooser_control"),
    ] {
        let output_dir = workspace.join(format!("{root}-output"));
        std::fs::create_dir_all(&output_dir).expect("create resolver-context sentinel output");
        std::fs::write(output_dir.join("keep.txt"), format!("keep-{root}\n"))
            .expect("write resolver-context sentinel");
        let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(workspace.join(root).join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(workspace.join(format!("{module}-preamble.toml")))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("transpile resolver-context control root");
        assert_success(&generated, &format!("resolver-context control {root}"));
        assert!(output_dir.join(format!("{module}.cppm")).is_file());
        assert!(
            output_dir
                .join(chooser_alias)
                .join("chooser.cppm")
                .is_file()
        );
        assert!(!output_dir.join("pm").exists());
        assert!(!output_dir.join("poison").exists());
        assert_eq!(
            std::fs::read_to_string(output_dir.join("keep.txt"))
                .expect("read resolver-context sentinel"),
            format!("keep-{root}\n")
        );
    }

    for existing in [false, true] {
        let lane = if existing { "existing" } else { "absent" };
        let output_dir = workspace.join(format!("root-target-{lane}"));
        if existing {
            std::fs::create_dir_all(&output_dir).expect("create target-active sentinel output");
            std::fs::write(output_dir.join("keep.txt"), "keep-target\n")
                .expect("write target-active sentinel");
        }
        let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg("--crate")
            .arg(workspace.join("root-target/Cargo.toml"))
            .arg("--output-dir")
            .arg(&output_dir)
            .arg("--type-map")
            .arg(&type_map)
            .arg("--module-preamble")
            .arg(workspace.join("root_target-preamble.toml"))
            .env("CARGO_TARGET_DIR", &cargo_target)
            .output()
            .expect("transpile target-active root");
        assert!(!generated.status.success());
        let stderr = String::from_utf8_lossy(&generated.stderr);
        assert!(
            stderr.contains("dependency 'chooser_active' codegen failed before output")
                && stderr.contains("dependency 'poison' codegen failed before output"),
            "target-active {lane} failed for the wrong reason: {stderr}"
        );
        if existing {
            assert_eq!(
                std::fs::read_dir(&output_dir)
                    .expect("read target-active sentinel output")
                    .count(),
                1,
                "target-active failure mutated preexisting output"
            );
            assert_eq!(
                std::fs::read_to_string(output_dir.join("keep.txt"))
                    .expect("read target-active sentinel"),
                "keep-target\n"
            );
        } else {
            assert!(!output_dir.exists(), "target-active failure created output");
        }
    }
}

#[test]
fn direct_modes_reject_default_contracts_without_partial_output() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = temp.path().join("fixture.rs");
    std::fs::write(
        &source,
        r#"pub fn print(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) {}"#,
    )
    .expect("write source");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");

    let missing_preamble = temp.path().join("missing-preamble.cppm");
    let missing_preamble_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&missing_preamble)
        .arg("-m")
        .arg("fixture")
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run missing-preamble negative");
    assert!(!missing_preamble_output.status.success());
    assert!(
        !missing_preamble.exists(),
        "missing-preamble rejection wrote output"
    );

    let moduleless = temp.path().join("moduleless.cppm");
    let moduleless_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&moduleless)
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run moduleless negative");
    assert!(!moduleless_output.status.success());
    assert!(!moduleless.exists(), "moduleless rejection wrote output");

    let expanded = temp.path().join("expanded.cppm");
    let expanded_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&expanded)
        .arg("-m")
        .arg("fixture")
        .arg("--expand")
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run expand negative");
    assert!(!expanded_output.status.success());
    assert!(!expanded.exists(), "expand rejection wrote output");

    std::fs::write(
        &source,
        r#"pub fn print(
    #[cpp_default_argument(stderr)]
    stream: *mut ::rusty::CFile,
) {}"#,
    )
    .expect("write malformed source contract");
    let malformed = temp.path().join("malformed.cppm");
    let malformed_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&malformed)
        .arg("-m")
        .arg("fixture")
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run malformed marker negative");
    assert!(!malformed_output.status.success());
    assert!(
        !malformed.exists(),
        "malformed marker rejection wrote output"
    );

    std::fs::write(
        &source,
        r#"use dependency::WIDTH;
pub type Payload = [u8; WIDTH];
pub fn unresolved(
    value: Payload,
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) { let _ = (value, stream); }
"#,
    )
    .expect("write unresolved const source");
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture'\nincludes=[{path='cstdio',form='angle'}]\n",
    )
    .expect("write exact preamble");
    let unresolved = temp.path().join("unresolved.cppm");
    let unresolved_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&unresolved)
        .arg("-m")
        .arg("fixture")
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run unresolved const negative");
    assert!(!unresolved_output.status.success());
    assert!(
        String::from_utf8_lossy(&unresolved_output.stderr)
            .contains("cannot prove external const closure"),
        "unexpected unresolved-const diagnostic: {}",
        String::from_utf8_lossy(&unresolved_output.stderr)
    );
    assert!(
        !unresolved.exists(),
        "unresolved const rejection wrote output"
    );
}

#[test]
fn crate_mode_finds_out_of_line_defaults_and_rejects_dependency_contracts() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let runtime = temp.path().join("rusty");
    std::fs::create_dir_all(runtime.join("src")).expect("create runtime facade");
    std::fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname='rusty'\nversion='0.1.0'\nedition='2024'\n[lib]\nname='rusty'\npath='src/lib.rs'\n",
    )
    .expect("write runtime manifest");
    std::fs::write(runtime.join("src/lib.rs"), "pub struct CFile;\n")
        .expect("write runtime facade");

    let crate_dir = temp.path().join("fixture");
    std::fs::create_dir_all(crate_dir.join("src")).expect("create fixture src");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nrusty={path='../rusty'}\n",
    )
    .expect("write manifest");
    std::fs::write(crate_dir.join("src/lib.rs"), "pub mod child;\n").expect("write root source");
    std::fs::write(
        crate_dir.join("src/child.rs"),
        r#"pub fn print(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) {}"#,
    )
    .expect("write child source");
    let type_map = temp.path().join("type-map.toml");
    std::fs::write(&type_map, "[rusty]\nCFile='FILE'\n").expect("write type map");
    let missing_preamble = temp.path().join("missing-preamble");
    let missing_preamble_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&missing_preamble)
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run missing-preamble crate negative");
    assert!(!missing_preamble_output.status.success());
    assert!(
        !missing_preamble.exists(),
        "crate preflight wrote output before rejecting a missing preamble"
    );
    let preamble = temp.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version=1\n[[module]]\nname='fixture.child'\nincludes=[{path='stdio.h',form='angle'}]\n",
    )
    .expect("write module preamble");
    let generated = temp.path().join("generated");
    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&generated)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run out-of-line positive");
    assert_success(&output, "out-of-line default transpilation");
    let child = std::fs::read_to_string(generated.join("fixture.child.cppm"))
        .expect("read generated child");
    assert_eq!(child.matches("FILE* stream = stderr").count(), 1, "{child}");

    std::fs::write(
        crate_dir.join("src/lib.rs"),
        "#[cfg(any())]\npub mod child;\n",
    )
    .expect("write configured owner declaration");
    let configured = temp.path().join("configured-owner");
    let configured_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&configured)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run configured-owner negative");
    assert!(!configured_output.status.success());
    assert!(
        !configured.exists(),
        "configured owner declaration rejection wrote output"
    );
    std::fs::write(crate_dir.join("src/lib.rs"), "mod child;\n")
        .expect("write private owner declaration");
    let private = temp.path().join("private-owner");
    let private_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&private)
        .arg("--type-map")
        .arg(&type_map)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("run private-owner negative");
    assert!(!private_output.status.success());
    assert!(
        !private.exists(),
        "private owner declaration rejection wrote output"
    );

    let dependency = temp.path().join("dependency");
    std::fs::create_dir_all(dependency.join("src")).expect("create dependency src");
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='dependency'\nversion='0.1.0'\nedition='2024'\n",
    )
    .expect("write dependency manifest");
    std::fs::write(
        dependency.join("src/lib.rs"),
        r#"pub fn print(
    #[cfg_attr(any(), cpp_default_argument(stderr))]
    stream: *mut ::rusty::CFile,
) {}"#,
    )
    .expect("write dependency contract");
    let root = temp.path().join("root");
    std::fs::create_dir_all(root.join("src")).expect("create root src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='root'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ndependency={path='../dependency'}\n",
    )
    .expect("write root manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn root() {}\n").expect("write root source");
    let rejected = temp.path().join("rejected");
    let dependency_output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&rejected)
        .arg("--type-map")
        .arg(&type_map)
        .output()
        .expect("run dependency negative");
    assert!(!dependency_output.status.success());
    assert!(!rejected.exists(), "dependency rejection wrote root output");
}
