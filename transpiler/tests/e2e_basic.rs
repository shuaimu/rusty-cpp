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
