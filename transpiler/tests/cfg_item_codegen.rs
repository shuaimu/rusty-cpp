use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
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
fn cfg_const_and_static_variants_compile_as_module_for_linux_and_simulated_apple() {
    let Some(clang) = find_clang() else {
        eprintln!("skipping cfg item compile gate: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let rust = temp.path().join("cfg_items.rs");
    std::fs::write(
        &rust,
        r#"
        const fn mac_value() -> i32 { 35 }
        const fn other_value() -> i32 { 11 }

        #[cfg(target_os = "macos")]
        pub const CODE: i32 = 60;
        #[cfg(not(target_os = "macos"))]
        pub const CODE: i32 = 110;

        #[cfg(target_os = "macos")]
        pub const COMPUTED: i32 = mac_value();
        #[cfg(not(target_os = "macos"))]
        pub const COMPUTED: i32 = other_value();

        #[cfg(target_os = "macos")]
        pub static SLOT: i32 = 1;
        #[cfg(not(target_os = "macos"))]
        pub static SLOT: i32 = 2;
        "#,
    )
    .expect("write Rust fixture");
    let cpp = temp.path().join("cfg_items.cppm");
    let transpiler = env!("CARGO_BIN_EXE_rusty-cpp-transpiler");
    let transpile = Command::new(transpiler)
        .arg(&rust)
        .arg("-o")
        .arg(&cpp)
        .arg("-m")
        .arg("cfg_items")
        .output()
        .expect("run transpiler");
    assert!(
        transpile.status.success(),
        "transpilation failed:\n{}\n{}",
        String::from_utf8_lossy(&transpile.stdout),
        String::from_utf8_lossy(&transpile.stderr)
    );

    let generated = std::fs::read_to_string(&cpp).expect("read generated C++ module");
    assert!(
        generated.contains("export module cfg_items;"),
        "fixture must exercise named-module output:\n{generated}"
    );
    assert_eq!(
        generated.matches("export constexpr int32_t CODE").count(),
        2,
        "both target variants must remain exported:\n{generated}"
    );
    assert_eq!(
        generated
            .matches("export extern const int32_t COMPUTED;")
            .count(),
        2,
        "both target declarations must remain exported:\n{generated}"
    );
    assert_eq!(
        generated.matches("export extern int32_t SLOT;").count(),
        2,
        "both target statics must remain exported:\n{generated}"
    );

    let importer = temp.path().join("importer.cpp");
    std::fs::write(
        &importer,
        r#"
import cfg_items;
#if defined(__APPLE__)
static_assert(CODE == 60 && COMPUTED == 35);
#else
static_assert(CODE == 110 && COMPUTED == 11);
#endif
int main() {
#if defined(__APPLE__)
    return SLOT == 1 ? 0 : 1;
#else
    return SLOT == 2 ? 0 : 2;
#endif
}
"#,
    )
    .expect("write module importer");

    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include");
    for (label, target_args) in [
        ("linux", Vec::<&str>::new()),
        (
            "macos-simulated",
            vec!["-U__linux__", "-U__linux", "-D__APPLE__=1", "-D__MACH__=1"],
        ),
    ] {
        let pcm = temp.path().join(format!("cfg_items_{label}.pcm"));
        let module_object = temp.path().join(format!("cfg_items_{label}.o"));
        let importer_object = temp.path().join(format!("importer_{label}.o"));
        let binary = temp.path().join(format!("cfg_items_{label}"));

        let mut precompile = Command::new(&clang);
        precompile
            .arg("-std=c++23")
            .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
            .arg("-I")
            .arg(&include);
        precompile
            .args(&target_args)
            .arg("-x")
            .arg("c++-module")
            .arg("--precompile")
            .arg(&cpp)
            .arg("-o")
            .arg(&pcm);
        let compile = precompile.output().expect("precompile generated module");
        assert!(
            compile.status.success(),
            "{label} cfg module failed to precompile:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );

        for (label_suffix, source, output, language) in [
            ("module", &cpp, &module_object, "c++-module"),
            ("importer", &importer, &importer_object, "c++"),
        ] {
            let mut command = Command::new(&clang);
            command
                .arg("-std=c++23")
                .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
                .arg("-I")
                .arg(&include)
                .args(&target_args)
                .arg("-x")
                .arg(language)
                .arg("-c")
                .arg(source)
                .arg(format!("-fmodule-file=cfg_items={}", pcm.display()))
                .arg("-o")
                .arg(output);
            let compile = command.output().expect("compile cfg module lane");
            assert!(
                compile.status.success(),
                "{label} {label_suffix} object failed to compile:\n{}",
                String::from_utf8_lossy(&compile.stderr)
            );
        }

        let link = Command::new(&clang)
            .arg(&module_object)
            .arg(&importer_object)
            .arg("-o")
            .arg(&binary)
            .output()
            .expect("link cfg module lane");
        assert!(
            link.status.success(),
            "{label} cfg module lane failed to link:\n{}",
            String::from_utf8_lossy(&link.stderr)
        );
        let run = Command::new(&binary).output().expect("run cfg item binary");
        assert!(run.status.success(), "{label} cfg item runtime failed");
    }
}

#[test]
fn unsupported_cfg_const_fails_before_writing_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let rust = temp.path().join("unsupported.rs");
    std::fs::write(
        &rust,
        r#"
#[cfg_attr(
    target_os = "linux",
    cfg_attr(target_arch = "x86_64", cfg(target_arch = "aarch64"))
)]
pub const HIDDEN: i32 = 1;
"#,
    )
    .expect("write unsupported cfg fixture");
    let cpp = temp.path().join("unsupported.cppm");
    let transpile = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust)
        .arg("-o")
        .arg(&cpp)
        .arg("-m")
        .arg("unsupported")
        .output()
        .expect("run transpiler");
    assert!(
        !transpile.status.success(),
        "presence-changing cfg_attr must fail closed"
    );
    assert!(
        String::from_utf8_lossy(&transpile.stderr)
            .contains("cannot preserve unsupported #[cfg] predicate"),
        "unexpected diagnostic:\n{}",
        String::from_utf8_lossy(&transpile.stderr)
    );
    assert!(
        !cpp.exists(),
        "failed translation must not leave a partial output"
    );
}
