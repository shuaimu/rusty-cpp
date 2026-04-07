use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_cpp_compiler() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["c++", "g++", "clang++"] {
        let status = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

fn compile_and_run_cpp(source: &str, test_name: &str) {
    let compiler = find_cpp_compiler().expect("no C++ compiler found in PATH or CXX");
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join(format!("{test_name}.cpp"));
    let bin_path = temp.path().join(format!("{test_name}.bin"));

    std::fs::write(&source_path, source).expect("write C++ source");

    let include_dir = project_include_dir();
    let compile = Command::new(&compiler)
        .arg("-std=c++20")
        .arg("-I")
        .arg(&include_dir)
        .arg(&source_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke C++ compiler");

    assert!(
        compile.status.success(),
        "C++ compile failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "C++ binary failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn test_ptr_read_const_pointer_supports_move_only_payloads() {
    let source = r#"
        #include <rusty/ptr.hpp>
        #include <utility>

        struct MoveOnly {
            int value;
            explicit MoveOnly(int v) : value(v) {}
            MoveOnly(const MoveOnly&) = delete;
            MoveOnly& operator=(const MoveOnly&) = delete;
            MoveOnly(MoveOnly&&) noexcept = default;
            MoveOnly& operator=(MoveOnly&&) noexcept = default;
        };

        int main() {
            MoveOnly payload(7);
            const MoveOnly* ptr = &payload;
            auto moved = rusty::ptr::read(ptr);
            return moved.value == 7 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "ptr_read_move_only");
}

#[test]
fn test_mem_replace_supports_non_assignable_move_only_payloads() {
    let source = r#"
        #include <rusty/mem.hpp>

        struct NonAssignable {
            int value;
            explicit NonAssignable(int v) : value(v) {}
            NonAssignable(const NonAssignable&) = delete;
            NonAssignable& operator=(const NonAssignable&) = delete;
            NonAssignable(NonAssignable&&) noexcept = default;
            NonAssignable& operator=(NonAssignable&&) = delete;
        };

        int main() {
            NonAssignable dst(1);
            auto old = rusty::mem::replace(dst, NonAssignable(2));
            return (old.value == 1 && dst.value == 2) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_replace_non_assignable");
}
