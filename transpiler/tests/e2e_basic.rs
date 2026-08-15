use std::process::Command;

fn transpiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
}

fn assert_no_crate_output_transaction_artifacts(parent: &std::path::Path) {
    assert!(
        std::fs::read_dir(parent).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".rusty-cpp-")
        }),
        "crate output transaction leaked a staging, backup, or lock beside {}",
        parent.display()
    );
}

#[test]
fn test_cli_build_info_reports_embedded_revision_before_file_validation() {
    let output = transpiler_bin()
        .args(["--build-info", "definitely-does-not-exist.rs"])
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "{{\"git_hash\":\"{}\",\"git_dirty\":{}}}\n",
            env!("RUSTY_CPP_GIT_HASH"),
            env!("RUSTY_CPP_GIT_DIRTY")
        )
    );
}

#[test]
fn test_cli_missing_input() {
    let output = transpiler_bin().output().expect("failed to run");
    assert!(!output.status.success());
}

#[test]
fn test_cli_nonexistent_file() {
    let output = transpiler_bin()
        .arg("nonexistent.rs")
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn test_cli_transpile_basic() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    let output_path = dir.path().join("test.cppm");

    std::fs::write(
        &input,
        r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

struct Point {
    x: f64,
    y: f64,
}

const MAX: i32 = 100;
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("int32_t add(int32_t a, int32_t b)"));
    // The body may emit bare `a + b` or wrap each operand in
    // `rusty::detail::deref_if_pointer_like` when types are template-
    // bound or otherwise unresolved at the call site.
    assert!(
        cpp.contains("return a + b;")
            || cpp.contains(
                "return rusty::detail::deref_if_pointer_like(a) + rusty::detail::deref_if_pointer_like(b);"
            ),
        "unexpected add body: {cpp}"
    );
    assert!(cpp.contains("struct Point {"));
    assert!(cpp.contains("double x;"));
    // Constants may emit with or without an explicit static_cast for the
    // initializer (`MAX = 100;` vs `MAX = static_cast<int32_t>(100);`).
    assert!(
        cpp.contains("constexpr int32_t MAX = 100;")
            || cpp.contains("constexpr int32_t MAX = static_cast<int32_t>(100);"),
        "unexpected MAX const: {cpp}"
    );
}

#[test]
fn test_cpp_import_namespace_rejects_direct_named_module_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("consumer.rs");
    let output_path = dir.path().join("consumer.cppm");
    std::fs::write(
        &input,
        r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::randgen_rand_raw;
pub fn draw() -> u64 { randgen_rand_raw() }
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(&input)
        .args(["-o", output_path.to_str().unwrap()])
        .args(["-m", "rrr.consumer", "--cxx-namespace", "rrr"])
        .output()
        .expect("failed to run direct named-module rejection probe");

    assert!(!output.status.success());
    assert!(!output_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires prepared crate mode or prepared inline-rust mode"),
        "{stderr}"
    );
}

#[test]
fn test_cli_default_output_name() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("hello.rs");

    std::fs::write(&input, "fn hello() {}").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());

    // Should create hello.cppm in same directory
    let expected_output = dir.path().join("hello.cppm");
    assert!(
        expected_output.exists(),
        "Expected hello.cppm to be created"
    );
}

#[test]
fn test_transpile_rusty_types() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("types.rs");
    let output_path = dir.path().join("types.cppm");

    std::fs::write(
        &input,
        r#"
fn process(v: Vec<i32>, m: HashMap<String, f64>) -> Option<bool> {
    None
}
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("rusty::Vec<int32_t>"));
    assert!(cpp.contains("rusty::HashMap<rusty::String, double>"));
    assert!(cpp.contains("rusty::Option<bool>"));
}

#[test]
fn test_transpile_enum_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("enum.rs");
    let output_path = dir.path().join("enum.cppm");

    std::fs::write(
        &input,
        r#"
enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
    None,
}
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("struct Shape_Circle"));
    assert!(cpp.contains("struct Shape_Rect"));
    assert!(cpp.contains("struct Shape_None"));
    assert!(cpp.contains("using Shape = std::variant<"));
}

#[test]
fn test_expand_flag_without_cargo_toml() {
    // --expand on a file with no Cargo.toml should fail gracefully
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    std::fs::write(&input, "fn main() {}").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--expand")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cargo.toml") || stderr.contains("cargo expand"));
}

#[test]
fn test_module_name_flag() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lib.rs");
    let output_path = dir.path().join("lib.cppm");

    std::fs::write(&input, "pub fn hello() {}").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("-m")
        .arg("my_crate")
        .output()
        .expect("failed to run");

    assert!(output.status.success());

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("export module my_crate;"));
    assert!(cpp.contains("export void hello()"));
}

#[test]
fn test_module_preamble_single_file_preserves_include_order() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lib.rs");
    let output_path = dir.path().join("lib.cppm");
    let preamble = dir.path().join("module-preamble.toml");
    std::fs::write(&input, "pub fn hello() {}\n").unwrap();
    std::fs::write(
        &preamble,
        r#"
version = 1

[[module]]
name = "demo"
includes = [
    { path = "demo/local.hpp", form = "quote" },
    { path = "sys/types.h", form = "angle" },
]
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(&input)
        .arg("--module-name")
        .arg("demo")
        .arg("--module-preamble")
        .arg(&preamble)
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    let module_fragment = cpp.find("\nmodule;\n").unwrap();
    let local = cpp.find("#include \"demo/local.hpp\"").unwrap();
    let system = cpp.find("#include <sys/types.h>").unwrap();
    let module_decl = cpp.find("export module demo;").unwrap();
    assert!(module_fragment < local && local < system && system < module_decl);
}

#[test]
fn test_module_preamble_is_rejected_without_module_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("lib.rs");
    let preamble = dir.path().join("module-preamble.toml");
    std::fs::write(&input, "pub fn hello() {}\n").unwrap();
    std::fs::write(
        &preamble,
        "version = 1\n[[module]]\nname = \"demo\"\nincludes = [{ path = \"demo/local.hpp\", form = \"quote\" }]\n",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(&input)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("requires module output"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_module_preamble_is_not_silently_ignored_by_non_transpile_modes() {
    let dir = tempfile::tempdir().unwrap();
    let preamble = dir.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version = 1\n[[module]]\nname = \"demo\"\nincludes = [{ path = \"demo/local.hpp\", form = \"quote\" }]\n",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg("--module-preamble")
        .arg(&preamble)
        .arg("--build-info")
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("requires module output"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_module_preamble_crate_mode_selects_each_row_and_rejects_stale_rows() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .unwrap();
    std::fs::write(src.join("lib.rs"), "pub mod net;\n").unwrap();
    std::fs::write(src.join("net.rs"), "pub fn port() -> i32 { 7 }\n").unwrap();
    let preamble = dir.path().join("module-preamble.toml");
    std::fs::write(
        &preamble,
        "version = 1\n[[module]]\nname = \"demo.net\"\nincludes = [{ path = \"demo/net.hpp\", form = \"quote\" }]\n",
    )
    .unwrap();
    let output_dir = dir.path().join("cpp-out");

    let output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root_cpp = std::fs::read_to_string(output_dir.join("demo.cppm")).unwrap();
    let net_cpp = std::fs::read_to_string(output_dir.join("demo.net.cppm")).unwrap();
    assert!(!root_cpp.contains("demo/net.hpp"));
    assert!(net_cpp.contains("#include \"demo/net.hpp\""));

    std::fs::write(
        &preamble,
        "version = 1\n[[module]]\nname = \"demo.removed\"\nincludes = [{ path = \"demo/old.hpp\", form = \"quote\" }]\n",
    )
    .unwrap();
    let stale_output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("Cargo.toml"))
        .arg("--output-dir")
        .arg(dir.path().join("stale-out"))
        .arg("--module-preamble")
        .arg(&preamble)
        .output()
        .expect("failed to run");
    assert!(!stale_output.status.success());
    let stderr = String::from_utf8_lossy(&stale_output.stderr);
    assert!(stderr.contains("stale/uncollected"), "stderr: {stderr}");
    assert!(stderr.contains("demo.removed"), "stderr: {stderr}");
}

#[test]
fn test_cmake_generation() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    // Create a minimal Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"
[package]
name = "hello"
version = "1.0.0"

[[bin]]
name = "hello"
path = "src/main.rs"
"#,
    )
    .unwrap();

    // Create source files
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(src_dir.join("utils.rs"), "pub fn helper() {}").unwrap();

    // Run with --cmake flag (pass a dummy input file since it's required)
    let output = transpiler_bin()
        .arg(src_dir.join("main.rs").to_str().unwrap())
        .arg("--cmake")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify CMakeLists.txt was created
    let cmake_path = dir.path().join("CMakeLists.txt");
    assert!(cmake_path.exists());

    let cmake = std::fs::read_to_string(&cmake_path).unwrap();
    assert!(cmake.contains("project(hello VERSION 1.0.0"));
    assert!(cmake.contains("add_executable(hello"));
    assert!(cmake.contains("hello.cppm"));
}

#[test]
fn test_verify_flag_without_checker() {
    // --verify should attempt to run rusty-cpp-checker and fail gracefully if not found
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    let output_path = dir.path().join("test.cppm");

    std::fs::write(&input, "fn f() { let x = 42; }").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("--verify")
        .output()
        .expect("failed to run");

    // Transpilation should succeed (file written) even if verify fails
    assert!(
        output_path.exists(),
        "output file should be written before verification"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Transpiled"));
}

#[test]
fn test_verify_flag_with_checker() {
    // If rusty-cpp-checker is available (built from same workspace), verify should work
    let checker = std::path::Path::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .parent()
        .unwrap()
        .join("rusty-cpp-checker");

    if !checker.exists() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    let output_path = dir.path().join("test.cppm");

    std::fs::write(&input, "fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("--verify")
        .output()
        .expect("failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Transpiled"));
}

#[test]
fn test_crate_mode_basic() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"my_math\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"my_math\"\n",
    )
    .unwrap();

    std::fs::write(
        src_dir.join("lib.rs"),
        "pub mod vector;\npub fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("vector.rs"),
        "pub struct Vec2 { pub x: f64, pub y: f64 }",
    )
    .unwrap();

    let out_dir = dir.path().join("cpp_out");

    let output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .arg("--output-dir")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out_dir.join("my_math.cppm").exists());
    assert!(out_dir.join("my_math.vector.cppm").exists());
    assert!(out_dir.join("CMakeLists.txt").exists());

    let lib_cpp = std::fs::read_to_string(out_dir.join("my_math.cppm")).unwrap();
    assert!(lib_cpp.contains("export module my_math;"));
    assert!(lib_cpp.contains("export int32_t add("));

    let vec_cpp = std::fs::read_to_string(out_dir.join("my_math.vector.cppm")).unwrap();
    assert!(vec_cpp.contains("export module my_math.vector;"));
    assert!(vec_cpp.contains("export struct Vec2"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Transpiling crate 'my_math'"));
    assert!(stdout.contains("2 files transpiled"));
}

#[test]
fn crate_expand_uses_exact_conventional_lib_target_and_cargo_context() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let manifest = dir.path().join("Cargo.toml");
    std::fs::write(
        &manifest,
        "[package]\nname='crate_expand_probe'\nversion='0.0.0'\nedition='2024'\n\
         [features]\ndefault=[]\nexpanded-api=[]\n\
         [workspace]\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"
#[cfg(feature = "expanded-api")]
macro_rules! emit_expanded_only {
    () => { pub fn expanded_only() -> i32 { 7 } };
}

#[cfg(feature = "expanded-api")]
emit_expanded_only!();
"#,
    )
    .unwrap();

    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--features")
        .arg("expanded-api")
        .env("CARGO_TARGET_DIR", dir.path().join("cargo-target"))
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "macro-only crate-expand fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let output_dir = dir.path().join("cpp-out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(&manifest)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--expand")
        .arg("--package")
        .arg("crate_expand_probe")
        .arg("--features")
        .arg("expanded-api")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "exact crate expansion failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let generated =
        std::fs::read_to_string(output_dir.join("crate_expand_probe.cppm")).unwrap();
    assert!(
        generated.contains("expanded_only("),
        "macro-generated API was omitted:\n{generated}"
    );
    assert!(
        !generated.contains("TODO") && !generated.contains("emit_expanded_only!("),
        "explicit expansion silently used raw source:\n{generated}"
    );
    assert!(output_dir.join("CMakeLists.txt").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("falling back"), "stderr: {stderr}");
}

