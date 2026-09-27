//! Codegen fixtures for shapes `--crate-graph` output depends on: a crate's
//! Rust modules emitted as nested namespaces of ONE module, and dyn traits
//! named across those namespaces and implemented across crates.
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
