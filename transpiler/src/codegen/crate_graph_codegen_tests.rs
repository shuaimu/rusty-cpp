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

// ---- lion-reactor lowering gaps -------------------------------------------

#[test]
fn async_method_returns_its_task_and_co_returns() {
    let cpp = translate(
        r#"
pub struct Conn { pub n: u32 }
impl Conn {
    pub async fn read(&self) -> u32 { self.n + 1 }
}
"#,
    );
    assert!(cpp.contains("rusty::Task<uint32_t> read() const;"), "{cpp}");
    assert!(cpp.contains("rusty::Task<uint32_t> Conn::read() const {"), "{cpp}");
    assert!(cpp.contains("co_return "), "{cpp}");
}

#[test]
fn raw_pointer_lowerings_reach_untyped_closure_params_and_cast_locals() {
    let cpp = translate(
        r#"
use std::cell::Cell;
pub struct Node { pub v: u32 }
thread_local! { static CURRENT: Cell<Option<*mut Node>> = Cell::new(None); }
pub fn through(opt: Option<*mut Node>) -> Option<u32> {
    opt.and_then(|p| unsafe { p.as_mut() }).map(|n| n.v)
}
pub fn is_current(key: usize) -> bool {
    CURRENT.with(|r| r.get().map(|p| p as usize) == Some(key))
}
pub struct Owner { pub x: u64 }
impl Owner {
    pub fn key(&mut self) -> usize {
        let p = self as *mut Owner;
        p as usize
    }
}
"#,
    );
    // Untyped closure parameters: dispatch on the C++ type.
    assert!(cpp.contains("return rusty::ptr::as_mut_dispatch(p);"), "{cpp}");
    assert!(
        cpp.contains("return rusty::ptr::detail::integer_or_address_cast<size_t>(p);"),
        "{cpp}"
    );
    // A local bound by a pointer cast is a pointer (static_cast<size_t> of a
    // pointer is ill-formed).
    assert!(
        cpp.contains("return static_cast<size_t>(reinterpret_cast<std::uintptr_t>(p));"),
        "{cpp}"
    );
}

#[test]
fn trait_object_call_types_its_arguments_from_the_trait() {
    let cpp = translate(
        r#"
pub trait Backend {
    fn wait(&mut self, timeout: Option<u64>) -> usize;
}
pub fn park(b: &mut Box<dyn Backend>) -> usize {
    b.wait(None)
}
"#,
    );
    assert!(cpp.contains("b->wait(rusty::Option<uint64_t>{rusty::None})"), "{cpp}");
}

#[test]
fn closure_with_early_return_is_typed_by_its_tail() {
    let cpp = translate(
        r#"
pub struct Reactor { pub n: u64 }
impl Reactor {
    pub fn register(&mut self, d: u64) -> Result<u64, u32> { self.n += d; Ok(self.n) }
}
pub fn with_reactor<F, R>(r: &mut Reactor, f: F) -> Option<R> where F: FnOnce(&mut Reactor) -> R {
    Some(f(r))
}
pub fn reg(r: &mut Reactor, d: u64) -> Option<Result<u64, u32>> {
    with_reactor(r, |reactor| {
        if d == 0 { return Err(1); }
        reactor.register(d)
    })
}
"#,
    );
    // The first `return` alone would deduce the lambda's type from `Err(1)`.
    assert!(
        cpp.contains("[&](auto&& reactor) -> std::remove_cvref_t<decltype(reactor.register_("),
        "{cpp}"
    );
    // `R` appears only in the return type: it defaults to the callable's
    // result, so the forward declaration and every call can name it.
    assert!(
        cpp.contains(
            "typename R = rusty::detail::unit_if_void_t<std::remove_cvref_t<std::invoke_result_t<F &, Reactor &>>>"
        ),
        "{cpp}"
    );
}

#[test]
fn closure_tail_that_is_a_bare_variant_is_not_annotated() {
    // `IoResult_Ok{v}` names the variant struct, not the enum.
    let cpp = translate(
        r#"
pub enum IoResult<T> { Ok(T), Err(i32) }
pub fn run(x: u32) -> IoResult<u32> {
    let f = |v: u32| {
        if v == 0 { return IoResult::Err(1); }
        IoResult::Ok(v)
    };
    f(x)
}
"#,
    );
    assert!(!cpp.contains("decltype(IoResult_Ok"), "{cpp}");
}

#[test]
fn bare_tuple_struct_constructor_is_not_a_variant_of_the_expected_owner() {
    let cpp = translate(
        r#"
#[derive(Clone, Copy)]
pub struct ResourceId(pub u64);
pub struct IoEvent { pub id: ResourceId }
pub fn events(raw: u64) -> Vec<IoEvent> {
    let id = ResourceId(raw);
    let mut out = Vec::new();
    out.push(IoEvent { id });
    out
}
"#,
    );
    assert!(cpp.contains("auto id = ResourceId(std::move(raw));"), "{cpp}");
    assert!(!cpp.contains("rusty::Vec<IoEvent>::ResourceId"), "{cpp}");
}

