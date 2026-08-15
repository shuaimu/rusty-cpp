use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

#[test]
fn inert_cpp_ctor_constructs_arc_payload_in_place() {
    let temp = tempfile::tempdir().expect("tempdir");
    let rust = temp.path().join("owner.rs");
    std::fs::write(
        &rust,
        r#"
use std::sync::Arc;

pub struct Owner {
    pub value: i32,
}

impl Owner {
    #[cfg_attr(any(), cpp_ctor)]
    pub unsafe fn new(value: i32) -> Owner {
        Owner { value }
    }
}

pub fn make_owner(value: i32) -> Arc<Owner> {
    Arc::new(unsafe { Owner::new(value) })
}
"#,
    )
    .expect("write Rust fixture");

    let rust_library = temp.path().join("libowner.rlib");
    let rustc = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib"])
        .arg(&rust)
        .arg("-o")
        .arg(&rust_library)
        .output()
        .expect("run rustc");
    assert!(
        rustc.status.success(),
        "cpp_ctor Arc fixture is not rustc-valid:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let cpp = temp.path().join("owner.cppm");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .args(["-o", cpp.to_str().unwrap()])
        .args(["-m", "cpp_ctor_arc_runtime", "--cxx-namespace", "fixture"])
        .output()
        .expect("run transpiler");
    assert!(
        transpile.status.success(),
        "transpilation failed:\n{}\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );

    let generated = std::fs::read_to_string(&cpp).expect("read generated module");
    assert!(generated.contains("Owner(int32_t value)"), "{generated}");
    assert!(
        generated.contains("rusty::Arc<Owner>::make(") && !generated.contains("Owner::new_("),
        "the lowerable cpp_ctor must be fused into Arc::make:\n{generated}"
    );

    let Some(clang) = find_clang() else {
        eprintln!("skipping cpp_ctor Arc runtime gate: no clang++ in PATH or CXX");
        return;
    };
    let importer = temp.path().join("importer.cpp");
    std::fs::write(
        &importer,
        r#"
import cpp_ctor_arc_runtime;
int main() {
    auto owner = fixture::make_owner(41);
    return owner->value == 41 ? 0 : 1;
}
"#,
    )
    .expect("write importer");

    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include");
    let pcm = temp.path().join("owner.pcm");
    let module_object = temp.path().join("owner.o");
    let importer_object = temp.path().join("importer.o");
    let binary = temp.path().join("cpp_ctor_arc_runtime");
    let common = ["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-w"];

    let precompile = Command::new(&clang)
        .args(common)
        .arg("-I")
        .arg(&include)
        .args(["-x", "c++-module", "--precompile"])
        .arg(&cpp)
        .arg("-o")
        .arg(&pcm)
        .output()
        .expect("precompile module");
    assert!(
        precompile.status.success(),
        "cpp_ctor Arc module failed to precompile:\n{}",
        String::from_utf8_lossy(&precompile.stderr)
    );

    for (label, source, output, language) in [
        ("module", &cpp, &module_object, "c++-module"),
        ("importer", &importer, &importer_object, "c++"),
    ] {
        let compile = Command::new(&clang)
            .args(common)
            .arg("-I")
            .arg(&include)
            .arg("-x")
            .arg(language)
            .arg("-c")
            .arg(source)
            .arg(format!(
                "-fmodule-file=cpp_ctor_arc_runtime={}",
                pcm.display()
            ))
            .arg("-o")
            .arg(output)
            .output()
            .expect("compile runtime lane");
        assert!(
            compile.status.success(),
            "{label} failed to compile:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
    }

    let link = Command::new(&clang)
        .args([&module_object, &importer_object])
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("link runtime");
    assert!(
        link.status.success(),
        "cpp_ctor Arc runtime failed to link:\n{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let run = Command::new(&binary).output().expect("run binary");
    assert!(run.status.success(), "runtime returned {run:?}");
}

#[test]
fn ordinary_dependency_named_std_cannot_authenticate_arc_fusion() {
    let temp = tempfile::tempdir().expect("tempdir");
    let provider = temp.path().join("fake_std");
    let consumer = temp.path().join("consumer");
    std::fs::create_dir_all(provider.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::write(
        provider.join("Cargo.toml"),
        "[package]\nname='innocent_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='std'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        provider.join("src/lib.rs"),
        "#![no_std]\npub mod sync { pub struct Arc<T>(pub T); impl<T> Arc<T> { pub fn new(value: T) -> Self { Self(value) } } }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("Cargo.toml"),
        "[package]\nname='fake_arc_consumer'\nversion='0.0.0'\nedition='2024'\n[dependencies]\ninnocent_package={path='../fake_std'}\n[workspace]\n",
    )
    .unwrap();
    let source = consumer.join("src/lib.rs");
    std::fs::write(
        &source,
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

    let cargo = Command::new("cargo")
        .arg("check")
        .arg("--manifest-path")
        .arg(consumer.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", temp.path().join("cargo-target"))
        .output()
        .unwrap();
    assert!(
        cargo.status.success(),
        "fake-std fixture is not Cargo-valid:\n{}",
        String::from_utf8_lossy(&cargo.stderr)
    );

    let cpp = consumer.join("fake_arc.cppm");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .args(["--module-name", "fake_arc", "--output"])
        .arg(&cpp)
        .output()
        .unwrap();
    assert!(
        transpile.status.success(),
        "fake-std transpilation failed:\n{}",
        String::from_utf8_lossy(&transpile.stderr)
    );
    let generated = std::fs::read_to_string(cpp).unwrap();
    assert!(
        !generated.contains("rusty::Arc<Owner>::make("),
        "an ordinary dependency occupying `std` authenticated Arc fusion:\n{generated}"
    );
}
