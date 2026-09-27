//! Codegen fixtures for shapes `--crate-graph` output depends on: a crate's
//! Rust modules emitted as nested namespaces of ONE module, dyn traits named
//! across those namespaces and across crates, and the lowering gaps Lion's
//! runtime crates surfaced.
use super::*;

fn translate(source: &str) -> String {
    let mut generator = CodeGen::new();
    generator.emit_file(&syn::parse_str(source).unwrap(), Some("demo"));
    generator.into_output()
}

/// As a `--crate-graph` dependency crate is emitted.
fn translate_dependency(source: &str) -> String {
    let mut generator = CodeGen::new();
    generator.set_emit_dyn_adapters(true);
    generator.emit_file(&syn::parse_str(source).unwrap(), Some("demo"));
    generator.into_output()
}

#[test]
fn owner_constructor_local_is_typed_by_its_struct_field_consumption() {
    // `let inner = Slab::new(); R { inner }`: the field's declared type pins
    // the generic owner (Rust infers `Slab<u32>` backwards). Before, the
    // initializer emitted `Slab<auto>::new_()` (a hard emit-time failure).
    let cpp = translate(
        r#"
pub mod m {
    pub struct Slab<V> { v: Vec<V> }
    impl<V> Slab<V> { pub fn new() -> Self { Slab { v: Vec::new() } } }
}
pub struct R { pub inner: m::Slab<u32> }
impl R {
    pub fn new() -> Self {
        let inner = m::Slab::new();
        let r = R { inner };
        r
    }
}
"#,
    );
    assert!(cpp.contains("auto inner = ::m::Slab<uint32_t>::new_();"), "{cpp}");
}

#[test]
fn dyn_trait_is_named_through_its_declaring_module() {
    let cpp = translate(
        r#"
pub mod os {
    pub trait Backend { fn wait(&mut self) -> u32; }
    pub struct Poll { pub inner: Box<dyn Backend> }
}
pub mod reactor {
    use crate::os::{Backend, Poll};
    pub fn make(b: Box<dyn Backend>) -> Poll { Poll { inner: b } }
    pub struct Null;
    impl Backend for Null { fn wait(&mut self) -> u32 { 7 } }
    pub fn run() -> u32 { let mut p = make(Box::new(Null)); p.inner.wait() }
}
"#,
    );
    // Inside `os` the bare name; from `reactor` (where the Rust-only trait
    // import emits nothing) the declaring module's path.
    assert!(cpp.contains("rusty::Box<Backend> inner;"), "{cpp}");
    assert!(cpp.contains("::os::Poll make(rusty::Box<::os::Backend> b)"), "{cpp}");
    // The construction-site adapter keeps the qualifier.
    assert!(cpp.contains("rusty::Box<::os::BackendAdapter<Null>>::new_(Null{})"), "{cpp}");
}

#[test]
fn trait_interface_names_its_generic_owning_adapter() {
    let plain = translate("pub trait OsBackend { fn wait(&mut self) -> u32; }");
    assert!(!plain.contains("DynAdapter") && !plain.contains("rusty_dyn_adapter"), "{plain}");
    let cpp = translate_dependency(
        r#"
pub trait OsBackend: Send {
    fn register(&mut self, fd: i32, token: usize) -> u32;
    fn deregister(&self, fd: &i32) -> bool;
}
"#,
    );
    assert!(cpp.contains("template <class U> using rusty_dyn_adapter = OsBackendDynAdapter<U>;"), "{cpp}");
    assert!(cpp.contains("template <class U> class OsBackendDynAdapter final : public OsBackend {"), "{cpp}");
    // Declared in the class; defined after the purview, where every type the
    // signatures name is complete.
    assert!(cpp.contains("uint32_t register_(int32_t fd, size_t token) override;"), "{cpp}");
    assert!(
        cpp.contains(
            "template <class U>\nauto ::OsBackendDynAdapter<U>::register_(int32_t fd, size_t token) -> uint32_t { return this->rusty_target().register_(std::move(fd), std::move(token)); }"
        ),
        "{cpp}"
    );
    assert!(
        cpp.contains(
            "auto ::OsBackendDynAdapter<U>::deregister(const int32_t& fd) const -> bool { return this->rusty_target().deregister(fd); }"
        ),
        "{cpp}"
    );
    // A private trait cannot be implemented by another crate.
    let private = translate_dependency("trait Hidden { fn f(&self) -> u32; }");
    assert!(!private.contains("DynAdapter"), "{private}");
    // Generic traits keep only the explicit-specialization adapters.
    let generic = translate_dependency("pub trait Sink<T> { fn put(&mut self, value: T); }");
    assert!(!generic.contains("DynAdapter"), "{generic}");
}

#[test]
fn renamed_poll_import_is_an_alias_template() {
    let cpp = translate(
        r#"
use std::task::Poll as StdPoll;
pub fn ready(v: u8) -> StdPoll<u8> { StdPoll::Ready(v) }
"#,
    );
    assert!(cpp.contains("template<typename T0> using StdPoll = rusty::Poll<T0>;"), "{cpp}");
}

#[test]
fn hash_map_value_views_map_to_the_std_port() {
    let cpp = translate(
        r#"
use std::collections::HashMap;
use std::collections::hash_map::Values;
pub struct Slab<V> { pub inner: HashMap<u64, V> }
impl<V> Slab<V> {
    pub fn values(&self) -> Values<'_, u64, V> { self.inner.values() }
}
"#,
    );
    assert!(
        cpp.contains("::std_port::collections::hash::map::Values<uint64_t, V> values() const"),
        "{cpp}"
    );
}

#[test]
fn derive_copy_and_hash_lower_without_slots() {
    let cpp = translate(
        r#"
pub mod wheel {
    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
    pub struct WheelPos { pub level: u8, pub slot: u8, pub idx: u64 }
    #[derive(Clone, Copy, Hash, PartialEq, Eq)]
    pub struct Id(pub u64);
    #[derive(Hash)]
    pub struct Tagged<T> { pub tag: T }
}
"#,
    );
    // Copy is the aggregate's implicit copy: no TODO slot.
    assert!(!cpp.contains("TODO"), "{cpp}");
    assert!(cpp.contains("// derive(Copy): implicit member-wise copy"), "{cpp}");
    // Hash combines every field, as a global-scope std::hash specialization
    // spelled with the type's qualified name.
    assert!(
        cpp.contains(
            "template<>\nstruct std::hash<::wheel::WheelPos> {\n    size_t operator()(const ::wheel::WheelPos& v) const { return rusty::detail::hash_fields(v.level, v.slot, v.idx); }\n};"
        ),
        "{cpp}"
    );
    assert!(cpp.contains("return rusty::detail::hash_fields(v._0);"), "{cpp}");
    assert!(
        cpp.contains("template<typename T>\nstruct std::hash<::wheel::Tagged<T>> {"),
        "{cpp}"
    );
    // At namespace scope 0 (unindented), after the module's namespaces.
    let hash_at = cpp.find("\ntemplate<>\nstruct std::hash<::wheel::WheelPos> {").unwrap();
    let struct_at = cpp.find("struct Tagged {").unwrap();
    assert!(hash_at > struct_at, "{cpp}");
}
