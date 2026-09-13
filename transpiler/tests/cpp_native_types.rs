use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../include")
}
fn compiler() -> String {
    std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string())
}
fn checked(command: &mut Command) {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
fn emit(root: &Path, source: &str, mapping: &str, module: bool) -> (Output, PathBuf) {
    let input = root.join("source.rs");
    let map = root.join("map.toml");
    let output = root.join(if module { "native.cppm" } else { "native.cpp" });
    std::fs::write(&input, source).unwrap();
    std::fs::write(&map, mapping).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
    command
        .arg(input)
        .arg("--type-map")
        .arg(map)
        .arg("-o")
        .arg(&output);
    if module {
        let preamble = root.join("preamble.toml");
        std::fs::write(&preamble, "version=1\n[[module]]\nname='native_probe'\nincludes=[{path='stdio.h',form='angle'}]\n").unwrap();
        command
            .args(["-m", "native_probe", "--module-preamble"])
            .arg(preamble);
    }
    (command.output().unwrap(), output)
}

#[test]
fn native_c_fields_and_opaque_pthread_pointers_use_header_types() {
    let root = tempfile::tempdir().unwrap();
    let (result, output) = emit(
        root.path(),
        r#"
#[repr(C)] #[cfg_attr(any(), cpp_native_type)]
pub struct Native { pub value: i32 }
#[repr(C)] #[cfg_attr(any(), cpp_native_type)]
pub struct Mutex { _opaque: [u8; 0] }
pub fn read(value: &Native) -> i32 { read_later(value) }
fn read_later(value: &Native) -> i32 { value.value }
pub fn valid(mutex: *mut Mutex) -> bool { !mutex.is_null() }
"#,
        "Native='::native_t'\nMutex='::pthread_mutex_t'\n",
        false,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let cpp = std::fs::read_to_string(&output).unwrap();
    assert!(!cpp.contains("struct Native"), "{cpp}");
    assert!(!cpp.contains("struct Mutex"), "{cpp}");
    assert!(cpp.contains("const ::native_t&"), "{cpp}");
    assert!(cpp.contains("pthread_mutex_t*"), "{cpp}");
    std::fs::write(&output, format!("#include <pthread.h>\nstruct native_t {{ int value; }};\n{cpp}\nint main() {{ native_t value{{37}}; pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER; return read(value) == 37 && valid(&mutex) && !valid(nullptr) ? 0 : 1; }}")).unwrap();
    let binary = root.path().join("native");
    checked(
        Command::new(compiler())
            .args([
                "-std=c++23",
                "-DRUSTY_PORTABLE_INTRINSICS=1",
                "-pthread",
                "-I",
            ])
            .arg(include_dir())
            .arg(output)
            .arg("-o")
            .arg(&binary),
    );
    checked(&mut Command::new(binary));
}

#[test]
fn native_file_binding_authenticates_stderr_default_in_a_named_module() {
    let root = tempfile::tempdir().unwrap();
    let (result, output) = emit(
        root.path(),
        r#"
#[repr(C)] #[cfg_attr(any(), cpp_native_type)]
pub struct CFile { _opaque: [u8; 0] }
pub fn absent(#[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut CFile) -> bool { stream.is_null() }
"#,
        "CFile='FILE'\n",
        true,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let cpp = std::fs::read_to_string(&output).unwrap();
    assert!(cpp.contains("FILE* stream = stderr"), "{cpp}");
    assert!(!cpp.contains("struct CFile"), "{cpp}");
    let pcm = root.path().join("native.pcm");
    let object = root.path().join("native.o");
    let consumer = root.path().join("main.cpp");
    let binary = root.path().join("native");
    std::fs::write(
        &consumer,
        "import native_probe;\nint main() { return !absent() && absent(nullptr) ? 0 : 1; }\n",
    )
    .unwrap();
    checked(
        Command::new(compiler())
            .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-I"])
            .arg(include_dir())
            .arg("--precompile")
            .arg(output)
            .arg("-o")
            .arg(&pcm),
    );
    checked(
        Command::new(compiler())
            .args(["-std=c++23", "-c"])
            .arg(&pcm)
            .arg("-o")
            .arg(&object),
    );
    checked(
        Command::new(compiler())
            .arg("-std=c++23")
            .arg(format!("-fmodule-file=native_probe={}", pcm.display()))
            .arg(consumer)
            .arg(object)
            .arg("-o")
            .arg(&binary),
    );
    checked(&mut Command::new(binary));
}

#[test]
fn file_name_or_arbitrary_native_mapping_cannot_authenticate_stderr() {
    for (attr, map) in [
        ("", "CFile='FILE'"),
        (
            "#[cfg_attr(any(), cpp_native_type)]",
            "CFile='pthread_mutex_t'",
        ),
        ("#[cfg_attr(any(), cpp_native_type)]", ""),
    ] {
        let root = tempfile::tempdir().unwrap();
        let source = format!(
            "#[repr(C)] {attr} pub struct CFile {{ _opaque: [u8;0] }} pub fn absent(#[cfg_attr(any(), cpp_default_argument(stderr))] stream: *mut CFile) -> bool {{ stream.is_null() }}"
        );
        let (result, _) = emit(root.path(), &source, map, true);
        assert!(!result.status.success(), "{source}\n{map}");
    }
}
