use std::path::Path;
use std::process::Command;

#[test]
fn explicit_nullable_owner_aliases_preserve_presence_ownership_and_plain_options() {
    let directory = tempfile::tempdir().unwrap();
    let rust_path = directory.path().join("nullable_owner.rs");
    let cpp_path = directory.path().join("nullable_owner.cpp");
    let map_path = directory.path().join("types.toml");
    std::fs::write(&rust_path, SOURCE).unwrap();
    std::fs::write(&map_path, "MaybeArc = \"rusty::Arc\"\nMaybeBox = \"rusty::Box\"\nMaybeOwned = \"OwnedThing\"\n").unwrap();
    let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&rust_path).arg("--type-map").arg(&map_path)
        .arg("-o").arg(&cpp_path).output().unwrap();
    assert!(generated.status.success(), "{}", String::from_utf8_lossy(&generated.stderr));
    let mut cpp = std::fs::read_to_string(&cpp_path).unwrap();
    assert!(cpp.contains("using MaybeArc = rusty::Arc<T>;"), "{cpp}");
    assert!(cpp.contains("using MaybeBox = rusty::Box<T>;"), "{cpp}");
    assert!(cpp.contains("rusty::Option<rusty::Box<int32_t>> ordinary"), "{cpp}");
    cpp.push_str("\nint main() { return check() + inspect_box(rusty::Box<int32_t>(nullptr)); }\n");
    std::fs::write(&cpp_path, cpp).unwrap();
    let cpp_binary = directory.path().join("cpp_check");
    let compiled = Command::new(std::env::var("CXX").unwrap_or_else(|_| "clang++".into()))
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-pthread", "-I"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&cpp_path).arg("-o").arg(&cpp_binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let ran = Command::new(cpp_binary).output().unwrap();
    assert!(ran.status.success(), "{}\n{}", ran.status, String::from_utf8_lossy(&ran.stderr));
    std::fs::write(&rust_path, format!("{SOURCE}\nfn main() {{ assert_eq!(check(), 0); assert_eq!(inspect_box(None), 0); }}\n")).unwrap();
    let rust_binary = directory.path().join("rust_check");
    let compiled = Command::new("rustc").arg("--edition=2024")
        .arg(&rust_path).arg("-o").arg(&rust_binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let ran = Command::new(rust_binary).output().unwrap();
    assert!(ran.status.success(), "{}\n{}", ran.status, String::from_utf8_lossy(&ran.stderr));
}

const SOURCE: &str = r#"
use std::sync::Arc;
pub type MaybeArc<T> = Option<Arc<T>>;
pub type MaybeBox<T> = Option<Box<T>>;
pub type OwnedThing = Box<i32>;
pub type MaybeOwned = Option<OwnedThing>;

pub fn inspect_box(value: MaybeBox<i32>) -> i32 {
    if let Some(value) = value { *value } else { 0 }
}

pub fn check() -> i32 {
    let mut slot: MaybeBox<i32> = None;
    if slot.is_some() { return 1; }
    slot = Some(Box::new(41));
    if let Some(value) = &mut slot { **value += 1; }
    {
        let borrowed = slot.as_ref();
        if **borrowed.unwrap() != 42 { return 2; }
    }
    let previous = slot.replace(Box::new(9));
    if inspect_box(previous) != 42 { return 3; }
    let taken = slot.take();
    if slot.is_some() { return 4; }
    if *taken.unwrap() != 9 { return 5; }

    let source = Arc::new(7);
    let owned: MaybeArc<i32> = Some(source.clone());
    let duplicate = owned.clone();
    if Arc::strong_count(&source) != 3 { return 6; }
    drop(duplicate);
    if Arc::strong_count(&source) != 2 { return 7; }
    if let Some(borrowed) = &owned {
        if **borrowed != 7 { return 8; }
    } else { return 9; }
    drop(owned);
    if Arc::strong_count(&source) != 1 { return 10; }

    let ordinary: Option<Box<i32>> = Some(Box::new(11));
    if *ordinary.unwrap() != 11 { return 12; }
    let empty: MaybeBox<i32> = None;
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| empty.unwrap()));
    if failed.is_ok() { return 13; }
    let absent: MaybeArc<i32> = Default::default();
    if absent.clone().is_some() { return 14; }
    let alias: MaybeOwned = Some(Box::new(15));
    if *alias.unwrap() != 15 { return 15; }
    let callback: Box<dyn Fn(MaybeBox<i32>) -> i32> = Box::new(|value: MaybeBox<i32>| inspect_box(value));
    if callback(Some(Box::new(16))) != 16 { return 16; }
    let absent_box: MaybeBox<i32> = None;
    if absent_box.clone().is_some() { return 17; }
    let present_box: MaybeBox<i32> = Some(Box::new(18));
    let mut separate = present_box.clone();
    if let Some(value) = separate.as_mut() { **value += 1; }
    if *present_box.unwrap() != 18 || *separate.unwrap() != 19 { return 18; }
    0
}
"#;
