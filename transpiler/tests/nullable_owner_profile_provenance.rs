use std::process::Command;

fn consumer_output(provider: &str, consumer: &str, mapping: &str) -> String {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"owner_provenance\"\nversion = \"0.1.0\"\nedition = \"2024\"\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub mod provider; pub mod consumer;\n").unwrap();
    std::fs::write(root.join("src/provider.rs"), provider).unwrap();
    std::fs::write(root.join("src/consumer.rs"), consumer).unwrap();
    std::fs::write(root.join("types.toml"), mapping).unwrap();
    let native = Command::new("rustc").args(["--edition=2024", "--crate-type=lib"])
        .arg(root.join("src/lib.rs")).arg("-o").arg(root.join("native.rlib"))
        .output().unwrap();
    assert!(native.status.success(), "{}", String::from_utf8_lossy(&native.stderr));
    let generated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg("--crate").arg(root.join("Cargo.toml"))
        .arg("--output-dir").arg(root.join("out"))
        .args(["--cxx-namespace", "probe", "--flat-import-namespace", "probe"])
        .arg("--type-map").arg(root.join("types.toml")).output().unwrap();
    assert!(generated.status.success(), "{}\n{}", String::from_utf8_lossy(&generated.stdout), String::from_utf8_lossy(&generated.stderr));
    std::fs::read_to_string(root.join("out/owner_provenance.consumer.cppm")).unwrap()
}

#[test]
fn imported_nullable_owners_reject_provider_local_standard_lookalikes() {
    let consumer = "use crate::provider::MaybeOwner; pub fn present(value: &MaybeOwner) -> bool { value.is_some() }";
    for provider in [
        "pub struct Box<T> { pub payload: T } pub type MaybeOwner = Option<Box<i32>>;",
        "pub mod std { pub mod boxed { pub struct Box<T> { pub payload: T } } } pub type MaybeOwner = Option<std::boxed::Box<i32>>;",
        "mod local { pub struct Box<T> { pub payload: T } } use self::local::Box; pub type MaybeOwner = Option<Box<i32>>;",
        "pub mod std { pub mod boxed { pub struct Box<T> { pub payload: T } } } use std::boxed::Box; pub type MaybeOwner = Option<Box<i32>>;",
        "pub struct Option<T> { pub payload: T } impl<T> Option<T> { pub fn is_some(&self) -> bool { true } } pub type MaybeOwner = Option<Box<i32>>;",
    ] {
        let cpp = consumer_output(provider, consumer, "MaybeOwner = \"rusty::Box<int32_t>\"\n");
        assert!(!cpp.contains("static_cast<bool>(value)"), "provider was incorrectly profiled: {provider}\n{cpp}");
    }
    let cpp = consumer_output(
        "pub struct Arc<T> { pub payload: T } pub type MaybeOwner = Option<Arc<i32>>;",
        consumer, "MaybeOwner = \"rusty::Arc<int32_t>\"\n",
    );
    assert!(!cpp.contains("static_cast<bool>(value)"), "{cpp}");
    let cpp = consumer_output(
        "pub struct Box<T> { pub payload: T } pub type MaybeOwner = Option<Box<i32>>;",
        "use crate::provider::MaybeOwner; mod unrelated { type MaybeOwner = Option<Box<i32>>; } pub fn present(value: &MaybeOwner) -> bool { value.is_some() }",
        "MaybeOwner = \"rusty::Box<int32_t>\"\n",
    );
    assert!(!cpp.contains("static_cast<bool>(value)"), "unrelated consumer alias bypassed provider proof: {cpp}");
}

#[test]
fn canonical_imports_and_inner_owner_aliases_keep_explicit_profiles() {
    let consumer = "use crate::provider::MaybeOwner; pub fn present(value: &MaybeOwner) -> bool { value.is_some() }";
    for provider in [
        "pub type MaybeOwner = Option<Box<i32>>;",
        "use std::boxed::Box as Heap; use std::option::Option as Maybe; pub type MaybeOwner = Maybe<Heap<i32>>;",
        "mod std {} pub type MaybeOwner = ::std::option::Option<::std::boxed::Box<i32>>;",
    ] {
        let cpp = consumer_output(provider, consumer, "MaybeOwner = \"rusty::Box<int32_t>\"\n");
        assert!(cpp.contains("static_cast<bool>(value)"), "canonical owner lost profile: {provider}\n{cpp}");
    }
    let cpp = consumer_output(
        "pub trait ChannelConnectionBase { fn value(&self) -> i32; } pub type ChannelConnectionProxy = Box<dyn ChannelConnectionBase>; pub type NullableChannelConnectionProxy = Option<ChannelConnectionProxy>;",
        "use crate::provider::{ChannelConnectionProxy, NullableChannelConnectionProxy}; pub fn present(value: &NullableChannelConnectionProxy) -> bool { value.is_some() } pub fn ordinary(value: &Option<ChannelConnectionProxy>) -> bool { value.is_some() }",
        "NullableChannelConnectionProxy = \"::probe::ChannelConnectionProxy\"\n",
    );
    assert!(cpp.contains("static_cast<bool>(value)"), "{cpp}");
    assert!(cpp.contains("rusty::Option<::probe::ChannelConnectionProxy>"), "ordinary Option must retain its representation: {cpp}");
    assert!(cpp.contains("return value.is_some();"), "{cpp}");
}

#[test]
fn imported_lookalike_callback_boxes_do_not_gain_implicit_transparency() {
    let cpp = consumer_output(
        "pub struct Box<T: ?Sized> { pub pointer: *const T } pub type Callback = Option<Box<dyn Fn()>>;",
        "use crate::provider::Callback; pub fn present(value: &Callback) -> bool { value.is_some() }",
        "",
    );
    assert!(!cpp.contains("static_cast<bool>(value)"), "{cpp}");
}
