use std::path::{Path, PathBuf};
use std::process::Command;

fn run(command: &mut Command, label: &str) {
    let result = command
        .output()
        .unwrap_or_else(|error| panic!("{label}: {error}"));
    assert!(
        result.status.success(),
        "{label} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
}

#[test]
fn nullable_callback_receivers_retain_their_source_types_and_run_in_both_languages() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source = include_str!("fixtures/nullable_callback_receivers.rs");
    let source_path = temp.path().join("nullable_callback_receivers.rs");
    let cpp_path = temp.path().join("nullable_callback_receivers.cpp");
    let rust_bin = temp.path().join("rust_probe");
    let cpp_bin = temp.path().join("cpp_probe");
    std::fs::write(&source_path, source).expect("write Rust source");
    run(
        Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
            .arg(&source_path)
            .arg("-o")
            .arg(&cpp_path),
        "transpile callback receivers",
    );

    let generated = std::fs::read_to_string(&cpp_path).expect("read generated C++");
    std::fs::write(
        &cpp_path,
        format!("{generated}\nint main() {{ return check_receiver_inference(); }}\n"),
    )
    .expect("write C++ driver");
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let include_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../include");
    run(
        Command::new(compiler)
            .arg("-std=c++23")
            .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
            .arg("-pthread")
            .arg("-I")
            .arg(include_dir)
            .arg(&cpp_path)
            .arg("-o")
            .arg(&cpp_bin),
        "compile generated callback receivers",
    );
    run(
        &mut Command::new(&cpp_bin),
        "run generated callback receivers",
    );

    std::fs::write(
        &source_path,
        format!("{source}\nfn main() {{ assert_eq!(check_receiver_inference(), 0); }}\n"),
    )
    .expect("write Rust driver");
    run(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source_path)
            .arg("-o")
            .arg(&rust_bin),
        "compile native callback receivers",
    );
    run(
        &mut Command::new(&rust_bin),
        "run native callback receivers",
    );
}