#[test]
fn crate_expand_preserves_target_normal_dependency_features_and_clang_imports() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let dependency = fixture.path().join("selected_dep");
    let poison = fixture.path().join("build_poison");
    for package in [&root, &dependency, &poison] {
        std::fs::create_dir_all(package.join("src")).unwrap();
    }

    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='feature_root'\nversion='0.0.0'\nedition='2024'\nbuild='build.rs'\n\
         [dependencies]\nselected_dep={path='../selected_dep',default-features=false,features=['expanded-api']}\n\
         [build-dependencies]\nselected_dep={path='../selected_dep',default-features=false,features=['build-only']}\n\
         [workspace]\nresolver='2'\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn root_value() -> i32 { selected_dep::expanded_only() }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("build.rs"),
        "fn main() { assert_eq!(selected_dep::build_only(), 11); }\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='selected_dep'\nversion='0.0.0'\nedition='2024'\n\
         [features]\ndefault=[]\nexpanded-api=[]\nbuild-only=['dep:build_poison']\n\
         [dependencies]\nbuild_poison={path='../build_poison',optional=true}\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("src/lib.rs"),
        r#"
#[cfg(feature = "expanded-api")]
macro_rules! emit_expanded_only {
    () => { pub fn expanded_only() -> i32 { 7 } };
}
#[cfg(feature = "expanded-api")]
emit_expanded_only!();

#[cfg(feature = "build-only")]
pub fn build_only() -> i32 { build_poison::poison() }

#[cfg(not(any(feature = "expanded-api", feature = "build-only")))]
pub fn baseline_only() -> i32 { 3 }
"#,
    )
    .unwrap();
    std::fs::write(
        poison.join("Cargo.toml"),
        "[package]\nname='build_poison'\nversion='0.0.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(poison.join("src/lib.rs"), "pub fn poison() -> i32 { 11 }\n").unwrap();

    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "feature-context fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let output_dir = fixture.path().join("cpp-out");
    assert!(!output_dir.exists());
    let generated = transpiler_bin()
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--expand")
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "feature-exact expansion failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    assert_no_crate_output_transaction_artifacts(fixture.path());

    let dependency_cpp =
        std::fs::read_to_string(output_dir.join("selected_dep/selected_dep.cppm")).unwrap();
    let root_cpp = std::fs::read_to_string(output_dir.join("feature_root.cppm")).unwrap();
    assert!(dependency_cpp.contains("namespace selected_dep"), "{dependency_cpp}");
    assert!(dependency_cpp.contains("expanded_only("), "{dependency_cpp}");
    assert!(!dependency_cpp.contains("build_only("), "{dependency_cpp}");
    assert!(!dependency_cpp.contains("baseline_only("), "{dependency_cpp}");
    assert!(root_cpp.contains("export import selected_dep;"), "{root_cpp}");
    assert!(
        root_cpp.contains("selected_dep::expanded_only()"),
        "{root_cpp}"
    );
    assert!(!output_dir.join("build_poison").exists());

    let existing_output = fixture.path().join("existing-out");
    std::fs::create_dir_all(existing_output.join("selected_dep")).unwrap();
    std::fs::write(existing_output.join("sentinel.txt"), b"old tree\n").unwrap();
    std::fs::write(
        existing_output.join("selected_dep/stale.cppm"),
        b"stale dependency\n",
    )
    .unwrap();
    let replacement = transpiler_bin()
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&existing_output)
        .arg("--expand")
        .output()
        .unwrap();
    assert!(
        replacement.status.success(),
        "existing feature-exact expansion failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&replacement.stdout),
        String::from_utf8_lossy(&replacement.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(existing_output.join("selected_dep/selected_dep.cppm")).unwrap(),
        dependency_cpp
    );
    assert_eq!(
        std::fs::read_to_string(existing_output.join("feature_root.cppm")).unwrap(),
        root_cpp
    );
    assert!(!existing_output.join("sentinel.txt").exists());
    assert!(!existing_output.join("selected_dep/stale.cppm").exists());
    assert_no_crate_output_transaction_artifacts(fixture.path());

    let clang = std::env::var_os("RUSTY_CPP_TEST_CLANG").unwrap_or_else(|| "clang++".into());
    let include_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("include");
    let clang_dir = fixture.path().join("clang");
    std::fs::create_dir(&clang_dir).unwrap();
    let dependency_pcm = clang_dir.join("selected_dep.pcm");
    let dependency_object = clang_dir.join("selected_dep.o");
    let root_pcm = clang_dir.join("feature_root.pcm");
    let root_object = clang_dir.join("feature_root.o");
    let importer_source = clang_dir.join("importer.cpp");
    let importer_object = clang_dir.join("importer.o");
    let importer = clang_dir.join("importer");
    std::fs::write(
        &importer_source,
        "import feature_root;\nint main() { return root_value() == 7 ? 0 : 1; }\n",
    )
    .unwrap();
    let common = ["-std=c++23", "-march=native", "-w", "-I"];
    let run_clang = |label: &str, arguments: &[std::ffi::OsString]| {
        let mut command = Command::new(&clang);
        command.args(common).arg(&include_dir).args(arguments);
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("failed to execute Clang for {label}: {error}"));
        assert!(
            output.status.success(),
            "Clang {label} failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run_clang(
        "dependency precompile",
        &[
            "-x".into(),
            "c++-module".into(),
            "--precompile".into(),
            output_dir
                .join("selected_dep/selected_dep.cppm")
                .into_os_string(),
            "-o".into(),
            dependency_pcm.clone().into_os_string(),
        ],
    );
    run_clang(
        "dependency object",
        &[
            "-c".into(),
            dependency_pcm.clone().into_os_string(),
            "-o".into(),
            dependency_object.clone().into_os_string(),
        ],
    );
    let dependency_module_arg = format!(
        "-fmodule-file=selected_dep={}",
        dependency_pcm.display()
    );
    run_clang(
        "root precompile",
        &[
            dependency_module_arg.clone().into(),
            "-x".into(),
            "c++-module".into(),
            "--precompile".into(),
            output_dir.join("feature_root.cppm").into_os_string(),
            "-o".into(),
            root_pcm.clone().into_os_string(),
        ],
    );
    run_clang(
        "root object",
        &[
            dependency_module_arg.clone().into(),
            "-c".into(),
            root_pcm.clone().into_os_string(),
            "-o".into(),
            root_object.clone().into_os_string(),
        ],
    );
    let root_module_arg = format!("-fmodule-file=feature_root={}", root_pcm.display());
    run_clang(
        "importer object",
        &[
            dependency_module_arg.into(),
            root_module_arg.into(),
            "-c".into(),
            importer_source.into_os_string(),
            "-o".into(),
            importer_object.clone().into_os_string(),
        ],
    );
    run_clang(
        "link",
        &[
            dependency_object.into_os_string(),
            root_object.into_os_string(),
            importer_object.into_os_string(),
            "-o".into(),
            importer.clone().into_os_string(),
        ],
    );
    assert!(
        Command::new(importer).status().unwrap().success(),
        "generated module runtime failed"
    );
}

#[test]
#[cfg(unix)]
fn crate_expand_rejects_ambiguity_and_failures_without_touching_output() {
    let failure = tempfile::tempdir().unwrap();
    std::fs::create_dir(failure.path().join("src")).unwrap();
    let failure_manifest = failure.path().join("Cargo.toml");
    std::fs::write(
        &failure_manifest,
        "[package]\nname='crate_expand_failure'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        failure.path().join("src/lib.rs"),
        "pub fn ordinary() -> i32 { 7 }\n",
    )
    .unwrap();
    let cargo_shim_dir = failure.path().join("cargo-shim");
    std::fs::create_dir(&cargo_shim_dir).unwrap();
    let real_cargo = Command::new("which").arg("cargo").output().unwrap();
    assert!(real_cargo.status.success());
    let real_cargo = String::from_utf8(real_cargo.stdout)
        .unwrap()
        .trim()
        .to_string();
    let cargo_shim = cargo_shim_dir.join("cargo");
    std::fs::write(
        &cargo_shim,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"expand\" ]; then\n  echo forced crate expansion failure >&2\n  exit 86\nfi\nexec '{real_cargo}' \"$@\"\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&cargo_shim).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo_shim, permissions).unwrap();
    }
    let shim_path = format!(
        "{}:{}",
        cargo_shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let absent_output = failure.path().join("absent-out");
    let absent_failure = transpiler_bin()
        .arg("--crate")
        .arg(&failure_manifest)
        .arg("--output-dir")
        .arg(&absent_output)
        .arg("--expand")
        .env("PATH", &shim_path)
        .output()
        .unwrap();
    assert!(!absent_failure.status.success());
    assert!(
        String::from_utf8_lossy(&absent_failure.stderr).contains("cargo expand failed"),
        "unexpected expansion diagnostic:\n{}",
        String::from_utf8_lossy(&absent_failure.stderr)
    );
    assert!(
        !absent_output.exists(),
        "failed expansion created {}",
        absent_output.display()
    );

    let existing_output = failure.path().join("existing-out");
    std::fs::create_dir(&existing_output).unwrap();
    let sentinel_path = existing_output.join("sentinel.txt");
    let generated_path = existing_output.join("crate_expand_failure.cppm");
    std::fs::write(&sentinel_path, b"preserve-sentinel\n").unwrap();
    std::fs::write(&generated_path, b"preserve-generated\n").unwrap();
    let existing_failure = transpiler_bin()
        .arg("--crate")
        .arg(&failure_manifest)
        .arg("--output-dir")
        .arg(&existing_output)
        .arg("--expand")
        .env("PATH", &shim_path)
        .output()
        .unwrap();
    assert!(!existing_failure.status.success());
    assert_eq!(
        std::fs::read(&sentinel_path).unwrap(),
        b"preserve-sentinel\n"
    );
    assert_eq!(
        std::fs::read(&generated_path).unwrap(),
        b"preserve-generated\n"
    );
    assert!(!existing_output.join("CMakeLists.txt").exists());
    assert!(
        std::fs::read_dir(failure.path())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".rusty-cpp-")),
        "failed expansion leaked a staging directory"
    );

    let ambiguous = tempfile::tempdir().unwrap();
    std::fs::create_dir(ambiguous.path().join("src")).unwrap();
    let ambiguous_manifest = ambiguous.path().join("Cargo.toml");
    std::fs::write(
        &ambiguous_manifest,
        "[package]\nname='crate_expand_ambiguous'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(ambiguous.path().join("src/lib.rs"), "pub fn library() {}\n").unwrap();
    std::fs::write(ambiguous.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--all-targets")
        .arg("--manifest-path")
        .arg(&ambiguous_manifest)
        .env("CARGO_TARGET_DIR", ambiguous.path().join("cargo-target"))
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "multi-target fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    let ambiguous_output = ambiguous.path().join("cpp-out");
    let ambiguous_failure = transpiler_bin()
        .arg("--crate")
        .arg(&ambiguous_manifest)
        .arg("--output-dir")
        .arg(&ambiguous_output)
        .arg("--expand")
        .output()
        .unwrap();
    assert!(!ambiguous_failure.status.success());
    assert!(
        String::from_utf8_lossy(&ambiguous_failure.stderr)
            .contains("exactly one unambiguous conventional library target"),
        "ambiguous crate selected a target silently:\n{}",
        String::from_utf8_lossy(&ambiguous_failure.stderr)
    );
    assert!(
        !ambiguous_output.exists(),
        "ambiguous target selection created {}",
        ambiguous_output.display()
    );
}

#[test]
fn crate_expand_publishes_an_exact_complete_local_dependency_tree() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let dependency = root.join("local_dep");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(dependency.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='publish_root'\nversion='0.0.0'\nedition='2021'\n\
         [dependencies]\nlocal_dep={path='local_dep'}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn root_value() -> i32 { local_dep::dep_value() }\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='local_dep'\nversion='0.0.0'\nedition='2021'\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("src/lib.rs"),
        "pub fn dep_value() -> i32 { 7 }\n",
    )
    .unwrap();

    let output_dir = fixture.path().join("cpp-out");
    // These deliberately collide in both directions with the newly generated
    // tree. They also reproduce the V8 review's late recursive-publish shape.
    std::fs::create_dir_all(output_dir.join("CMakeLists.txt")).unwrap();
    std::fs::write(output_dir.join("CMakeLists.txt/old-child"), b"old\n").unwrap();
    std::fs::write(output_dir.join("local_dep"), b"old dependency file\n").unwrap();
    std::fs::write(output_dir.join("stale.cppm"), b"stale generated bytes\n").unwrap();
    std::fs::write(output_dir.join("sentinel.txt"), b"old unrelated bytes\n").unwrap();

    let output = transpiler_bin()
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--expand")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "exact-tree expanded publication failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output_dir.join("publish_root.cppm").is_file());
    assert!(output_dir.join("CMakeLists.txt").is_file());
    assert!(output_dir.join("local_dep/local_dep.cppm").is_file());
    assert!(output_dir.join("local_dep/CMakeLists.txt").is_file());
    assert!(!output_dir.join("stale.cppm").exists());
    assert!(!output_dir.join("sentinel.txt").exists());
    assert_no_crate_output_transaction_artifacts(fixture.path());
}

