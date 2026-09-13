use std::path::Path;
use std::process::Command;

#[test]
fn explicit_nullable_owner_aliases_preserve_presence_ownership_and_plain_options() {
    let directory = tempfile::tempdir().unwrap();
    let rust_path = directory.path().join("nullable_owner.rs");
    let cpp_path = directory.path().join("nullable_owner.cpp");
    let map_path = directory.path().join("types.toml");
    std::fs::write(&rust_path, SOURCE).unwrap();
    std::fs::write(&map_path, "MaybeArc = \"rusty::Arc\"\nMaybeBox = \"rusty::Box\"\nMaybeOwned = \"OwnedThing\"\nMaybeCounter = \"rusty::Arc<rusty::sync::atomic::AtomicI32>\"\n").unwrap();
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
type Counter = std::sync::atomic::AtomicI32;
type MaybeCounter = Option<Arc<Counter>>;
struct Holder { value: i32 }
impl Holder {
    fn inspect(&self, counter: &MaybeCounter) -> i32 {
        if let Some(counter) = counter { counter.load(std::sync::atomic::Ordering::Relaxed) } else { self.value }
    }
    fn inspect_box(&self, value: MaybeOwned) -> i32 {
        if let Some(value) = value { *value } else { self.value }
    }
}
struct Callback<F> { function: F }
impl<F> Callback<F> { fn callable(&self) -> &F { &self.function } }


pub fn inspect_box(value: MaybeBox<i32>) -> i32 {
    if let Some(value) = value { *value } else { 0 }
}

fn mutate_borrowed(value: &mut MaybeBox<i32>) {
    if let Some(payload) = value { **payload += 1; }
}
fn inspect_borrowed(value: &MaybeBox<i32>) -> i32 {
    if let Some(payload) = value { **payload } else { 0 }
}
fn count_borrowed(value: &MaybeArc<i32>) -> usize {
    if let Some(payload) = value { Arc::strong_count(payload) } else { 0 }
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
    let holder = Holder { value: 19 };
    if holder.inspect(&Some(Arc::new(Counter::new(20)))) != 20 { return 19; }
    if holder.inspect_box(Some(Box::new(21))) != 21 { return 20; }
    let wrapped: Callback<Box<dyn Fn(MaybeOwned) -> i32>> = Callback { function: Box::new(|value: MaybeOwned| holder.inspect_box(value)) as Box<dyn Fn(MaybeOwned) -> i32> };
    if (wrapped.callable())(Some(Box::new(22))) != 22 { return 21; }
    let dispatched: MaybeOwned = Some(Box::new(22));
    if (wrapped.callable())(dispatched) != 22 { return 21; }
    let mut borrowed_owner: MaybeBox<i32> = Some(Box::new(23));
    mutate_borrowed(&mut borrowed_owner);
    if inspect_borrowed(&borrowed_owner) != 24 { return 22; }
    if let Some(ref payload) = borrowed_owner {
        if **payload != 24 { return 23; }
    }
    if borrowed_owner.is_none() { return 24; }
    if let Some(ref mut payload) = borrowed_owner { **payload += 1; }
    if let Some(_) = borrowed_owner { } else { return 25; }
    if let None = borrowed_owner { return 26; }
    if inspect_borrowed(&borrowed_owner) != 25 { return 27; }
    let borrowed_arc: MaybeArc<i32> = Some(Arc::new(26));
    if count_borrowed(&borrowed_arc) != 1 { return 28; }
    if count_borrowed(&borrowed_arc) != 1 { return 29; }
    0
}
"#;

#[test]
fn imported_nullable_profiles_reach_function_arguments_and_typed_callback_values() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"nullable_imports\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[lib]\npath = \"src/lib.rs\"\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub mod wrapper;\npub mod model;\npub mod consumer;\n").unwrap();
    std::fs::write(root.join("src/wrapper.rs"), r#"
pub struct Wrapper<F> { pub function: F }
impl<F> Wrapper<F> { pub fn callable(&self) -> &F { &self.function } }
"#).unwrap();
    std::fs::write(root.join("src/model.rs"), r#"
pub type OwnedThing = Box<i32>;
pub type MaybeOwned = Option<OwnedThing>;
pub type Callback = crate::wrapper::Wrapper<Box<dyn Fn(self::MaybeOwned) -> i32>>;
"#).unwrap();
    std::fs::write(root.join("src/consumer.rs"), r#"
use crate::model::{OwnedThing, MaybeOwned, Callback};
pub fn inspect(value: MaybeOwned) -> i32 {
    if let Some(value) = value { *value } else { 0 }
}
pub fn check(callback: &Callback) -> i32 {
    let connection: MaybeOwned = Some(Box::new(3));
    inspect(Some(Box::new(4))) + (callback.callable())(connection)
}
"#).unwrap();
    let map = root.join("types.toml");
    std::fs::write(&map, "MaybeOwned = \"::probe::OwnedThing\"\n").unwrap();
    let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate").arg(root.join("Cargo.toml"))
        .arg("--output-dir").arg(root.join("out"))
        .args(["--cxx-namespace", "probe", "--flat-import-namespace", "probe"])
        .arg("--type-map").arg(map).output().unwrap();
    assert!(generated.status.success(), "{}\n{}", String::from_utf8_lossy(&generated.stdout), String::from_utf8_lossy(&generated.stderr));
    let cpp = std::fs::read_to_string(root.join("out/nullable_imports.consumer.cppm")).unwrap();
    assert!(!cpp.contains("rusty::Option<"), "profile arguments must construct the nullable owner directly: {cpp}");
    assert!(!cpp.contains(".is_some()"), "profile patterns must inspect owner presence: {cpp}");
    assert!(cpp.contains("std::move(connection)"), "callback dispatch must move the profiled Box: {cpp}");
    let mut combined = String::new();
    for module in ["wrapper", "model", "consumer"] {
        let module_cpp = std::fs::read_to_string(root.join(format!("out/nullable_imports.{module}.cppm"))).unwrap();
        for line in module_cpp.lines() {
            if line == "module;" || line.starts_with("import ") || line.starts_with("export module ") { continue; }
            combined.push_str(line.strip_prefix("export ").unwrap_or(line));
            combined.push('\n');
        }
    }
    combined.push_str(r#"
int main() {
    probe::Callback callback{rusty::Function<int32_t(probe::OwnedThing) const>(
        [](probe::OwnedThing value) { return *value; })};
    return probe::check(callback) == 7 ? 0 : 1;
}
"#);
    let cpp_path = root.join("imported_check.cpp");
    std::fs::write(&cpp_path, combined).unwrap();
    let cpp_binary = root.join("imported_check");
    let compiled = Command::new(std::env::var("CXX").unwrap_or_else(|_| "clang++".into()))
        .args(["-std=c++23", "-DRUSTY_PORTABLE_INTRINSICS=1", "-pthread", "-I"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&cpp_path).arg("-o").arg(&cpp_binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let ran = Command::new(cpp_binary).output().unwrap();
    assert!(ran.status.success(), "{}\n{}", ran.status, String::from_utf8_lossy(&ran.stderr));
    let native = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
        .arg(root.join("src/lib.rs")).arg("-o").arg(root.join("native.rlib")).output().unwrap();
    assert!(native.status.success(), "{}", String::from_utf8_lossy(&native.stderr));
}
