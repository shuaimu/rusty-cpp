#![cfg(all(target_os = "linux", target_env = "gnu"))]

use std::path::Path;
use std::process::Command;

fn check(posix: bool, sanitize: bool) {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("tls_teardown");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".into());
    let mut compile = Command::new(compiler);
    compile
        .args(["-std=c++23", "-pthread", "-g", "-O1", "-I"])
        .arg(root.join("include"))
        .arg(root.join("tests/test_thread_tls_teardown.cpp"))
        .arg("-o")
        .arg(&binary);
    if posix {
        compile.arg("-DRUSTY_PLATFORM_BACKEND_POSIX=1");
    }
    if sanitize {
        compile.args(["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]);
    }
    let compiled = compile.output().unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(binary)
        .env(
            "ASAN_OPTIONS",
            "detect_leaks=1:detect_stack_use_after_return=1",
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1")
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}\n{}",
        ran.status,
        String::from_utf8_lossy(&ran.stderr)
    );
}

#[test]
fn current_and_park_survive_tls_teardown_on_both_backends() {
    check(false, false);
    check(true, false);
}

#[test]
fn current_and_park_tls_owners_are_freed_under_sanitizers() {
    check(false, true);
    check(true, true);
}