#[test]
#[cfg(unix)]
fn crate_expand_nested_late_failures_leave_absent_and_existing_outputs_untouched() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    let dependency = root.join("local_dep");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(dependency.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='late_failure_root'\nversion='0.0.0'\nedition='2021'\n\
         [dependencies]\nlocal_dep={path='local_dep'}\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn root_value() -> i32 { 9 }\n").unwrap();
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='local_dep'\nversion='0.0.0'\nedition='2021'\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("src/lib.rs"),
        "pub fn dep_value() -> i32 { 7 }\n",
    )
    .unwrap();

    let real_cargo = Command::new("which").arg("cargo").output().unwrap();
    assert!(real_cargo.status.success());
    let real_cargo = String::from_utf8(real_cargo.stdout)
        .unwrap()
        .trim()
        .to_string();
    let cargo_shim_dir = fixture.path().join("cargo-shim");
    std::fs::create_dir(&cargo_shim_dir).unwrap();
    let cargo_shim = cargo_shim_dir.join("cargo");
    std::fs::write(
        &cargo_shim,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"expand\" ] && [ \"$PWD\" = '{}' ]; then\n  echo forced late root expansion failure >&2\n  exit 86\nfi\nexec '{}' \"$@\"\n",
            root.display(), real_cargo
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&cargo_shim).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo_shim, permissions).unwrap();
    }
    let shim_path = format!(
        "{}:{}",
        cargo_shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let run = |output_dir: &std::path::Path| {
        transpiler_bin()
            .arg("--crate")
            .arg(root.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(output_dir)
            .arg("--expand")
            .env("PATH", &shim_path)
            .output()
            .unwrap()
    };

    let absent_output = fixture.path().join("absent-out");
    let absent_failure = run(&absent_output);
    assert!(!absent_failure.status.success());
    assert!(
        String::from_utf8_lossy(&absent_failure.stderr)
            .contains("forced late root expansion failure")
    );
    assert!(!absent_output.exists());
    assert_no_crate_output_transaction_artifacts(fixture.path());

    let existing_output = fixture.path().join("existing-out");
    std::fs::create_dir_all(existing_output.join("local_dep")).unwrap();
    std::fs::write(existing_output.join("root.cppm"), b"old root\n").unwrap();
    std::fs::write(
        existing_output.join("local_dep/local_dep.cppm"),
        b"old dependency\n",
    )
    .unwrap();
    std::fs::write(existing_output.join("sentinel.txt"), b"preserve exactly\n").unwrap();
    let existing_failure = run(&existing_output);
    assert!(!existing_failure.status.success());
    assert_eq!(
        std::fs::read(existing_output.join("root.cppm")).unwrap(),
        b"old root\n"
    );
    assert_eq!(
        std::fs::read(existing_output.join("local_dep/local_dep.cppm")).unwrap(),
        b"old dependency\n"
    );
    assert_eq!(
        std::fs::read(existing_output.join("sentinel.txt")).unwrap(),
        b"preserve exactly\n"
    );
    assert!(!existing_output.join("CMakeLists.txt").exists());
    assert_no_crate_output_transaction_artifacts(fixture.path());
}

#[test]
#[cfg(unix)]
fn crate_expand_post_expansion_transpile_failure_preserves_existing_output() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("root");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='transpile_failure_root'\nversion='0.0.0'\nedition='2021'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn rustc_valid() {}\n").unwrap();

    let real_cargo = Command::new("which").arg("cargo").output().unwrap();
    assert!(real_cargo.status.success());
    let real_cargo = String::from_utf8(real_cargo.stdout)
        .unwrap()
        .trim()
        .to_string();
    let cargo_shim_dir = fixture.path().join("cargo-shim");
    std::fs::create_dir(&cargo_shim_dir).unwrap();
    let cargo_shim = cargo_shim_dir.join("cargo");
    std::fs::write(
        &cargo_shim,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"expand\" ] && [ \"$PWD\" = '{}' ]; then\n  printf '%s\\n' 'extern \"Rust\" {{ fn bridge(); }}'\n  exit 0\nfi\nexec '{}' \"$@\"\n",
            root.display(), real_cargo
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&cargo_shim).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo_shim, permissions).unwrap();
    }
    let shim_path = format!(
        "{}:{}",
        cargo_shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output_dir = fixture.path().join("existing-out");
    std::fs::create_dir(&output_dir).unwrap();
    std::fs::write(output_dir.join("root.cppm"), b"old root\n").unwrap();
    std::fs::write(output_dir.join("sentinel.txt"), b"preserve exactly\n").unwrap();
    let failure = transpiler_bin()
        .arg("--crate")
        .arg(root.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--expand")
        .env("PATH", shim_path)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(
        String::from_utf8_lossy(&failure.stderr)
            .contains("Transpilation of expanded source failed"),
        "unexpected post-expansion failure:\n{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    assert_eq!(std::fs::read(output_dir.join("root.cppm")).unwrap(), b"old root\n");
    assert_eq!(
        std::fs::read(output_dir.join("sentinel.txt")).unwrap(),
        b"preserve exactly\n"
    );
    assert!(!output_dir.join("CMakeLists.txt").exists());
    assert_no_crate_output_transaction_artifacts(fixture.path());
}

#[test]
fn crate_expand_rejects_dot_output_before_touching_the_source_tree() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("dot-output-root");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let manifest_bytes = b"[package]\nname='dot_output_root'\nversion='0.0.0'\nedition='2021'\n[workspace]\n";
    let source_bytes = b"pub fn value() -> i32 { 7 }\n";
    std::fs::write(root.join("Cargo.toml"), manifest_bytes).unwrap();
    std::fs::write(root.join("src/lib.rs"), source_bytes).unwrap();

    let before = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    let failure = transpiler_bin()
        .current_dir(&root)
        .arg("--crate")
        .arg("./Cargo.toml")
        .arg("--output-dir")
        .arg(".")
        .arg("--expand")
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(
        String::from_utf8_lossy(&failure.stderr).contains("generator-owned child"),
        "unexpected unsafe-output diagnostic:\n{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    let after = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(after, before, "dot-output failure touched the source tree");
    assert_eq!(std::fs::read(root.join("Cargo.toml")).unwrap(), manifest_bytes);
    assert_eq!(std::fs::read(root.join("src/lib.rs")).unwrap(), source_bytes);
    assert_no_crate_output_transaction_artifacts(&root);
}

#[test]
fn crate_expand_accepts_a_bare_manifest_path_with_a_dedicated_output_child() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("bare-manifest-root");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='bare_manifest_root'\nversion='0.0.0'\nedition='2021'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn value() -> i32 { 11 }\n").unwrap();

    let output = transpiler_bin()
        .current_dir(&root)
        .arg("--crate")
        .arg("Cargo.toml")
        .arg("--output-dir")
        .arg("cpp-out")
        .arg("--expand")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "bare manifest expansion failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join("cpp-out/bare_manifest_root.cppm").is_file());
    assert!(root.join("cpp-out/CMakeLists.txt").is_file());
    assert_no_crate_output_transaction_artifacts(&root);
}

#[test]
fn test_crate_mode_missing_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();

    let output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("nonexistent.toml").to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
}

#[test]
fn test_type_map_flag() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    let output_path = dir.path().join("test.cppm");
    let type_map = dir.path().join("types.toml");

    std::fs::write(&input, "fn f(s: serde::Serialize) {}").unwrap();
    std::fs::write(&type_map, "[serde]\nSerialize = \"custom::Serialize\"\n").unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("--type-map")
        .arg(type_map.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("custom::Serialize"));
}

#[test]
fn test_cli_cpp_module_index_flag_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("test.rs");
    let output_path = dir.path().join("test.cppm");
    let index_path = dir.path().join("cpp_index.toml");

    std::fs::write(&input, "use cpp::std as cpp_std;\nfn f() {}").unwrap();
    std::fs::write(
        &index_path,
        r#"
version = 1
[modules.std]
namespace = "std"
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .arg("--cpp-module-index")
        .arg(index_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(cpp.contains("// C++ module import (reserved cpp::): std as cpp_std"));
}

#[test]
fn test_crate_mode_cpp_import_requires_symbol_index() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"cpp_dep\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"cpp_dep\"\n",
    )
    .unwrap();
    std::fs::write(src_dir.join("lib.rs"), "use cpp::std;\npub fn f() {}").unwrap();

    let out_dir = dir.path().join("cpp_out");

    let output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .arg("--output-dir")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no C++ module symbol index is configured"));
    assert!(stderr.contains("--cpp-module-index"));
}

#[test]
fn test_crate_mode_cpp_import_with_symbol_index_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"cpp_dep\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"cpp_dep\"\n",
    )
    .unwrap();
    std::fs::write(src_dir.join("lib.rs"), "use cpp::std;\npub fn f() {}").unwrap();
    let index_path = dir.path().join("cpp_index.toml");
    std::fs::write(
        &index_path,
        r#"
version = 1
[modules.std]
namespace = "std"
"#,
    )
    .unwrap();

    let out_dir = dir.path().join("cpp_out");

    let output = transpiler_bin()
        .arg("--crate")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .arg("--output-dir")
        .arg(out_dir.to_str().unwrap())
        .arg("--cpp-module-index")
        .arg(index_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("cpp_dep.cppm").exists());
}

