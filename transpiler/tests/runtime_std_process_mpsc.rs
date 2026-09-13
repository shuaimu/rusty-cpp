use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(compiler) = env::var("CXX") {
        if !compiler.trim().is_empty() {
            return Some(compiler);
        }
    }
    ["clang++", "clang++-22", "clang++-21"]
        .into_iter()
        .find(|compiler| {
            Command::new(compiler)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        })
        .map(str::to_string)
}

fn transpile_compile_run(source: &str, cpp_main: &str, expected: &[&str]) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join("std_runtime_paths.rs");
    let cpp_path = temp.path().join("std_runtime_paths.cpp");
    let bin_path = temp.path().join("std_runtime_paths.bin");
    std::fs::write(&source_path, source).expect("write Rust source");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source_path)
        .arg("-o")
        .arg(&cpp_path)
        .output()
        .expect("invoke transpiler");
    assert!(
        transpile.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&transpile.stderr)
    );
    let mut cpp = std::fs::read_to_string(&cpp_path).expect("read generated C++");
    for spelling in expected {
        assert!(cpp.contains(spelling), "missing {spelling}:\n{cpp}");
    }
    let Some(compiler) = find_clang() else {
        eprintln!("skipping C++ runtime check: no clang++ found");
        return;
    };
    cpp.push_str(cpp_main);
    std::fs::write(&cpp_path, cpp).expect("write C++ runtime check");
    let include_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include");
    let compile = Command::new(compiler)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(include_dir)
        .arg(&cpp_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke clang++");
    assert!(
        compile.status.success(),
        "C++ compile failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&bin_path).output().expect("run C++ probe");
    assert!(
        run.status.success(),
        "C++ runtime check failed with {:?}:\n{}",
        run.status.code(),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn fully_qualified_std_mpsc_types_and_channel_translate_and_run() {
    transpile_compile_run(
        r#"
        pub struct Pipe {
            sender: std::sync::mpsc::Sender<i32>,
            receiver: std::sync::mpsc::Receiver<i32>,
        }
        pub fn pipe() -> Pipe {
            let (sender, receiver) = std::sync::mpsc::channel::<i32>();
            Pipe { sender, receiver }
        }
        pub fn exchange() -> i32 {
            let pipe = pipe();
            if pipe.receiver.try_recv().is_ok() { return -1; }
            pipe.sender.send(42).unwrap();
            pipe.receiver.recv().unwrap()
        }
        "#,
        "\nint main() { return exchange() == 42 ? 0 : 1; }\n",
        &[
            "rusty::sync::mpsc::Sender<int32_t> sender",
            "rusty::sync::mpsc::Receiver<int32_t> receiver",
            "rusty::sync::mpsc::channel<int32_t>()",
        ],
    );
}

#[test]
fn std_process_id_returns_the_unsigned_native_pid() {
    transpile_compile_run(
        r#"
        pub fn process_id() -> u32 { std::process::id() }
        "#,
        r#"
static_assert(std::is_same_v<decltype(rusty::process::id()), std::uint32_t>);
int main() {
#if defined(_WIN32)
    return process_id() == static_cast<std::uint32_t>(::_getpid()) ? 0 : 1;
#else
    return process_id() == static_cast<std::uint32_t>(::getpid()) ? 0 : 1;
#endif
}
"#,
        &["return rusty::process::id();"],
    );
}

#[test]
fn qualified_std_runtime_paths_preserve_local_modules() {
    transpile_compile_run(
        r#"
        mod std {
            pub mod process {
                pub fn id() -> i32 { 7 }
            }
            pub mod sync {
                pub mod mpsc {
                    pub struct Sender<T> { pub value: T }
                    pub struct Receiver<T> { pub value: T }
                    pub fn channel() -> i32 { 8 }
                }
            }
        }
        pub fn local_sum() -> i32 {
            let sender = std::sync::mpsc::Sender { value: 3 };
            let receiver = std::sync::mpsc::Receiver { value: 4 };
            sender.value + receiver.value + std::process::id() + std::sync::mpsc::channel()
        }
        "#,
        "\nint main() { return local_sum() == 22 ? 0 : 1; }\n",
        &["std_mod::process::id()", "std_mod::sync_mod::mpsc::channel()"],
    );
}
