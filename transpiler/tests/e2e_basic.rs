use std::process::Command;

fn transpiler_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
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
    assert!(cpp.contains("return a + b;"));
    assert!(cpp.contains("struct Point {"));
    assert!(cpp.contains("double x;"));
    assert!(cpp.contains("constexpr int32_t MAX = 100;"));
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