#[test]
fn test_crate_mode_with_path_dependency() {
    let dir = tempfile::tempdir().unwrap();

    // Create dependency crate: my_utils
    let utils_dir = dir.path().join("my_utils");
    let utils_src = utils_dir.join("src");
    std::fs::create_dir_all(&utils_src).unwrap();
    std::fs::write(
        utils_dir.join("Cargo.toml"),
        "[package]\nname = \"my_utils\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"my_utils\"\n",
    )
    .unwrap();
    std::fs::write(utils_src.join("lib.rs"), "pub fn helper() -> i32 { 42 }").unwrap();

    // Create main crate: my_app (depends on my_utils via path)
    let app_dir = dir.path().join("my_app");
    let app_src = app_dir.join("src");
    std::fs::create_dir_all(&app_src).unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"my_app\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"my_app\"\n\n[dependencies]\nmy_utils = { path = \"../my_utils\" }\n",
    )
    .unwrap();
    std::fs::write(
        app_src.join("lib.rs"),
        "pub fn run() -> i32 { my_utils::helper() }",
    )
    .unwrap();

    let out_dir = dir.path().join("cpp_out");

    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml").to_str().unwrap())
        .arg("--output-dir")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Main crate output
    assert!(out_dir.join("my_app.cppm").exists());

    // Dependency crate output (in subdirectory)
    assert!(out_dir.join("my_utils").join("my_utils.cppm").exists());
    assert!(out_dir.join("my_utils").join("CMakeLists.txt").exists());

    // Main CMakeLists.txt should have add_subdirectory and target_link_libraries
    let cmake = std::fs::read_to_string(out_dir.join("CMakeLists.txt")).unwrap();
    assert!(cmake.contains("add_subdirectory(my_utils)"));
    assert!(cmake.contains("target_link_libraries(my_app"));

    // Stdout should mention recursive transpilation
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("will transpile recursively"));
}

#[test]
fn test_crate_mode_uses_exact_local_rusty_package_as_rustc_only_runtime_facade() {
    let dir = tempfile::tempdir().unwrap();

    let runtime_dir = dir.path().join("rusty-rustc");
    std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
    std::fs::write(
        runtime_dir.join("Cargo.toml"),
        "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        runtime_dir.join("src/lib.rs"),
        "#![deny(unsafe_code)]\npub struct Function<F: ?Sized> { inner: Option<Box<F>> }\n",
    )
    .unwrap();

    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nrusty = { path = \"../rusty-rustc\" }\n",
    )
    .unwrap();
    std::fs::write(
        app_dir.join("src/lib.rs"),
        "pub type Callback = rusty::Function<dyn Fn(i32) -> i32>;\n",
    )
    .unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated = std::fs::read_to_string(out_dir.join("app.cppm")).unwrap();
    assert!(
        generated.contains("rusty::Function<int32_t(int32_t) const>"),
        "{generated}"
    );
    assert!(
        !out_dir.join("rusty").exists(),
        "runtime facade must not produce a generated dependency directory"
    );
    let cmake = std::fs::read_to_string(out_dir.join("CMakeLists.txt")).unwrap();
    assert!(!cmake.contains("add_subdirectory(rusty)"), "{cmake}");
    assert!(!cmake.contains("PRIVATE rusty"), "{cmake}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("provided by the rusty C++ runtime"),
        "{stdout}"
    );
    assert!(stdout.contains("rustc facade is not generated"), "{stdout}");
}

#[test]
fn test_crate_mode_rusty_runtime_identity_mismatches_fail_before_output() {
    for (case_name, dependency, runtime_manifest, expected) in [
        (
            "package_mismatch",
            "rusty = { path = \"../runtime\" }",
            Some(
                "[package]\nname = \"not-rusty\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.rs\"\n",
            ),
            "refusing to omit a non-runtime crate",
        ),
        (
            "library_mismatch",
            "rusty = { path = \"../runtime\" }",
            Some(
                "[package]\nname = \"rusty\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"not_rusty\"\npath = \"src/lib.rs\"\n",
            ),
            "does not expose an ordinary Rust library target named exactly 'rusty'",
        ),
        (
            "renamed_reserved_package",
            "runtime = { package = \"rusty\", path = \"../runtime\" }",
            Some(
                "[package]\nname = \"rusty\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.rs\"\n",
            ),
            "renames reserved runtime package 'rusty'",
        ),
        (
            "registry_package",
            "rusty = \"1\"",
            None,
            "reserved for an exact local path dependency",
        ),
        (
            "optional_runtime",
            "rusty = { path = \"../runtime\", optional = true }",
            Some(
                "[package]\nname = \"rusty\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/lib.rs\"\n",
            ),
            "reserved runtime identity 'rusty' but is optional",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        if let Some(runtime_manifest) = runtime_manifest {
            let runtime_dir = dir.path().join("runtime");
            std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
            std::fs::write(runtime_dir.join("Cargo.toml"), runtime_manifest).unwrap();
            std::fs::write(runtime_dir.join("src/lib.rs"), "pub struct Facade;\n").unwrap();
        }

        let app_dir = dir.path().join("app");
        std::fs::create_dir_all(app_dir.join("src")).unwrap();
        std::fs::write(
            app_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{dependency}\n"
            ),
        )
        .unwrap();
        std::fs::write(app_dir.join("src/lib.rs"), "pub fn marker_free() {}\n").unwrap();

        let out_dir = dir.path().join("cpp_out");
        let output = transpiler_bin()
            .arg("--crate")
            .arg(app_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&out_dir)
            .output()
            .expect("failed to run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "case {case_name} unexpectedly passed"
        );
        assert!(stderr.contains(expected), "case {case_name}: {stderr}");
        assert!(
            !out_dir.exists(),
            "case {case_name} created output before identity validation"
        );
    }
}

#[test]
fn test_crate_mode_preserves_unrelated_workspace_inherited_dependency() {
    let dir = tempfile::tempdir().unwrap();
    let helper_dir = dir.path().join("helper");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(helper_dir.join("src")).unwrap();
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"helper\"]\nresolver = \"2\"\n\n[workspace.dependencies]\nhelper = { path = \"helper\" }\n",
    )
    .unwrap();
    std::fs::write(
        helper_dir.join("Cargo.toml"),
        "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(helper_dir.join("src/lib.rs"), "pub fn helper() {}\n").unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nhelper = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(app_dir.join("src/lib.rs"), "pub fn marker_free() {}\n").unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("app.cppm").exists());
}

#[test]
fn test_crate_mode_resolves_exact_workspace_inherited_rusty_facade() {
    let dir = tempfile::tempdir().unwrap();
    let runtime_dir = dir.path().join("runtime");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"runtime\"]\nresolver = \"2\"\n\n[workspace.dependencies]\nrusty = { path = \"runtime\" }\n",
    )
    .unwrap();
    std::fs::write(
        runtime_dir.join("Cargo.toml"),
        "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n",
    )
    .unwrap();
    std::fs::write(
        runtime_dir.join("src/lib.rs"),
        "pub struct Function<F: ?Sized> { inner: Option<Box<F>> }\n",
    )
    .unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nrusty = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(
        app_dir.join("src/lib.rs"),
        "pub type Callback = rusty::Function<dyn Fn()>;\n",
    )
    .unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!out_dir.join("rusty").exists());
    let cmake = std::fs::read_to_string(out_dir.join("CMakeLists.txt")).unwrap();
    assert!(!cmake.contains("add_subdirectory(rusty)"), "{cmake}");
    assert!(!cmake.contains("PRIVATE rusty"), "{cmake}");
}

#[test]
fn test_crate_mode_rejects_workspace_alias_to_reserved_rusty_package() {
    let dir = tempfile::tempdir().unwrap();
    let runtime_dir = dir.path().join("runtime");
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"runtime\"]\nresolver = \"2\"\n\n[workspace.dependencies]\nruntime = { package = \"rusty\", path = \"runtime\" }\n",
    )
    .unwrap();
    std::fs::write(
        runtime_dir.join("Cargo.toml"),
        "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(runtime_dir.join("src/lib.rs"), "pub struct Function;\n").unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nruntime = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(app_dir.join("src/lib.rs"), "pub fn marker_free() {}\n").unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("renames reserved runtime package 'rusty'"),
        "{stderr}"
    );
    assert!(!out_dir.exists());
}