#[test]
fn hoisted_enum_methods_see_the_scope_imports() {
    let cpp = translate(
        r#"
pub mod types {
    pub struct Interest(pub u8);
    impl Interest { pub const READABLE: Interest = Interest(1); }
}
pub mod ev {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Kind { Read, Write }
    use crate::types::Interest;
    impl Kind {
        pub fn interest(self) -> u8 { match self { Kind::Read => Interest::READABLE.0, Kind::Write => 2 } }
    }
}
"#,
    );
    let import = cpp.find("using types::Interest;").expect(&cpp);
    let body = cpp.find("inline uint8_t interest(Kind self_) {").expect(&cpp);
    assert!(import < body, "{cpp}");
}

#[test]
fn poll_ready_payload_is_typed_by_the_poll_output() {
    let cpp = translate(
        r#"
use std::task::Poll;
use std::io;
pub fn poll_read(fail: bool) -> Poll<io::Result<usize>> {
    if fail {
        return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "x")));
    }
    Poll::Ready(Ok(3))
}
"#,
    );
    assert!(cpp.contains("ready_with(rusty::io::Result<size_t>::err("), "{cpp}");
    assert!(
        cpp.contains("ready_with(rusty::io::Result<size_t>::ok(static_cast<size_t>(3)))"),
        "{cpp}"
    );
    assert!(!cpp.contains("rusty::Poll<rusty::io::Result<size_t>>::Err("), "{cpp}");
}

#[test]
fn once_lock_and_poll_fn_map_to_the_runtime() {
    let cpp = translate(
        r#"
use std::future::poll_fn;
use std::sync::OnceLock;
use std::task::Poll;
static CELL: OnceLock<u32> = OnceLock::new();
pub fn get() -> u32 { *CELL.get_or_init(|| 7) }
pub async fn ready_now() -> u32 {
    poll_fn(|_cx| Poll::Ready(5u32)).await
}
"#,
    );
    assert!(
        cpp.contains("inline rusty::OnceLock<uint32_t> CELL = rusty::OnceLock<uint32_t>::new_();"),
        "{cpp}"
    );
    assert!(cpp.contains("co_await rusty::future::poll_fn("), "{cpp}");
    assert!(!cpp.contains("std::sync::OnceLock<"), "{cpp}");
}

#[test]
fn call_initialised_local_consumed_by_value_is_not_const() {
    let cpp = translate(
        r#"
use std::collections::HashMap;
pub struct Waiter { pub n: u32 }
impl Waiter {
    pub fn with_n(self, n: u32) -> Waiter { Waiter { n: self.n + n } }
}
pub struct Table { pub inner: HashMap<u64, Waiter> }
impl Table {
    pub fn bump(&mut self, key: u64) {
        let old = self.inner.remove(&key).unwrap();
        let new = old.with_n(5);
        self.inner.insert(key, new);
    }
}
"#,
    );
    assert!(cpp.contains("    auto old = this->inner.remove(key).unwrap();"), "{cpp}");
}

#[test]
fn returned_match_local_takes_the_full_return_type() {
    let cpp = translate(
        r#"
pub struct ResourceId(pub u64);
pub enum IoResult<T> { Ok(T), Err(i32) }
pub fn reg(ok: bool) -> IoResult<ResourceId> {
    let r = match ok {
        true => IoResult::Ok(ResourceId(1)),
        false => IoResult::Err(2),
    };
    r
}
"#,
    );
    assert!(cpp.contains("auto r = [&]() -> IoResult<ResourceId> {"), "{cpp}");
}

#[test]
fn guard_receiver_reaches_the_member_before_an_enum_free_function() {
    // `clear` is also a C-like enum's method (a free function in C++); a
    // `RefMut<HashMap>` reaches HashMap::clear through its deref.
    let cpp = translate(
        r#"
use std::cell::RefCell;
use std::collections::HashMap;
#[derive(Clone, Copy)]
pub enum Direction { Read, Write }
impl Direction {
    pub fn clear(self, x: u64) -> u64 { match self { Direction::Read => x, Direction::Write => x + 1 } }
}
pub fn wipe(cell: &RefCell<HashMap<u64, u8>>) {
    if let Ok(mut r) = cell.try_borrow_mut() {
        r.clear();
    }
}
"#,
    );
    assert!(
        cpp.contains("else if constexpr (requires { (*__self).clear(); }) { return (*__self).clear(); }"),
        "{cpp}"
    );
}
