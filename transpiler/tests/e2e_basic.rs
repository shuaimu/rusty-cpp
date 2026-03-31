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

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

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
    assert!(expected_output.exists(), "Expected hello.cppm to be created");
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