#[test]
fn test_crate_mode_rejects_nonordinary_rusty_library_targets() {
    for (case_name, runtime_manifest) in [
        (
            "autolib_disabled",
            "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\nautolib = false\n\n[[bin]]\nname = \"rusty-tool\"\npath = \"src/main.rs\"\n",
        ),
        (
            "proc_macro",
            "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\nproc-macro = true\n",
        ),
        (
            "cdylib_only",
            "[package]\nname = \"rusty\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let runtime_dir = dir.path().join("runtime");
        let app_dir = dir.path().join("app");
        std::fs::create_dir_all(runtime_dir.join("src")).unwrap();
        std::fs::create_dir_all(app_dir.join("src")).unwrap();
        std::fs::write(runtime_dir.join("Cargo.toml"), runtime_manifest).unwrap();
        std::fs::write(runtime_dir.join("src/lib.rs"), "pub struct Facade;\n").unwrap();
        std::fs::write(runtime_dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nrusty = { path = \"../runtime\" }\n",
        )
        .unwrap();
        std::fs::write(app_dir.join("src/lib.rs"), "pub fn marker_free() {}\n").unwrap();

        let out_dir = dir.path().join("cpp_out");
        let output = transpiler_bin()
            .arg("--crate")
            .arg(app_dir.join("Cargo.toml"))
            .arg("--output-dir")
            .arg(&out_dir)
            .output()
            .expect("failed to run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "case {case_name}: {stderr}");
        assert!(
            stderr.contains("does not expose an ordinary Rust library target named exactly 'rusty'"),
            "case {case_name}: {stderr}"
        );
        assert!(!out_dir.exists(), "case {case_name} created output");
    }
}

#[test]
fn test_crate_mode_target_qualified_rusty_identity_fails_before_output() {
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[target.'cfg(unix)'.dependencies]\nruntime = { package = \"rusty\", version = \"1\" }\n",
    )
    .unwrap();
    std::fs::write(app_dir.join("src/lib.rs"), "pub fn marker_free() {}\n").unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("target-qualified dependency 'runtime'"),
        "{stderr}"
    );
    assert!(!out_dir.exists(), "created output at {}", out_dir.display());
}

#[test]
fn test_crate_mode_transitive_rusty_identity_mismatch_fails_before_root_output() {
    let dir = tempfile::tempdir().unwrap();

    let impostor_dir = dir.path().join("impostor");
    std::fs::create_dir_all(impostor_dir.join("src")).unwrap();
    std::fs::write(
        impostor_dir.join("Cargo.toml"),
        "[package]\nname = \"not-rusty\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(impostor_dir.join("src/lib.rs"), "pub struct Impostor;\n").unwrap();

    let middle_dir = dir.path().join("middle");
    std::fs::create_dir_all(middle_dir.join("src")).unwrap();
    std::fs::write(
        middle_dir.join("Cargo.toml"),
        "[package]\nname = \"middle\"\nversion = \"0.1.0\"\n\n[dependencies]\nrusty = { path = \"../impostor\" }\n",
    )
    .unwrap();
    std::fs::write(middle_dir.join("src/lib.rs"), "pub fn middle() {}\n").unwrap();

    let app_dir = dir.path().join("app");
    std::fs::create_dir_all(app_dir.join("src")).unwrap();
    std::fs::write(
        app_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmiddle = { path = \"../middle\" }\n",
    )
    .unwrap();
    std::fs::write(app_dir.join("src/lib.rs"), "pub fn app() {}\n").unwrap();

    let out_dir = dir.path().join("cpp_out");
    let output = transpiler_bin()
        .arg("--crate")
        .arg(app_dir.join("Cargo.toml"))
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("whole local-dependency closure preflight failed before output"),
        "{stderr}"
    );
    assert!(
        stderr.contains("refusing to omit a non-runtime crate"),
        "{stderr}"
    );
    assert!(!out_dir.exists(), "created output at {}", out_dir.display());
}

// ── parity-test subcommand tests ────────────────────────

#[test]
fn test_parity_test_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test_crate\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .arg("--dry-run")
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parity Test: test_crate"));
    assert!(stdout.contains("[dry-run]"));
    assert!(stdout.contains("Stage A"));
    assert!(stdout.contains("Stage B"));
}

#[test]
fn test_parity_test_missing_manifest() {
    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg("nonexistent.toml")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Manifest not found"));
}

#[test]
fn test_parity_test_invalid_stop_after() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(dir.path().join("Cargo.toml").to_str().unwrap())
        .arg("--stop-after")
        .arg("invalid")
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid --stop-after"));
}

#[test]
fn test_parity_test_help() {
    let output = transpiler_bin()
        .arg("parity-test")
        .arg("--help")
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--manifest-path"));
    assert!(stdout.contains("--stop-after"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--no-baseline"));
}

// ── inline-rust subcommand tests ────────────────────────

fn inline_rust_fixture(gen_hash: &str) -> String {
    format!(
        r#"#if RUSTYCPP_RUST
fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.add version=1 rust_sha256={}*/
// stale generated text
/*RUSTYCPP:GEN-END id=demo.add*/
"#,
        gen_hash
    )
}

const INLINE_RUST_FIRST: &str = "fn first() -> i32 {\n    1\n}";
const INLINE_RUST_SECOND: &str = "fn second() -> i32 {\n    2\n}";

fn inline_rust_emit_fixture() -> String {
    format!(
        r#"#if RUSTYCPP_RUST
{INLINE_RUST_FIRST}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.first version=1 rust_sha256=d65aec84cdd9517a9441838327ee2678f6af0a73364499331d155c6f4bb090ae*/
// generated first
/*RUSTYCPP:GEN-END id=demo.first*/

#if RUSTYCPP_RUST
{INLINE_RUST_SECOND}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.second version=1 rust_sha256=d042b306f6e1e636c47813a66ec381e91b4431c3b95a53cd90abd44914fb9d16*/
// generated second
/*RUSTYCPP:GEN-END id=demo.second*/
"#
    )
}

fn emit_rust(
    file: &std::path::Path,
    output: &std::path::Path,
    ids: &[&str],
) -> std::process::Output {
    let mut command = transpiler_bin();
    command
        .arg("inline-rust")
        .arg("--emit-rust")
        .arg(output)
        .arg("--files")
        .arg(file);
    for id in ids {
        command.arg("--block-id").arg(id);
    }
    command.output().expect("failed to run emit-rust")
}

#[test]
fn test_inline_rust_check_fails_on_hash_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("demo.hpp");
    std::fs::write(&file, inline_rust_fixture("deadbeef")).unwrap();

    let output = transpiler_bin()
        .arg("inline-rust")
        .arg("--check")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("hash mismatch"));
}

#[test]
fn test_inline_rust_rewrite_then_check_passes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("demo.hpp");
    std::fs::write(&file, inline_rust_fixture("deadbeef")).unwrap();

    let rewrite = transpiler_bin()
        .arg("inline-rust")
        .arg("--rewrite")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run rewrite");
    assert!(
        rewrite.status.success(),
        "rewrite stderr: {}",
        String::from_utf8_lossy(&rewrite.stderr)
    );

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("int32_t add(int32_t a, int32_t b);"));
    assert!(content.contains("int32_t add(int32_t a, int32_t b) {"));
    assert!(!content.contains("#include <cstdint>"));
    assert!(!content.contains("// stale generated text"));
    assert!(!content.contains("\n#else\n"));
    assert!(!content.contains("RUSTYCPP:RUST-BEGIN"));
    assert!(!content.contains("@rust {"));
    assert!(content.contains("rust_sha256="));
    assert!(!content.contains("rust_sha256=deadbeef"));

    let check = transpiler_bin()
        .arg("inline-rust")
        .arg("--check")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run check");
    assert!(
        check.status.success(),
        "check stderr: {}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn test_inline_rust_emit_all_in_source_order_and_selected_in_requested_order() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("demo.hpp");
    let all_output = dir.path().join("all.rs");
    let selected_output = dir.path().join("selected.rs");
    let source = inline_rust_emit_fixture();
    std::fs::write(&file, &source).unwrap();

    let all = emit_rust(&file, &all_output, &[]);
    assert!(
        all.status.success(),
        "emit-all stderr: {}",
        String::from_utf8_lossy(&all.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&all_output).unwrap(),
        format!("{INLINE_RUST_FIRST}\n\n{INLINE_RUST_SECOND}\n")
    );

    let selected = emit_rust(&file, &selected_output, &["demo.second", "demo.first"]);
    assert!(
        selected.status.success(),
        "emit-selected stderr: {}",
        String::from_utf8_lossy(&selected.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&selected_output).unwrap(),
        format!("{INLINE_RUST_SECOND}\n\n{INLINE_RUST_FIRST}\n")
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
}

#[test]
fn test_inline_rust_emit_requires_exactly_one_input() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.hpp");
    let second = dir.path().join("second.hpp");
    let output_path = dir.path().join("out.rs");
    std::fs::write(&first, inline_rust_emit_fixture()).unwrap();
    std::fs::write(&second, inline_rust_emit_fixture()).unwrap();
    std::fs::write(&output_path, "sentinel").unwrap();

    let output = transpiler_bin()
        .arg("inline-rust")
        .arg("--emit-rust")
        .arg(&output_path)
        .arg("--files")
        .arg(&first)
        .arg(&second)
        .output()
        .expect("failed to run emit-rust");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires exactly one --files input"));
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");
}

#[test]
fn test_inline_rust_emit_rejects_missing_and_duplicate_requested_ids() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("demo.hpp");
    let output_path = dir.path().join("out.rs");
    std::fs::write(&file, inline_rust_emit_fixture()).unwrap();
    std::fs::write(&output_path, "sentinel").unwrap();

    let missing = emit_rust(&file, &output_path, &["demo.missing"]);
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("missing inline block id=demo.missing")
    );
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");

    let duplicate = emit_rust(&file, &output_path, &["demo.first", "demo.first"]);
    assert!(!duplicate.status.success());
    assert!(
        String::from_utf8_lossy(&duplicate.stderr)
            .contains("duplicate requested block id=demo.first")
    );
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");
}

#[test]
fn test_inline_rust_emit_requires_matching_hash_gen_v1_and_gen_region() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("out.rs");

    let bad_hash_file = dir.path().join("bad_hash.hpp");
    let bad_hash = inline_rust_emit_fixture().replace(
        "d65aec84cdd9517a9441838327ee2678f6af0a73364499331d155c6f4bb090ae",
        "deadbeef",
    );
    std::fs::write(&bad_hash_file, bad_hash).unwrap();
    std::fs::write(&output_path, "sentinel").unwrap();
    let bad_hash_output = emit_rust(&bad_hash_file, &output_path, &["demo.first"]);
    assert!(!bad_hash_output.status.success());
    assert!(String::from_utf8_lossy(&bad_hash_output.stderr).contains("hash mismatch"));
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");

    let bad_version_file = dir.path().join("bad_version.hpp");
    let bad_version = inline_rust_emit_fixture().replacen("version=1", "version=2", 1);
    std::fs::write(&bad_version_file, bad_version).unwrap();
    let bad_version_output = emit_rust(&bad_version_file, &output_path, &["demo.first"]);
    assert!(!bad_version_output.status.success());
    assert!(
        String::from_utf8_lossy(&bad_version_output.stderr)
            .contains("unsupported GEN marker version 2; expected 1")
    );
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");

    let missing_gen_file = dir.path().join("missing_gen.hpp");
    std::fs::write(
        &missing_gen_file,
        format!("#if RUSTYCPP_RUST\n{INLINE_RUST_FIRST}\n#endif\n"),
    )
    .unwrap();
    let missing_gen_output = emit_rust(&missing_gen_file, &output_path, &[]);
    assert!(!missing_gen_output.status.success());
    assert!(
        String::from_utf8_lossy(&missing_gen_output.stderr).contains("missing generated region")
    );
    assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "sentinel");
}

#[test]
fn test_inline_rust_emit_refuses_to_overwrite_source() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("demo.hpp");
    let source = inline_rust_emit_fixture();
    std::fs::write(&file, &source).unwrap();

    let output = emit_rust(&file, &file, &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing to emit Rust over source"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
}

// ── #[cpp_ctor] lowering ────────────────────────────────

#[test]
fn test_inline_rust_cpp_ctor_lowers_to_real_ctor() {
    // `#[cpp_ctor]` on a factory whose body is a single `Self { ... }`
    // literal should emit a C++ constructor (no `static`, no return
    // type, name = owner struct, body = member init list) — instead of
    // the default `static Owner Owner::new_(args)` factory.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ctor.hpp");
    let source = r#"#if RUSTYCPP_RUST
struct Calc {
    limit: u32,
    seen: u32,
}

impl Calc {
    #[cpp_ctor]
    fn new(limit: u32) -> Calc {
        Calc { limit: limit, seen: 0u32 }
    }

    fn over(&self) -> bool {
        self.seen >= self.limit
    }
}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.calc version=1 rust_sha256=deadbeef*/
/*RUSTYCPP:GEN-END id=demo.calc*/
"#;
    std::fs::write(&file, source).unwrap();

    let rewrite = transpiler_bin()
        .arg("inline-rust")
        .arg("--rewrite")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run rewrite");
    assert!(
        rewrite.status.success(),
        "rewrite stderr: {}",
        String::from_utf8_lossy(&rewrite.stderr)
    );

    let content = std::fs::read_to_string(&file).unwrap();
    // In-class declaration: `Calc(uint32_t limit);` — no `static`, no
    // return type.
    assert!(
        content.contains("Calc(uint32_t limit);"),
        "missing ctor decl: {content}"
    );
    assert!(
        !content.contains("static Calc Calc::new_"),
        "factory leaked through despite #[cpp_ctor]: {content}"
    );
    assert!(
        !content.contains("static Calc new_"),
        "factory leaked through despite #[cpp_ctor]: {content}"
    );
    // Out-of-line definition with member init list.
    assert!(
        content.contains("Calc::Calc(uint32_t limit)"),
        "missing out-of-line ctor: {content}"
    );
    assert!(
        // A field initialized from a ctor param moves it (Rust struct-literal
        // semantics): `field(std::move(param))`.
        content.contains(": limit(std::move(limit))"),
        "missing init list head: {content}"
    );
    assert!(
        content.contains("seen("),
        "missing seen init: {content}"
    );
    // Regular method continues to emit normally.
    assert!(
        content.contains("bool over() const"),
        "missing regular method: {content}"
    );
}

