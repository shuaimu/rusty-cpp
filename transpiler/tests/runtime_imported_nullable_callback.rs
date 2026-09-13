use std::path::Path;
use std::process::Command;

#[test]
fn imported_callback_construction_borrow_and_dispatch_run_in_rust_and_cpp() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"imported_callback\"\nversion = \"0.1.0\"\nedition = \"2024\"\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub mod provider;\npub mod consumer;\n").unwrap();
    std::fs::write(root.join("src/provider.rs"), r#"
use std::ops::Fn as Predicate;
pub type EventTestFn = Option<Box<dyn Predicate(i32) -> bool + Send + Sync>>;
"#).unwrap();
    std::fs::write(root.join("src/consumer.rs"), r#"
use crate::provider::EventTestFn;
pub fn absent() -> EventTestFn { None }
pub fn check() -> bool {
    let mut predicate: EventTestFn = Some(Box::new(|value: i32| value == 7));
    if predicate.is_none() || absent().is_some() { return false; }
    if !predicate.as_ref().unwrap()(7) || predicate.as_ref().unwrap()(8) { return false; }
    let taken = predicate.take();
    if predicate.is_some() { return false; }
    taken.unwrap()(7)
}
"#).unwrap();
    let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate").arg(root.join("Cargo.toml"))
        .arg("--output-dir").arg(root.join("out"))
        .args(["--cxx-namespace", "probe", "--flat-import-namespace", "probe"])
        .output().unwrap();
    assert!(generated.status.success(), "{}\n{}", String::from_utf8_lossy(&generated.stdout), String::from_utf8_lossy(&generated.stderr));
    let mut cpp = String::new();
    for module in ["provider", "consumer"] {
        let module_cpp = std::fs::read_to_string(root.join(format!("out/imported_callback.{module}.cppm"))).unwrap();
        for line in module_cpp.lines() {
            if line == "module;" || line.starts_with("export module ") || line.starts_with("import ") { continue; }
            cpp.push_str(line.strip_prefix("export ").unwrap_or(line));
            cpp.push('\n');
        }
    }
    cpp.push_str("\nint main() { return probe::check() ? 0 : 1; }\n");
    let cpp_path = root.join("check.cpp");
    std::fs::write(&cpp_path, cpp).unwrap();
    let binary = root.join("check_cpp");
    let compiled = Command::new(std::env::var("CXX").unwrap_or_else(|_| "clang++".into()))
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-pthread", "-I"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(cpp_path).arg("-o").arg(&binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let ran = Command::new(binary).output().unwrap();
    assert!(ran.status.success(), "{}\n{}", ran.status, String::from_utf8_lossy(&ran.stderr));
    std::fs::write(root.join("src/lib.rs"), "pub mod provider;\npub mod consumer;\nfn main() { assert!(consumer::check()); }\n").unwrap();
    let binary = root.join("check_rust");
    let compiled = Command::new("rustc").arg("--edition=2024").arg(root.join("src/lib.rs"))
        .arg("-o").arg(&binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let ran = Command::new(binary).output().unwrap();
    assert!(ran.status.success(), "{}", String::from_utf8_lossy(&ran.stderr));
}