#[test]
fn test_inline_rust_drop_cpp_ctor_is_nothrow_and_has_no_fieldwise_bypass() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("pinned_drop_ctor.hpp");
    let source = r#"#if RUSTYCPP_RUST
#[cfg_attr(any(), cpp_no_fieldwise_ctor)]
struct PinnedTask {
    value: i32,
    _pin: rusty::marker::PhantomPinned,
}

impl PinnedTask {
    #[cpp_ctor]
    #[cfg_attr(any(), cpp_explicit)]
    fn new(value: i32) -> PinnedTask {
        PinnedTask {
            value: value,
            _pin: rusty::marker::PhantomPinned {},
        }
    }
}

impl Drop for PinnedTask {
    #[cfg_attr(any(), cpp_noexcept)]
    fn drop(&mut self) {}
}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.pinned_drop_ctor version=1 rust_sha256=deadbeef*/
/*RUSTYCPP:GEN-END id=demo.pinned_drop_ctor*/
"#;
    std::fs::write(&file, source).unwrap();

    let rewrite = transpiler_bin()
        .arg("inline-rust")
        .arg("--rewrite")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run rewrite");
    assert!(
        rewrite.status.success(),
        "rewrite stderr: {}",
        String::from_utf8_lossy(&rewrite.stderr)
    );

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("explicit PinnedTask(int32_t value);"),
        "{content}"
    );
    assert!(
        content.contains("PinnedTask::PinnedTask(int32_t value)"),
        "{content}"
    );
    assert!(!content.contains("value_init"), "{content}");
    assert!(!content.contains("_pin_init"), "{content}");
    assert!(
        content.contains("PinnedTask(PinnedTask&&) = delete;"),
        "{content}"
    );
    assert!(
        content.contains("~PinnedTask() noexcept(true)"),
        "{content}"
    );
}

#[test]
fn test_result_ok_qualifier_preserves_signature_t() {
    // Regression: when an impl block has more type params than the host
    // struct (impl<BorrowType,K,V,NodeType> on a Handle<Node,Type>) and
    // a method returns Result<Handle<NodeRef<…>,marker::KV>, Self>, the
    // Ok-arm explicit Result qualifier used to leak `Self` into the T
    // position because:
    //   1. the impl-level params get decomposed into the struct's `Node`
    //      via __TemplateArgs<Node>::arg_N, so they're not in
    //      `type_param_scopes`; and
    //   2. the placeholder-check therefore flagged the signature's T as
    //      unresolved, routing through inference, which read `Self` from
    //      the ctor expression's return type (`Handle::new_kv` is on a
    //      parallel impl whose Self is the KV-specialized handle).
    // After the fix, when the inferred type from the ctor arg is plain
    // `Self`, the substitution is skipped and the signature's T is
    // preserved as-is.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("ok_qualifier.rs");
    let output_path = dir.path().join("ok_qualifier.cppm");

    std::fs::write(
        &input,
        r#"
pub mod marker {
    pub enum Edge {}
    pub enum KV {}
}
pub struct NodeRef<BorrowType, K, V, NodeType> {
    pub _b: std::marker::PhantomData<BorrowType>,
    pub _k: std::marker::PhantomData<K>,
    pub _v: std::marker::PhantomData<V>,
    pub _n: std::marker::PhantomData<NodeType>,
}
pub struct Handle<Node, Type> {
    pub node: Node,
    pub idx: usize,
    pub _t: std::marker::PhantomData<Type>,
}
impl<BorrowType, K, V, NodeType> Handle<NodeRef<BorrowType, K, V, NodeType>, marker::KV> {
    pub unsafe fn new_kv(node: NodeRef<BorrowType, K, V, NodeType>, idx: usize) -> Self {
        Handle { node, idx, _t: std::marker::PhantomData }
    }
}
impl<BorrowType, K, V, NodeType> Handle<NodeRef<BorrowType, K, V, NodeType>, marker::Edge> {
    pub fn left_kv(
        self,
    ) -> Result<Handle<NodeRef<BorrowType, K, V, NodeType>, marker::KV>, Self> {
        if self.idx > 0 {
            Ok(unsafe { Handle::new_kv(self.node, self.idx - 1) })
        } else {
            Err(self)
        }
    }
}
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    // The buggy emit produced `Result<Handle<Node, Type>, Handle<Node, Type>>::Ok(`
    // for the Ok arm — both T and E collapsed to Self. With the fix the Ok
    // arm should keep the signature's full `Handle<NodeRef<…>, ::marker::KV>`
    // in the T position.
    assert!(
        !cpp.contains("Result<Handle<Node, Type>, Handle<Node, Type>>::Ok("),
        "Ok-arm Result qualifier still collapses to Self<Self>:\n{cpp}"
    );
    assert!(
        cpp.contains("::marker::KV>, Handle<Node, Type>>::Ok("),
        "Ok-arm Result qualifier missing expected `<…KV>, Handle<Node, Type>>::Ok(`:\n{cpp}"
    );
}

#[test]
fn test_cxx_namespace_wraps_exports() {
    // Verify the `--cxx-namespace` flag wraps exports in
    // `export namespace NS { … }`. Lets sibling modules export the
    // same names without colliding at importer scope — see
    // rusty-std-book §2.10.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("ns_test.rs");
    let output_path = dir.path().join("ns_test.cppm");

    std::fs::write(
        &input,
        r#"
pub struct Widget { pub x: i32 }
pub fn make_widget(x: i32) -> Widget { Widget { x } }
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("foo")
        .arg("--cxx-namespace")
        .arg("foo::bar")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    // Must have the module declaration first.
    assert!(
        cpp.contains("export module foo;"),
        "module decl missing:\n{cpp}"
    );
    // Then the namespace open …
    assert!(
        cpp.contains("namespace foo::bar {"),
        "namespace-open missing:\n{cpp}"
    );
    // Must NOT be `export namespace` — that would nest exports
    // since inner items already carry their own `export` keyword,
    // and C++20 rejects nested export declarations.
    assert!(
        !cpp.contains("export namespace foo::bar"),
        "should not use `export namespace` (nested exports are ill-formed):\n{cpp}"
    );
    // … the struct inside …
    assert!(
        cpp.contains("export struct Widget"),
        "Widget definition missing:\n{cpp}"
    );
    // … and a matching close.
    assert!(
        cpp.contains("} // namespace foo::bar"),
        "namespace-close missing:\n{cpp}"
    );
    // Order: namespace open before the struct, close after.
    let open_pos = cpp.find("namespace foo::bar {").unwrap();
    let struct_pos = cpp.find("export struct Widget").unwrap();
    let close_pos = cpp.find("} // namespace foo::bar").unwrap();
    assert!(
        open_pos < struct_pos && struct_pos < close_pos,
        "ordering wrong: open={open_pos} struct={struct_pos} close={close_pos}\n{cpp}"    );
}

#[test]
fn test_inline_rust_cpp_ctor_no_fields() {
    // An empty struct ctor should emit `Owner() {}`.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty_ctor.hpp");
    let source = r#"#if RUSTYCPP_RUST
struct Empty {
}

impl Empty {
    #[cpp_ctor]
    fn new() -> Empty {
        Empty {}
    }
}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.empty version=1 rust_sha256=deadbeef*/
/*RUSTYCPP:GEN-END id=demo.empty*/
"#;
    std::fs::write(&file, source).unwrap();

    let rewrite = transpiler_bin()
        .arg("inline-rust")
        .arg("--rewrite")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run rewrite");
    assert!(
        rewrite.status.success(),
        "rewrite stderr: {}",
        String::from_utf8_lossy(&rewrite.stderr)
    );

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("Empty();"),
        "missing empty ctor decl: {content}"
    );
    assert!(
        content.contains("Empty::Empty() {}"),
        "missing empty ctor def: {content}"
    );
}

fn test_cxx_namespace_off_by_default() {
    // Without the flag, exports stay flat — legacy ports rely on this
    // and the migration would be intrusive. Off-by-default protects them.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("flat_test.rs");
    let output_path = dir.path().join("flat_test.cppm");

    std::fs::write(
        &input,
        "pub struct Widget { pub x: i32 }\n",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("foo")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        !cpp.contains("export namespace"),
        "flag-off mode should not emit `export namespace`:\n{cpp}"
    );
    assert!(cpp.contains("export struct Widget"));
}

#[test]
fn test_auto_namespace_derives_from_module_name() {
    // --auto-namespace auto-derives the C++ namespace from --module-name
    // by replacing `.` with `::`. The output should be wrapped in
    // `namespace btree_port::btree::map { … }`.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("auto_ns.rs");
    let output_path = dir.path().join("auto_ns.cppm");

    std::fs::write(
        &input,
        "pub struct Widget { pub x: i32 }\n",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("btree_port.btree.map")
        .arg("--auto-namespace")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        cpp.contains("namespace btree_port::btree::map {"),
        "auto-derived namespace missing:\n{cpp}"
    );
    assert!(
        cpp.contains("} // namespace btree_port::btree::map"),
        "namespace close missing:\n{cpp}"
    );
    assert!(
        !cpp.contains("export namespace btree_port::btree::map"),
        "should be plain `namespace`, not `export namespace`:\n{cpp}"    );
}

#[test]
fn test_inline_rust_no_attribute_keeps_factory() {
    // Without `#[cpp_ctor]`, factory-style `fn new` continues to lower
    // to `static Owner Owner::new_(args)` — preserves backward
    // compatibility for all existing call sites.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("factory.hpp");
    let source = r#"#if RUSTYCPP_RUST
struct Calc {
    limit: u32,
}

impl Calc {
    fn new(limit: u32) -> Calc {
        Calc { limit: limit }
    }
}
#endif
/*RUSTYCPP:GEN-BEGIN id=demo.fact version=1 rust_sha256=deadbeef*/
/*RUSTYCPP:GEN-END id=demo.fact*/
"#;
    std::fs::write(&file, source).unwrap();

    let rewrite = transpiler_bin()
        .arg("inline-rust")
        .arg("--rewrite")
        .arg("--files")
        .arg(file.to_str().unwrap())
        .output()
        .expect("failed to run rewrite");
    assert!(rewrite.status.success());

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(
        content.contains("static Calc new_(uint32_t limit);"),
        "factory decl missing without attr: {content}"
    );
    assert!(
        !content.contains("Calc(uint32_t limit);"),
        "ctor leaked through without attr: {content}"
    );
}

fn test_auto_namespace_explicit_override_wins() {
    // If both --auto-namespace and --cxx-namespace are passed, the
    // explicit --cxx-namespace value takes precedence.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("override.rs");
    let output_path = dir.path().join("override.cppm");

    std::fs::write(
        &input,
        "pub struct Widget { pub x: i32 }\n",
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("btree_port.btree.map")
        .arg("--auto-namespace")
        .arg("--cxx-namespace")
        .arg("manual_override")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(output.status.success());
    let cpp = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        cpp.contains("namespace manual_override {"),
        "explicit --cxx-namespace should win over --auto-namespace:\n{cpp}"
    );
    assert!(
        !cpp.contains("namespace btree_port::btree::map {"),
        "auto-derived namespace should not be used when explicit is given:\n{cpp}"    );
}

#[test]
fn test_primitive_impl_self_receiver_numeric_lowering() {
    // Issue #40: `self` inside `impl … for <primitive>` must type as the
    // primitive so the receiver-type-gated byte-conversion lowerings fire
    // (scalar Serialize impls: `self.to_le_bytes()`); f64 exercises the
    // float variant (std::byteswap is integral-only — floats reverse the
    // byte array instead).
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("prim_self.rs");
    let output_path = dir.path().join("prim_self.cppm");

    std::fs::write(
        &input,
        r#"
pub trait Ser { fn ser(&self) -> u8; }
impl Ser for i32 {
    fn ser(&self) -> u8 { self.to_le_bytes()[0] }
}
impl Ser for f64 {
    fn ser(&self) -> u8 { self.to_le_bytes()[0] }
}
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("prim")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    // The raw member call must be gone in both impls…
    assert!(
        !cpp.contains("self_.to_le_bytes()"),
        "raw member to_le_bytes survived on a scalar receiver:\n{cpp}"
    );
    // …replaced by the bit_cast lowering (int form uses byteswap on the
    // big-endian path; float form reverses the array).
    assert!(
        cpp.contains("std::bit_cast"),
        "bit_cast byte-conversion lowering missing:\n{cpp}"
    );
    assert!(
        cpp.contains("std::reverse"),
        "float to_le_bytes variant (array reverse) missing:\n{cpp}"
    );
}
#[test]
fn test_cxx_namespace_requalifies_crate_root_item_refs() {
    // Issue #37: the expression emitters conservatively global-qualify
    // crate-local free-fn references as `::callee` — fine in the legacy
    // flat-export mode, but once `--cxx-namespace` wraps the purview
    // those refs look in the (now-empty) global namespace and miss.
    // The wrap-close must re-qualify them to `::<ns>::callee`, exactly
    // as `wrap_module_purview_in_crate_namespace` does via its Rule 4.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("ns_requal.rs");
    let output_path = dir.path().join("ns_requal.cppm");

    std::fs::write(
        &input,
        r#"
pub fn callee(x: i64) -> usize { if x > 0 { 1 } else { 2 } }
pub fn caller(x: i64) -> usize { callee(x) }
"#,
    )
    .unwrap();

    let output = transpiler_bin()
        .arg(input.to_str().unwrap())
        .arg("--module-name")
        .arg("foo")
        .arg("--cxx-namespace")
        .arg("foo::bar")
        .arg("-o")
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cpp = std::fs::read_to_string(&output_path).unwrap();
    // The call may be emitted unqualified (resolves within the wrapping
    // namespace) or fully qualified against it — but a bare global
    // `::callee(` must NOT survive the wrap.
    let bare_global = cpp
        .match_indices("::callee(")
        .any(|(pos, _)| !cpp[..pos].ends_with("foo::bar") && !cpp[..pos].ends_with("bar"));
    assert!(
        !bare_global,
        "bare global-qualified crate-root call survived the namespace wrap:\n{cpp}"
    );
    // And the requalified (or unqualified) call still exists somewhere.
    assert!(
        cpp.contains("callee("),
        "caller lost its call entirely:\n{cpp}"
    );
}

#[test]
fn cargo_feature_context_authentication_is_identical_in_direct_and_parity_lanes() {
    fn write_fixture(root: &std::path::Path, package_rename: bool) -> std::path::PathBuf {
        let provider = root.join("provider");
        let consumer = root.join("consumer");
        std::fs::create_dir_all(provider.join("src")).unwrap();
        std::fs::create_dir_all(consumer.join("src")).unwrap();
        let (provider_name, lib_section, dependency) = if package_rename {
            (
                "renamed_provider",
                "",
                "std={package='renamed_provider',path='../provider',optional=true}",
            )
        } else {
            (
                "innocent_package",
                "[lib]\nname='std'\n",
                "innocent_package={path='../provider',optional=true}",
            )
        };
        std::fs::write(
            provider.join("Cargo.toml"),
            format!(
                "[package]\nname='{provider_name}'\nversion='0.0.0'\nedition='2024'\n{lib_section}[workspace]\n"
            ),
        )
        .unwrap();
        std::fs::write(
            provider.join("src/lib.rs"),
            "#![no_std]\npub mod default { pub trait Default {} }\n",
        )
        .unwrap();
        let feature_dep = if package_rename {
            "dep:std"
        } else {
            "dep:innocent_package"
        };
        std::fs::write(
            consumer.join("Cargo.toml"),
            format!(
                "[package]\nname='cargo_context_consumer'\nversion='0.0.0'\nedition='2024'\n[features]\ndefault=[]\nfake-std=['{feature_dep}']\nunrelated=[]\n[dependencies]\n{dependency}\n[workspace]\n"
            ),
        )
        .unwrap();
        std::fs::write(
            consumer.join("src/lib.rs"),
            r#"#![no_std]
extern crate std;
pub trait Decode { fn decode(&mut self); }
impl<T: std::default::Default> Decode for core::option::Option<T> {
    fn decode(&mut self) {}
}
"#,
        )
        .unwrap();
        consumer
    }

    fn cargo_check(manifest: &std::path::Path, target: &std::path::Path, flags: &[&str]) {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(manifest)
            .args(flags)
            .env("CARGO_TARGET_DIR", target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Cargo-invalid regression fixture for {flags:?}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn assert_hand_slot(cpp: &str, label: &str) {
        assert!(
            cpp.contains("constrained/const generic partial specializations are unsupported"),
            "{label}: missing hand slot:\n{cpp}"
        );
        for forbidden in [
            "class DecodeAdapter<rusty::Option<T>>",
            "class DecodeAdapterRef<rusty::Option<T>>",
            "class DecodeAdapterRefMut<rusty::Option<T>>",
        ] {
            assert!(!cpp.contains(forbidden), "{label}: emitted {forbidden}:\n{cpp}");
        }
    }

    fn assert_adapter(cpp: &str, label: &str) {
        for expected in [
            "class DecodeAdapter<rusty::Option<T>>",
            "class DecodeAdapterRef<rusty::Option<T>>",
            "class DecodeAdapterRefMut<rusty::Option<T>>",
        ] {
            assert!(cpp.contains(expected), "{label}: missing {expected}:\n{cpp}");
        }
    }

    fn direct(
        consumer: &std::path::Path,
        output_name: &str,
        flags: &[&str],
    ) -> String {
        let output_path = consumer.join(output_name);
        let output = transpiler_bin()
            .arg(consumer.join("src/lib.rs"))
            .args(flags)
            .arg("--module-name")
            .arg("cargo_context_consumer")
            .arg("--output")
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "direct lane failed for {flags:?}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(output_path).unwrap()
    }

    fn direct_expanded(
        consumer: &std::path::Path,
        output_name: &str,
        flags: &[&str],
    ) -> String {
        let output_path = consumer.join(output_name);
        let output = transpiler_bin()
            .arg(consumer.join("src/lib.rs"))
            .arg("--expand")
            .args(flags)
            .arg("--module-name")
            .arg("cargo_context_consumer")
            .arg("--output")
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "expanded direct lane failed for {flags:?}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(output_path).unwrap()
    }

    fn parity(
        consumer: &std::path::Path,
        work_name: &str,
        flags: &[&str],
    ) -> String {
        let work_dir = consumer.join(work_name);
        let output = transpiler_bin()
            .arg("parity-test")
            .arg("--manifest-path")
            .arg(consumer.join("Cargo.toml"))
            .arg("--work-dir")
            .arg(&work_dir)
            .arg("--keep-work-dir")
            .args(flags)
            .arg("--stop-after")
            .arg("transpile")
            .arg("--allow-empty-tests")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "parity lane failed for {flags:?}:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(
            work_dir
                .join("targets/cargo_context_consumer/cargo_context_consumer.cppm"),
        )
        .unwrap()
    }

    let fixture = tempfile::tempdir().unwrap();
    let lib_name_consumer = write_fixture(&fixture.path().join("lib_name"), false);
    let lib_manifest = lib_name_consumer.join("Cargo.toml");
    cargo_check(
        &lib_manifest,
        &fixture.path().join("cargo-target"),
        &["--features", "fake-std"],
    );
    cargo_check(
        &lib_manifest,
        &fixture.path().join("cargo-target"),
        &["--all-features"],
    );
    cargo_check(
        &lib_manifest,
        &fixture.path().join("cargo-target"),
        &["--no-default-features"],
    );

    assert_hand_slot(
        &direct(&lib_name_consumer, "direct-conservative.cppm", &[]),
        "direct conservative source context",
    );
    assert_hand_slot(
        &direct(
            &lib_name_consumer,
            "direct-feature.cppm",
            &["--features", "fake-std"],
        ),
        "direct explicit feature",
    );
    assert_hand_slot(
        &direct(
            &lib_name_consumer,
            "direct-all.cppm",
            &["--all-features"],
        ),
        "direct all-features",
    );
    assert_adapter(
        &direct(
            &lib_name_consumer,
            "direct-no-default.cppm",
            &["--no-default-features"],
        ),
        "direct no-default inactive optional",
    );
    assert_adapter(
        &direct(
            &lib_name_consumer,
            "direct-unrelated.cppm",
            &["--features", "unrelated"],
        ),
        "direct explicit unrelated feature",
    );
    assert_hand_slot(
        &direct(
            &lib_name_consumer,
            "direct-target-config.cppm",
            &[
                "--features",
                "fake-std",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--config",
                "net.offline=true",
            ],
        ),
        "direct target/config context",
    );
    assert_adapter(
        &direct_expanded(&lib_name_consumer, "direct-expanded-default.cppm", &[]),
        "direct expanded default Cargo context",
    );
    assert_hand_slot(
        &direct_expanded(
            &lib_name_consumer,
            "direct-expanded-feature.cppm",
            &["--features", "fake-std"],
        ),
        "direct expanded explicit feature",
    );

    assert_hand_slot(
        &parity(
            &lib_name_consumer,
            "parity-feature",
            &["--features", "fake-std"],
        ),
        "parity explicit feature",
    );
    assert_hand_slot(
        &parity(
            &lib_name_consumer,
            "parity-all",
            &["--all-features"],
        ),
        "parity all-features",
    );
    assert_adapter(
        &parity(
            &lib_name_consumer,
            "parity-no-default",
            &["--no-default-features"],
        ),
        "parity no-default inactive optional",
    );
    assert_adapter(
        &parity(&lib_name_consumer, "parity-default", &[]),
        "parity default inactive optional",
    );
    assert_hand_slot(
        &parity(
            &lib_name_consumer,
            "parity-target-config",
            &[
                "--features",
                "fake-std",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--config",
                "net.offline=true",
            ],
        ),
        "parity target/config context",
    );

    let reusable_cppm = lib_name_consumer
        .join("parity-feature/targets/cargo_context_consumer/cargo_context_consumer.cppm");
    let reusable_before = std::fs::read(&reusable_cppm).unwrap();
    let same_context_reuse = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&lib_manifest)
        .arg("--work-dir")
        .arg(lib_name_consumer.join("parity-feature"))
        .arg("--incremental-transpile")
        .arg("--features")
        .arg("fake-std")
        .arg("--stop-after")
        .arg("transpile")
        .arg("--allow-empty-tests")
        .output()
        .unwrap();
    assert!(
        same_context_reuse.status.success(),
        "same-context reuse failed:\n{}",
        String::from_utf8_lossy(&same_context_reuse.stderr)
    );
    let mismatched_reuse = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&lib_manifest)
        .arg("--work-dir")
        .arg(lib_name_consumer.join("parity-feature"))
        .arg("--incremental-transpile")
        .arg("--no-default-features")
        .arg("--stop-after")
        .arg("transpile")
        .arg("--allow-empty-tests")
        .output()
        .unwrap();
    assert!(!mismatched_reuse.status.success());
    assert!(
        String::from_utf8_lossy(&mismatched_reuse.stderr)
            .contains("different Cargo resolution context"),
        "unexpected mismatch diagnostic:\n{}",
        String::from_utf8_lossy(&mismatched_reuse.stderr)
    );
    assert_eq!(
        std::fs::read(&reusable_cppm).unwrap(),
        reusable_before,
        "mismatched context modified the reusable artifact"
    );

    // The command-line context alone is insufficient evidence: the same
    // feature flags can resolve a different extern identity after a manifest
    // edit.  Reuse must compare the complete context-matched metadata graph
    // and fail before touching the previously authenticated artifact.
    std::fs::write(
        lib_name_consumer.join("../provider/Cargo.toml"),
        "[package]\nname='innocent_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='ordinary_provider'\n[workspace]\n",
    )
    .unwrap();
    let changed_graph_reuse = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(&lib_manifest)
        .arg("--work-dir")
        .arg(lib_name_consumer.join("parity-feature"))
        .arg("--incremental-transpile")
        .arg("--features")
        .arg("fake-std")
        .arg("--stop-after")
        .arg("transpile")
        .arg("--allow-empty-tests")
        .output()
        .unwrap();
    assert!(!changed_graph_reuse.status.success());
    assert!(
        String::from_utf8_lossy(&changed_graph_reuse.stderr)
            .contains("different Cargo resolution context"),
        "unexpected changed-graph diagnostic:\n{}",
        String::from_utf8_lossy(&changed_graph_reuse.stderr)
    );
    assert_eq!(
        std::fs::read(&reusable_cppm).unwrap(),
        reusable_before,
        "changed Cargo graph modified the reusable artifact"
    );

    let renamed_consumer = write_fixture(&fixture.path().join("package_rename"), true);
    let renamed_manifest = renamed_consumer.join("Cargo.toml");
    cargo_check(
        &renamed_manifest,
        &fixture.path().join("cargo-target-renamed"),
        &["--features", "fake-std"],
    );
    assert_hand_slot(
        &direct(
            &renamed_consumer,
            "direct-renamed.cppm",
            &["--features", "fake-std"],
        ),
        "direct package rename",
    );
    assert_hand_slot(
        &parity(
            &renamed_consumer,
            "parity-renamed",
            &["--features", "fake-std"],
        ),
        "parity package rename",
    );
}

#[test]
fn dev_dependency_lib_name_std_is_target_scoped_and_parity_authentication_is_atomic() {
    fn assert_hand_slot(cpp: &str, label: &str) {
        assert!(
            cpp.contains("constrained/const generic partial specializations are unsupported"),
            "{label}: missing constrained-generic hand slot:\n{cpp}"
        );
        for forbidden in [
            "class DecodeAdapter<rusty::Option<T>>",
            "class DecodeAdapterRef<rusty::Option<T>>",
            "class DecodeAdapterRefMut<rusty::Option<T>>",
        ] {
            assert!(!cpp.contains(forbidden), "{label}: emitted {forbidden}:\n{cpp}");
        }
    }

    fn assert_adapter(cpp: &str, label: &str) {
        for expected in [
            "class DecodeAdapter<rusty::Option<T>>",
            "class DecodeAdapterRef<rusty::Option<T>>",
            "class DecodeAdapterRefMut<rusty::Option<T>>",
        ] {
            assert!(cpp.contains(expected), "{label}: missing {expected}:\n{cpp}");
        }
    }

    let fixture = tempfile::tempdir().unwrap();
    let provider = fixture.path().join("provider");
    let build_provider = fixture.path().join("build-provider");
    let consumer = fixture.path().join("consumer");
    std::fs::create_dir_all(provider.join("src")).unwrap();
    std::fs::create_dir_all(build_provider.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        provider.join("Cargo.toml"),
        "[package]\nname='innocent_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='std'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        provider.join("src/lib.rs"),
        "#![no_std]\npub mod default { pub trait Default {} }\n",
    )
    .unwrap();
    std::fs::write(
        build_provider.join("Cargo.toml"),
        "[package]\nname='build_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='core'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        build_provider.join("src/lib.rs"),
        "#![no_std]\nextern crate core as real_core;\npub use real_core::*;\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("Cargo.toml"),
        "[package]\nname='dev_context_consumer'\nversion='0.0.0'\nedition='2024'\n\
         [lib]\ntest=false\ndoctest=false\n\
         [dev-dependencies]\ninnocent_package={path='../provider'}\n\
         [build-dependencies]\nbuild_package={path='../build-provider'}\n\
         [[test]]\nname='fake_std_test'\npath='tests/fake_std.rs'\nharness=false\n\
         [workspace]\n",
    )
    .unwrap();
    let generic_impl = r#"
pub trait Decode { fn decode(&mut self); }
impl<T: std::default::Default> Decode for core::option::Option<T> {
    fn decode(&mut self) {}
}
"#;
    std::fs::write(consumer.join("src/lib.rs"), generic_impl).unwrap();
    let test_source = format!(
        r#"#![no_std]
#![no_main]
extern crate std;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {{ loop {{}} }}
#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {{ 0 }}
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {{
    unsafe {{ core::arch::asm!("mov rax, 60", "xor rdi, rdi", "syscall", options(noreturn)); }}
}}
{generic_impl}
"#
    );
    let test_path = consumer.join("tests/fake_std.rs");
    std::fs::write(&test_path, &test_source).unwrap();
    let build_source = r#"extern crate core;
pub trait Decode { fn decode(&mut self); }
impl<T: core::default::Default> Decode for core::option::Option<T> {
    fn decode(&mut self) {}
}
fn main() {}
"#;
    let build_path = consumer.join("build.rs");
    std::fs::write(&build_path, build_source).unwrap();

    let cargo_check = Command::new("cargo")
        .arg("check")
        .arg("--tests")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
        .env("RUSTFLAGS", "-C panic=abort")
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "dev-dependency lib-name fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let direct_lib = consumer.join("direct-lib.cppm");
    let direct_lib_output = transpiler_bin()
        .arg(consumer.join("src/lib.rs"))
        .arg("--module-name")
        .arg("dev_context_consumer")
        .arg("--output")
        .arg(&direct_lib)
        .output()
        .unwrap();
    assert!(
        direct_lib_output.status.success(),
        "direct normal-target lane failed:\n{}",
        String::from_utf8_lossy(&direct_lib_output.stderr)
    );
    assert_adapter(
        &std::fs::read_to_string(&direct_lib).unwrap(),
        "direct library target must not inherit dev dependencies",
    );

    let direct_test = consumer.join("direct-test.cppm");
    let direct_test_output = transpiler_bin()
        .arg(&test_path)
        .arg("--module-name")
        .arg("fake_std_test")
        .arg("--output")
        .arg(&direct_test)
        .output()
        .unwrap();
    assert!(
        direct_test_output.status.success(),
        "direct test-target lane failed:\n{}",
        String::from_utf8_lossy(&direct_test_output.stderr)
    );
    assert_hand_slot(
        &std::fs::read_to_string(&direct_test).unwrap(),
        "direct test target must include dev dependency extern names",
    );

    let direct_build = consumer.join("direct-build.cppm");
    let direct_build_output = transpiler_bin()
        .arg(&build_path)
        .arg("--module-name")
        .arg("build_script")
        .arg("--output")
        .arg(&direct_build)
        .output()
        .unwrap();
    assert!(
        direct_build_output.status.success(),
        "direct build-target lane failed:\n{}",
        String::from_utf8_lossy(&direct_build_output.stderr)
    );
    assert_hand_slot(
        &std::fs::read_to_string(&direct_build).unwrap(),
        "direct build target must use build-only dependency provenance",
    );

    let expanded_build = consumer.join("expanded-build.cppm");
    let expanded_build_output = transpiler_bin()
        .arg(&build_path)
        .arg("--expand")
        .arg("--module-name")
        .arg("build_script")
        .arg("--output")
        .arg(&expanded_build)
        .output()
        .unwrap();
    assert!(!expanded_build_output.status.success());
    assert!(
        String::from_utf8_lossy(&expanded_build_output.stderr)
            .contains("has no faithful target selector"),
        "build-script expansion selected another target:\n{}",
        String::from_utf8_lossy(&expanded_build_output.stderr)
    );
    assert!(
        !expanded_build.exists(),
        "build-script expansion failure created {}",
        expanded_build.display()
    );

    let direct_expanded_test = consumer.join("direct-expanded-test.cppm");
    let direct_expanded_output = transpiler_bin()
        .arg(&test_path)
        .arg("--expand")
        .arg("--module-name")
        .arg("fake_std_test")
        .arg("--output")
        .arg(&direct_expanded_test)
        .output()
        .unwrap();
    assert!(
        direct_expanded_output.status.success(),
        "direct expanded test-target lane failed:\n{}",
        String::from_utf8_lossy(&direct_expanded_output.stderr)
    );
    assert_hand_slot(
        &std::fs::read_to_string(&direct_expanded_test).unwrap(),
        "direct expanded test target must preserve target/dev provenance",
    );

    let nested_source = consumer.join("src/nested.rs");
    std::fs::write(&nested_source, generic_impl).unwrap();
    let nested_output_path = consumer.join("nested-expanded.cppm");
    let nested_output = transpiler_bin()
        .arg(&nested_source)
        .arg("--expand")
        .arg("--module-name")
        .arg("nested")
        .arg("--output")
        .arg(&nested_output_path)
        .output()
        .unwrap();
    assert!(!nested_output.status.success());
    assert!(
        String::from_utf8_lossy(&nested_output.stderr)
            .contains("not one exact Cargo target root"),
        "unknown-source expansion did not fail closed:\n{}",
        String::from_utf8_lossy(&nested_output.stderr)
    );
    assert!(
        !nested_output_path.exists(),
        "unknown-source expansion created {}",
        nested_output_path.display()
    );

    let work_dir = consumer.join("parity");
    let parity = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .arg("--work-dir")
        .arg(&work_dir)
        .arg("--no-baseline")
        .arg("--stop-after")
        .arg("transpile")
        .output()
        .unwrap();
    assert!(
        parity.status.success(),
        "parity dev-target lane failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&parity.stdout),
        String::from_utf8_lossy(&parity.stderr)
    );
    let parity_test_cppm = work_dir.join("targets/fake_std_test/fake_std_test.cppm");
    assert_hand_slot(
        &std::fs::read_to_string(&parity_test_cppm).unwrap(),
        "parity test target must include dev dependency extern names",
    );

    let preserved = std::fs::read(&parity_test_cppm).unwrap();
    std::fs::write(&test_path, "#![no_std]\nthis is not valid Rust\n").unwrap();
    let preexisting_failure = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .arg("--work-dir")
        .arg(&work_dir)
        .arg("--no-baseline")
        .arg("--stop-after")
        .arg("transpile")
        .output()
        .unwrap();
    assert!(!preexisting_failure.status.success());
    assert!(
        String::from_utf8_lossy(&preexisting_failure.stderr)
            .contains("while authenticating sysroot crates"),
        "unexpected target-provenance failure:\n{}",
        String::from_utf8_lossy(&preexisting_failure.stderr)
    );
    assert_eq!(
        std::fs::read(&parity_test_cppm).unwrap(),
        preserved,
        "failed pre-output authentication mutated an existing parity artifact"
    );

    let absent_work_dir = consumer.join("parity-absent");
    let absent_failure = transpiler_bin()
        .arg("parity-test")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .arg("--work-dir")
        .arg(&absent_work_dir)
        .arg("--no-baseline")
        .arg("--stop-after")
        .arg("transpile")
        .output()
        .unwrap();
    assert!(!absent_failure.status.success());
    assert!(
        !absent_work_dir.exists(),
        "failed pre-output authentication created {}",
        absent_work_dir.display()
    );
}
