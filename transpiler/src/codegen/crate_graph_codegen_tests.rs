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
    // An implementor whose trait body was kept as the tagged member beside
    // a same-signature inherent method is reached through that member.
    assert!(
        cpp.contains(
            "template <class U>\nauto ::OsBackendDynAdapter<U>::register_(int32_t fd, size_t token) -> uint32_t { if constexpr (requires { this->rusty_target().rusty_OsBackend_register(std::move(fd), std::move(token)); }) { return this->rusty_target().rusty_OsBackend_register(std::move(fd), std::move(token)); } else { return this->rusty_target().register_(std::move(fd), std::move(token)); } }"
        ),
        "{cpp}"
    );
    assert!(
        cpp.contains(
            "auto ::OsBackendDynAdapter<U>::deregister(const int32_t& fd) const -> bool { if constexpr (requires { this->rusty_target().rusty_OsBackend_deregister(fd); }) { return this->rusty_target().rusty_OsBackend_deregister(fd); } else { return this->rusty_target().deregister(fd); } }"
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

// ---- lion-executor name resolution and runtime surface --------------------

#[test]
fn std_path_resolves_under_a_glob_import() {
    // A glob-imported `std` beside the prelude `std` would be an ambiguity
    // error, so under a glob `Context::from_waker` is still std's.
    let cpp = translate(
        r#"
pub mod waker {
    pub fn helper() -> u8 { 1 }
}
pub mod ext {
    use std::task::{Context, Waker};
    use super::waker::*;
    pub fn make(w: &Waker) -> u8 {
        let cx = Context::from_waker(w);
        let _ = cx;
        helper()
    }
}
"#,
    );
    assert!(cpp.contains("auto cx = rusty::Context{std::addressof(w)};"), "{cpp}");
}

#[test]
fn pinned_send_future_field_and_reexported_field_type_derive_send() {
    let cpp = translate(
        r#"
pub mod types {
    mod boxed {
        use std::future::Future;
        use std::pin::Pin;
        pub struct Boxed {
            pub inner: Pin<Box<dyn Future<Output = ()> + Send>>,
        }
    }
    mod task {
        use super::Boxed;
        pub struct Task {
            pub id: u64,
            pub future: Boxed,
        }
    }
    pub use boxed::Boxed;
    pub use task::Task;
}
"#,
    );
    let task = cpp.split("export struct Task {").nth(1).expect(&cpp);
    let task = &task[..task.find("};").expect(&cpp)];
    assert!(task.contains("static constexpr bool is_send = true;"), "{cpp}");
    let boxed = cpp.split("export struct Boxed {").nth(1).expect(&cpp);
    let boxed = &boxed[..boxed.find("};").expect(&cpp)];
    assert!(boxed.contains("static constexpr bool is_send = true;"), "{cpp}");
}

#[test]
fn arc_as_ptr_address_and_poison_into_inner_value() {
    let cpp = translate(
        r#"
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
pub fn id(a: &Arc<u64>) -> usize {
    Arc::as_ptr(a) as usize
}
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
"#,
    );
    assert!(
        cpp.contains("static_cast<size_t>(reinterpret_cast<std::uintptr_t>(Arc<uint64_t>::as_ptr(a)))"),
        "{cpp}"
    );
    assert!(cpp.contains("m.lock().unwrap_or_else(rusty::sync::poison_into_inner)"), "{cpp}");
}

#[test]
fn imported_constructors_and_result_ctors_are_not_variants_of_a_poll_return() {
    let cpp = translate(
        r#"
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::task::Poll;
pub fn run(f: fn() -> u8) -> Poll<u8> {
    let value = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => return Poll::Pending,
    };
    Poll::Ready(value)
}
pub fn pick(x: bool) -> Poll<Result<u8, u8>> {
    let r = if x { Err(1) } else { Ok(2) };
    Poll::Ready(r)
}
"#,
    );
    assert!(cpp.contains("catch_unwind_std(AssertUnwindSafe(std::move(f)))"), "{cpp}");
    assert!(!cpp.contains("Poll<uint8_t>::AssertUnwindSafe"), "{cpp}");
    assert!(!cpp.contains("rusty::Poll<rusty::Result<uint8_t, uint8_t>>::Err"), "{cpp}");
}

#[test]
fn guard_recovered_from_a_poison_error_is_dereferenced() {
    // `lock().unwrap_or_else(PoisonError::into_inner)` is the guard, as
    // `lock().unwrap()` is: its methods are the protected value's.
    let cpp = translate(
        r#"
use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
pub struct Queue {
    pub ids: Mutex<VecDeque<u64>>,
}
impl Queue {
    pub fn push(&self, id: u64) {
        self.ids.lock().unwrap_or_else(PoisonError::into_inner).push_back(id);
    }
}
"#,
    );
    assert!(
        cpp.contains(
            "(*this->ids.lock().unwrap_or_else(rusty::sync::poison_into_inner)).push_back(std::move(id));"
        ),
        "{cpp}"
    );
}

#[test]
fn owner_parameter_hint_is_dropped_outside_the_owner() {
    // `finish(self, r: Result<T, u8>)` of `Sender<T>`: on a destructured,
    // untyped `s`, `T` is not in scope and must not be spelled.
    let cpp = translate(
        r#"
pub struct Sender<T> {
    pub v: Option<T>,
}
impl<T> Sender<T> {
    pub fn finish(self, r: Result<T, u8>) -> bool {
        r.is_ok()
    }
}
pub fn make<T>() -> (u8, Sender<T>) {
    (0, Sender { v: None })
}
pub fn go<R>() -> bool {
    let (_a, s) = make::<R>();
    s.finish(Err(1))
}
"#,
    );
    assert!(!cpp.contains("s.finish(rusty::Result<T,"), "{cpp}");
    assert!(cpp.contains("return s.finish(rusty::Err(1));"), "{cpp}");
}

#[test]
fn reexported_variant_import_is_still_its_enums_variant() {
    // either's shape: the root re-exports `Either::{Left, Right}` and a child
    // module imports them through `super`; the expected owner is spelled
    // through an associated type (`Option<Self::Item>`). The binding names a
    // variant, so `Left(..)` is Either's (it had fallen through to the
    // builtin `Alignment::Left`, a hard error in a template body).
    let cpp = translate(
        r#"
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
pub use crate::Either::{Left, Right};
pub mod iterator {
    use super::{Either, Left, Right};
    pub struct IterEither<L, R> {
        pub inner: Either<L, R>,
    }
    impl<L: Iterator, R: Iterator> Iterator for IterEither<L, R> {
        type Item = Either<L::Item, R::Item>;
        fn next(&mut self) -> Option<Self::Item> {
            Some(match self.inner {
                Left(ref mut inner) => Left(inner.next()?),
                Right(ref mut inner) => Right(inner.next()?),
            })
        }
    }
}
"#,
    );
    assert!(!cpp.contains("Alignment::"), "{cpp}");
    assert!(cpp.contains("::Left(RUSTY_TRY_OPT(inner.next()))"), "{cpp}");
}

#[test]
fn thread_local_key_closure_parameter_is_typed_by_the_key() {
    // lion-executor's tls.rs shape: `KEY.with(|c| ..)` hands the closure a
    // `&T`, T the key's declared value type. Typed, the destructured payload
    // is an `Arc<Queue>` whose methods go through `->`; untyped, the body
    // spelled `queue.take_all()` on the Arc (a hard error).
    let cpp = translate(
        r#"
use std::cell::RefCell;
use std::sync::Arc;
pub struct Queue {
    pub n: u32,
}
impl Queue {
    pub fn take_all(&self) -> u32 {
        self.n
    }
}
thread_local! {
    static CTX: RefCell<Option<(Arc<Queue>, u32)>> = RefCell::new(None);
}
pub fn set_ctx(q: Arc<Queue>, k: u32) {
    CTX.with(|c| *c.borrow_mut() = Some((q, k)));
}
pub fn drain() -> Option<u32> {
    CTX.with(|c| c.borrow().as_ref().map(|(queue, _)| queue.take_all()))
}
"#,
    );
    assert!(cpp.contains("return queue->take_all();"), "{cpp}");
    assert!(cpp.contains("*c.borrow_mut() = rusty::Option<"), "{cpp}");
    assert!(!cpp.contains("__mdisp_as_ref"), "{cpp}");
}

#[test]
fn try_borrow_guard_is_held_by_value_and_reaches_through_a_box() {
    // lion-executor's Runtime keeps `RefCell<Box<Executor>>` and takes it with
    // `try_borrow_mut().expect(..)`: the guard is a `RefMut<Box<Executor>>`
    // held by value, and Rust autoderefs through BOTH layers — `->` stops at
    // the Box. Over a plain `RefCell<Exec>` one layer stays one layer.
    let cpp = translate(
        r#"
use std::cell::RefCell;
pub struct Exec {
    pub n: u64,
}
impl Exec {
    pub fn tick(&mut self) {
        self.n += 1;
    }
}
pub struct Runtime {
    pub boxed: RefCell<Box<Exec>>,
    pub plain: RefCell<Exec>,
}
impl Runtime {
    pub fn run(&self, n: u64) -> u64 {
        let mut exec = self.boxed.try_borrow_mut().expect("not re-entrant");
        exec.n = n;
        exec.tick();
        let mut plain = self.plain.try_borrow_mut().expect("not re-entrant");
        plain.n = n * 10;
        plain.tick();
        exec.n + plain.n
    }
    pub fn busy(&self) -> bool {
        let _held = self.boxed.borrow_mut();
        self.boxed.try_borrow().is_err()
    }
}
"#,
    );
    assert!(
        cpp.contains("auto exec = this->boxed.try_borrow_mut().expect(\"not re-entrant\");"),
        "{cpp}"
    );
    assert!(cpp.contains("(**exec).n = std::move(n);"), "{cpp}");
    assert!(cpp.contains("(**exec).tick();"), "{cpp}");
    assert!(cpp.contains("(*plain).n = "), "{cpp}");
    assert!(cpp.contains("plain->tick();"), "{cpp}");
    assert!(cpp.contains("return this->boxed.try_borrow().is_err();"), "{cpp}");
}

#[test]
fn value_if_and_value_match_with_escaping_arms_lower_as_statements() {
    // lion-executor's poll_task and new_reactor shapes. A value `if` whose
    // `else` tail is a match with a `return` arm, and a value `match` with a
    // `?` arm, both escape the function: an IIFE traps that arm, and the
    // if-let slot typed a bare `Err(e)` as `Result<JoinError, JoinError>`.
    // Both lower as statements assigning an optional slot, the slot typed
    // from the arms (`Result<T, JoinError>` from `Ok(value)` and `Err(..)`).
    // Compiled and run against rustc: 13 1011 3 -9 5 -77 either way.
    let cpp = translate(
        r#"
use std::task::Poll;
pub struct JoinError {
    pub code: u32,
}
impl JoinError {
    pub fn cancelled() -> JoinError {
        JoinError { code: 1 }
    }
}
pub struct Sender<T> {
    pub v: Option<T>,
    pub cancelled: bool,
}
impl<T> Sender<T> {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
    pub fn finish(self, r: Result<T, JoinError>) -> u64 {
        match r {
            Ok(_) => 1,
            Err(e) => 100 + e.code as u64,
        }
    }
}
pub struct Countdown<T> {
    pub n: u64,
    pub v: Option<T>,
}
impl<T> Countdown<T> {
    pub fn pull(&mut self) -> Poll<T> {
        if self.n > 0 {
            self.n -= 1;
            Poll::Pending
        } else {
            Poll::Ready(self.v.take().expect("pulled twice"))
        }
    }
}
pub fn poll_step<T>(sender: Sender<T>, s: &mut Countdown<T>) -> Poll<u64> {
    let outcome = if sender.is_cancelled() {
        Err(JoinError::cancelled())
    } else {
        match s.pull() {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(value) => Ok(value),
        }
    };
    Poll::Ready(sender.finish(outcome))
}
pub enum Created<T> {
    Ok(T),
    Err(u32),
}
pub fn make(k: u64) -> Created<u64> {
    if k == 0 { Created::Err(9) } else { Created::Ok(k) }
}
fn fallback(ok: bool) -> Result<Created<u64>, u32> {
    if ok { Ok(Created::Ok(5)) } else { Err(77) }
}
pub fn create(k: Option<u64>, ok: bool) -> Result<u64, u32> {
    let created = match k {
        Some(k) => make(k),
        None => fallback(ok)?,
    };
    match created {
        Created::Ok(v) => Ok(v),
        Created::Err(e) => Err(e),
    }
}
"#,
    );
    assert!(cpp.contains("std::optional<rusty::Result<T, JoinError>> _let_if_value;"), "{cpp}");
    assert!(cpp.contains("_let_if_value.emplace(rusty::Err(JoinError::cancelled()));"), "{cpp}");
    assert!(cpp.contains("return rusty::Poll<uint64_t>::pending();"), "{cpp}");
    assert!(
        cpp.contains("rusty::Result<T, JoinError> outcome = std::move(_let_if_value).value();"),
        "{cpp}"
    );
    assert!(!cpp.contains("Result<JoinError, JoinError>"), "{cpp}");
    assert!(cpp.contains("std::optional<Created<uint64_t>> _let_match_value;"), "{cpp}");
    assert!(
        cpp.contains("_let_match_value.emplace(RUSTY_TRY_INTO(::fallback("),
        "{cpp}"
    );
    assert!(
        cpp.contains("Created<uint64_t> created = std::move(_let_match_value).value();"),
        "{cpp}"
    );
}

#[test]
fn owned_scrutinee_payloads_move_on() {
    // Rust MOVES an owned scrutinee's by-value payloads into the arm's
    // bindings; passing one on by value is a move. The arm bodies copied
    // them (a deleted copy for a move-only payload): lion-executor's
    // `Some(task) => ..(Some(task))`, `other => ..(other)` and
    // `Some(backend) => Reactor::with_backend(backend)` (a `?`-arm match).
    // Compiled and run against rustc: 4057063 either way.
    let cpp = translate(
        r#"
pub struct Payload {
    pub v: Box<u64>,
}
pub fn keep(p: Option<Payload>) -> (u64, Option<Payload>) {
    match p {
        Some(x) => (*x.v, None),
        None => (0, None),
    }
}
pub struct Exec {
    pub total: u64,
}
impl Exec {
    pub fn step(&mut self, v: Option<Payload>) {
        match v {
            Some(task) => {
                let (n, back) = keep(Some(task));
                self.total += n;
                let _ = back;
            }
            None => {}
        }
    }
}
pub enum Job {
    Run(Box<u64>),
    Stop,
}
pub fn consume(j: Job) -> u64 {
    match j {
        Job::Run(b) => *b,
        Job::Stop => 1000,
    }
}
pub fn route(j: Job) -> u64 {
    match j {
        Job::Stop => 7,
        other => consume(other),
    }
}
pub fn with_backend(b: Box<u64>) -> Result<u64, u32> {
    Ok(*b + 1)
}
fn fallback(ok: bool) -> Result<Result<u64, u32>, u32> {
    if ok { Ok(Ok(50)) } else { Err(77) }
}
pub fn create(backend: Option<Box<u64>>, ok: bool) -> Result<u64, u32> {
    let created = match backend {
        Some(backend) => with_backend(backend),
        None => fallback(ok)?,
    };
    created
}
pub fn peek(backend: &Option<Box<u64>>, ok: bool) -> Result<u64, u32> {
    let seen = match backend {
        Some(b) => Ok(**b),
        None => fallback(ok)?,
    };
    seen
}
"#,
    );
    assert!(cpp.contains("::keep(rusty::Option<Payload>(std::move(task)))"), "{cpp}");
    assert!(
        cpp.contains("if (true) { auto&& other = _m; return ::consume(std::move(other)); }"),
        "{cpp}"
    );
    assert!(cpp.contains("_let_match_value.emplace(::with_backend(std::move(backend)));"), "{cpp}");
    // A borrowed scrutinee's payload stays a reference into it.
    assert!(!cpp.contains("std::move(b)"), "{cpp}");
}

#[test]
fn hand_written_future_poll_is_typed_through_the_pin_surface() {
    // lion-executor's TaskCell::poll and Runtime::block_on. The std pinning,
    // polling and unwinding surface types what the body builds from it:
    // `self.get_unchecked_mut()` is `&mut Self`; `Pin::new_unchecked(r)` over a
    // reference is the pinned place (aliased, never copied); `pin.poll(cx)`
    // is `Poll<F::Output>`, and `catch_unwind(..)` a `Result` of it, whose
    // nested `Poll::Ready(value)` types `value`. So the escaping `if`'s slot
    // is `Result<F::Output, JoinError>` (it was `Result<JoinError,
    // JoinError>`). `pin!(f)` over a parameter aliases it, and
    // `fut.as_mut()` reborrows the same place. Compiled and run against
    // rustc with a concrete future: 19007 19101 either way.
    let cpp = translate(
        r#"
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::{pin, Pin};
use std::task::{Context, Poll};
pub struct JoinError {
    pub code: u32,
}
impl JoinError {
    pub fn cancelled() -> JoinError {
        JoinError { code: 1 }
    }
}
pub struct TaskCell<F: Future> {
    future: Option<F>,
    cancelled: bool,
    out: Option<Result<F::Output, JoinError>>,
}
impl<F: Future> Future for TaskCell<F> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = unsafe { self.get_unchecked_mut() };
        let outcome = if this.cancelled {
            Err(JoinError::cancelled())
        } else {
            let future = this.future.as_mut().expect("unfinished task has its future");
            let future = unsafe { Pin::new_unchecked(future) };
            match catch_unwind(AssertUnwindSafe(|| future.poll(cx))) {
                Ok(Poll::Pending) => return Poll::Pending,
                Ok(Poll::Ready(value)) => Ok(value),
                Err(_payload) => Err(JoinError::cancelled()),
            }
        };
        this.out = Some(outcome);
        Poll::Ready(())
    }
}
pub fn poll_pinned<F: Future>(f: F, cx: &mut Context<'_>) -> Poll<F::Output> {
    let mut fut = pin!(f);
    let first = fut.as_mut().poll(cx);
    if first.is_ready() {
        return first;
    }
    fut.as_mut().poll(cx)
}
"#,
    );
    assert!(
        cpp.contains("TaskCell<F>& this_ = rusty::pin_place::get_unchecked_mut((*this));"),
        "{cpp}"
    );
    assert!(cpp.contains("auto& future_shadow1 = rusty::pin::new_unchecked(future);"), "{cpp}");
    assert!(
        cpp.contains("std::optional<rusty::Result<typename F::Output, JoinError>> _let_if_value;"),
        "{cpp}"
    );
    assert!(!cpp.contains("Result<JoinError, JoinError>"), "{cpp}");
    assert!(cpp.contains("auto& fut = f;"), "{cpp}");
    assert!(cpp.contains("auto first = rusty::pin_place::as_mut(fut).poll(cx);"), "{cpp}");
    assert!(!cpp.contains("pin!("), "{cpp}");
}

#[test]
fn generic_call_takes_template_arguments_from_its_bindings_later_use() {
    // `let (tx, rx) = queue();` with `T` only in `queue`'s return type:
    // Rust infers it from how the bindings are used later (lion-executor's
    // `Executor::new(rx, ..)` taking `Receiver<Task>`, and std's `channel()`
    // whose halves become fields typed `Sender<T>` / `Receiver<T>`); C++
    // deduced nothing (`queue()` has no viable overload) or `std::tuple<>`
    // (`channel()`). Compiled and run against rustc: 431 either way.
    let cpp = translate(
        r#"
use std::sync::mpsc::{channel, Receiver, Sender};
pub struct Tx<T> {
    pub v: Option<T>,
}
pub struct Rx<T> {
    pub v: Option<T>,
}
pub fn queue<T>() -> (Tx<T>, Rx<T>) {
    (Tx { v: None }, Rx { v: None })
}
pub struct Task {
    pub id: u64,
}
pub struct Exec {
    pub rx: Rx<Task>,
    pub n: u64,
}
impl Exec {
    pub fn new(rx: Rx<Task>, n: u64) -> Exec {
        Exec { rx, n }
    }
}
pub fn build() -> (Exec, Tx<Task>) {
    let (tx, rx) = queue();
    let e = Exec::new(rx, 3);
    (e, tx)
}
pub fn unused() -> u64 {
    let (_tx, _rx) = queue::<u64>();
    let (a, _b) = queue();
    let _keep: Tx<u8> = a;
    0
}
pub struct MpscSender<T> {
    sender: Sender<T>,
}
pub struct MpscReceiver<T> {
    receiver: Receiver<T>,
}
pub fn mpsc_queue<T>() -> (MpscSender<T>, MpscReceiver<T>) {
    let (sender, receiver) = channel();
    (MpscSender { sender }, MpscReceiver { receiver })
}
"#,
    );
    assert!(cpp.contains("::queue<Task>()"), "{cpp}");
    assert!(cpp.contains("channel<T>()"), "{cpp}");
    // Explicit arguments stay as written; a use the solver does not read
    // (a typed `let`) leaves the call as it was.
    assert!(cpp.contains("::queue<uint64_t>()"), "{cpp}");
    assert_eq!(cpp.matches("::queue()").count(), 1, "{cpp}");
}

#[test]
fn self_path_to_an_associated_fn_without_receiver_stays_a_static_call() {
    // lion-executor's `Self::try_recv_raw(&mut self.receiver)`: `Self::f`
    // names the impl's own type, whose `f` takes no receiver. It was lowered
    // as UFCS dispatch on the first argument (`this->q.try_recv_raw()`, no
    // such member); `Self::has(self)` stays static too. Compiled and run
    // against rustc (with a `run` driving both): 801 either way.
    let cpp = translate(
        r#"
pub struct Rx {
    pub v: Option<u64>,
}
impl Rx {
    pub fn pop(&mut self) -> Option<u64> {
        self.v.take()
    }
}
pub struct Exec {
    pub q: Rx,
}
impl Exec {
    fn try_recv_raw(rx: &mut Rx) -> Option<u64> {
        rx.pop()
    }
    pub fn drain(&mut self) -> u64 {
        let raw = Self::try_recv_raw(&mut self.q);
        raw.unwrap_or(0)
    }
    pub fn peek(&self) -> bool {
        Self::has(self)
    }
    fn has(this: &Exec) -> bool {
        this.q.v.is_some()
    }
}
"#,
    );
    assert!(cpp.contains("auto raw = Exec::try_recv_raw(this->q);"), "{cpp}");
    assert!(cpp.contains("return Exec::has((*this));"), "{cpp}");
    assert!(!cpp.contains("try_recv_raw();"), "{cpp}");
}

#[test]
fn foreign_from_impl_lowers_to_an_adl_conversion_hook() {
    // lion-executor's `impl<T> From<VecDeque<T>> for Vec<T>`: no struct the
    // crate emits can absorb a `from` for `Vec`, and C++ cannot add a member
    // to it. `from` becomes a free `rusty_from_impl(type_identity<Vec<T>>,
    // VecDeque<T>)` beside the impl, and the module's `from_into` (behind
    // `.into()` and `?`) finds it first by argument-dependent lookup. Its body
    // converts std's VecDeque through the runtime's own hook
    // (include/rusty/vecdeque.hpp). A `From` for a crate type stays a member.
    // Compiled (libc++, SRPC flags) and run against rustc: 2340 either way.
    let cpp = translate(
        r#"
pub mod collections {
    pub mod vec_deque {
        use std::collections::VecDeque as StdVecDeque;
        pub struct VecDeque<T> {
            inner: StdVecDeque<T>,
        }
        impl<T> VecDeque<T> {
            pub fn new() -> Self {
                VecDeque { inner: StdVecDeque::new() }
            }
            pub fn push_back(&mut self, v: T) {
                self.inner.push_back(v);
            }
        }
        impl<T> From<VecDeque<T>> for Vec<T> {
            fn from(deque: VecDeque<T>) -> Self {
                deque.inner.into()
            }
        }
    }
}
use collections::vec_deque::VecDeque;
pub struct Local(pub u8);
impl From<u8> for Local {
    fn from(v: u8) -> Self {
        Local(v)
    }
}
pub fn take() -> u64 {
    let mut d = VecDeque::new();
    d.push_back(3u64);
    d.push_back(4u64);
    let v: Vec<u64> = d.into();
    let l: Local = 7u8.into();
    v[0] * 100 + v[1] * 10 + v.len() as u64 * 1000 + l.0 as u64
}
"#,
    );
    assert!(
        cpp.contains("rusty::Vec<T> rusty_from_impl(std::type_identity<rusty::Vec<T>>, VecDeque<T> deque) {"),
        "{cpp}"
    );
    assert!(cpp.contains("return rusty::from_into<rusty::Vec<T>>(std::move(deque.inner));"), "{cpp}");
    assert!(
        cpp.contains(
            "if constexpr (requires { rusty_from_impl(std::type_identity<Target>{}, std::forward<Input>(input)); }) {"
        ),
        "{cpp}"
    );
    assert!(cpp.contains("rusty::from_into<rusty::Vec<uint64_t>>(std::move(d))"), "{cpp}");
    assert!(cpp.contains("static Local from(uint8_t v);"), "{cpp}");
    assert_eq!(cpp.matches("rusty_from_impl(std::type_identity<").count(), 3, "{cpp}");
}

#[test]
fn smart_pointer_assoc_fn_on_an_untyped_pointer_argument_names_its_type() {
    // lion-executor's `Arc::ptr_eq(q, queue)` with `q` a destructured closure
    // binding the emitter cannot type: the owner-args recovery read `q` as the
    // pointee and spelled `Arc<decltype(q)>`, i.e. `Arc<Arc<Queue>>`. The
    // owner IS the argument's own type (include/rusty/arc.hpp gains
    // `Arc::ptr_eq`). Compiled and run against rustc: 0 7 1 0 either way.
    let cpp = translate(
        r#"
use std::cell::RefCell;
use std::sync::Arc;
pub struct Queue {
    pub n: u32,
}
thread_local! {
    static CTX: RefCell<Option<(Arc<Queue>, u32)>> = RefCell::new(None);
}
pub fn is_ours(queue: &Arc<Queue>) -> bool {
    CTX.with(|c| c.borrow().as_ref().is_some_and(|(q, _)| Arc::ptr_eq(q, queue)))
}
pub fn same(a: &Arc<Queue>, b: &Arc<Queue>) -> bool {
    Arc::ptr_eq(a, b)
}
"#,
    );
    assert!(
        cpp.contains("std::remove_cvref_t<decltype(rusty::detail::deref_if_pointer(q))>::ptr_eq("),
        "{cpp}"
    );
    assert!(!cpp.contains("Arc<std::remove_cvref_t<decltype((q))>>"), "{cpp}");
    // A typed argument keeps its spelled owner.
    assert!(cpp.contains("return Arc<Queue>::ptr_eq(a, b);"), "{cpp}");
}

#[test]
fn merged_impl_method_resolves_names_through_its_authoring_module() {
    // lion-executor's `executor/ext.rs`: `impl Executor` blocks written in a
    // child module merge into the struct emitted in `executor`, which imports
    // no `Duration`; the method's names resolve through the module that
    // wrote it (`use crate::types::Duration;`). They were resolved from the
    // struct's module, where `Duration` fell to std's (`rusty::time::Duration`,
    // no `ms` field). Compiled and run against rustc: 5 either way.
    let cpp = translate(
        r#"
pub mod types {
    pub mod duration {
        #[derive(Clone, Copy)]
        pub struct Duration {
            pub ms: u64,
        }
        impl Duration {
            pub fn from_millis(ms: u64) -> Duration {
                Duration { ms }
            }
        }
    }
    pub use duration::Duration;
}
pub mod executor {
    pub mod ext {
        use super::Executor;
        use crate::types::Duration;
        impl Executor {
            pub fn idle_timeout(&self) -> Duration {
                Duration::from_millis(self.idle)
            }
        }
    }
    pub struct Executor {
        pub idle: u64,
    }
}
pub fn run() -> u64 {
    executor::Executor { idle: 5 }.idle_timeout().ms
}
"#,
    );
    assert!(!cpp.contains("rusty::time::Duration idle_timeout"), "{cpp}");
    assert!(cpp.contains("::types::duration::Duration idle_timeout() const;"), "{cpp}");
}

#[test]
fn closure_whose_body_is_an_assignment_returns_unit() {
    // lion-executor's TaskCell::poll drops its future in place through
    // `catch_unwind(AssertUnwindSafe(|| *future = None))`. An assignment is
    // `()` in Rust; returning the C++ assignment's value made the lambda
    // return a COPY of the assigned Option (a deleted copy for an
    // `Option<rusty::Task<T>>`). Compiled and run against rustc: the slot is
    // cleared either way.
    let cpp = translate(
        r#"
use std::panic::{catch_unwind, AssertUnwindSafe};
pub fn clear(slot: &mut Option<u64>) -> bool {
    catch_unwind(AssertUnwindSafe(|| *slot = None)).is_ok()
}
pub fn bump(n: &mut u64) {
    let mut add = |k: u64| *n += k;
    add(2);
}
pub fn get(v: &Option<u64>) -> u64 {
    let read = || v.unwrap_or(0);
    read()
}
"#,
    );
    assert!(cpp.contains("AssertUnwindSafe([&]() { *slot_shadow1 = rusty::None; })"), "{cpp}");
    assert!(!cpp.contains("return *slot_shadow1 = rusty::None;"), "{cpp}");
    assert!(!cpp.contains("(uint64_t k) { return "), "{cpp}");
    // A value body still returns its value.
    assert!(cpp.contains("[&]() { return v.unwrap_or(0); }"), "{cpp}");
}

#[test]
fn awaited_local_is_moved_and_an_awaited_match_scrutinee_is_hoisted() {
    // lion-executor's `match handle.await { Ok(v) => v, Err(_) => 0 }`:
    // `.await` consumes its operand, so the local is not `const` and moves
    // into the awaiter (a JoinHandle is move-only). The value-position match
    // lowers through a lambda, and a `co_await` inside a lambda made the
    // LAMBDA the coroutine (ill-formed with its `return`s): the scrutinee is
    // awaited first, in the enclosing coroutine. Compiled and run against
    // rustc with a polled JoinHandle: 0 1 1 either way.
    let cpp = translate(
        r#"
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
pub struct JoinHandle<T> {
    pub v: Option<T>,
}
impl<T> Unpin for JoinHandle<T> {}
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, u32>;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<T, u32>> {
        match self.v.take() {
            Some(v) => Poll::Ready(Ok(v)),
            None => Poll::Ready(Err(1)),
        }
    }
}
pub fn handle(v: u64) -> JoinHandle<u64> {
    JoinHandle { v: Some(v) }
}
pub async fn main_task() -> u64 {
    let sent = handle(21);
    let a = match sent.await {
        Ok(v) => v,
        Err(_) => 0,
    };
    a
}
"#,
    );
    assert!(cpp.contains("auto sent = ::handle(static_cast<uint64_t>(21));"), "{cpp}");
    assert!(cpp.contains("auto __awaited_1 = co_await std::move(sent);"), "{cpp}");
    assert!(cpp.contains("auto&& _m = __awaited_1;"), "{cpp}");
    assert!(!cpp.contains("_m = co_await"), "{cpp}");
}

#[test]
fn impl_future_argument_is_taken_by_value_and_its_output_deduced() {
    // lion-executor's `pub fn spawn<T>(future: impl Future<Output = T>) ->
    // JoinHandle<T>`: the future is consumed, so it is a by-value parameter
    // moved on (a `const auto&` could only copy the move-only rusty::Task);
    // and `T`, which Rust infers from the argument's `Output`, gets a
    // forwarding overload that computes it, so `spawn(fut)` resolves while
    // the transpiler's own `spawn<T>(..)` calls keep the primary.
    let cpp = translate(
        r#"
use std::future::Future;
pub struct JoinHandle<T> {
    pub v: Option<T>,
}
pub struct Boxed {
    pub n: u64,
}
fn boxed(_f: impl Future<Output = u64>) -> Boxed {
    Boxed { n: 1 }
}
pub fn spawn<T>(future: impl Future<Output = T>) -> JoinHandle<T> {
    let _keep = boxed_any(future);
    JoinHandle { v: None }
}
fn boxed_any<F: Future>(f: F) -> Option<F> {
    Some(f)
}
pub fn describe(x: impl std::fmt::Display) -> String {
    format!("{x}")
}
"#,
    );
    assert!(cpp.contains("JoinHandle<T> spawn(auto future) {"), "{cpp}");
    assert!(cpp.contains("::boxed_any(std::move(future))"), "{cpp}");
    assert!(
        cpp.contains(
            "decltype(auto) spawn(__RustyImplArg0 future) requires (!std::is_same_v<typename rusty::detail::assoc_Output<__RustyImplArg0>::type, rusty::detail::missing_assoc_type>) {"
        ),
        "{cpp}"
    );
    assert!(
        cpp.contains(
            "return spawn<typename rusty::detail::assoc_Output<__RustyImplArg0>::type>(std::forward<decltype(future)>(future));"
        ),
        "{cpp}"
    );
    // A fully deducible signature gets no overload; a non-future impl
    // argument keeps its reference.
    assert_eq!(cpp.matches("decltype(auto) spawn(").count(), 1, "{cpp}");
    assert!(!cpp.contains("decltype(auto) boxed(") && !cpp.contains("decltype(auto) describe("), "{cpp}");
    assert!(cpp.contains("describe(const auto& x)"), "{cpp}");
}

#[test]
fn non_copy_local_passed_by_value_to_an_unknown_callee_is_not_const() {
    // SRPC's poll thread: `let os_backend: Box<dyn OsBackend> = ..;
    // RuntimeBuilder::new().os_backend(os_backend)` and `let lion_task =
    // StacklessLionVoidTask { .. }; lion_executor::spawn_local(lion_task)`,
    // both callees in a dependency crate with no signature here. A non-Copy
    // local passed by value is moved; declared `const`, the emitted
    // `std::move` copied it, and a move-only value has no copy.
    let cpp = translate(
        r#"
pub struct Task {
    pub id: u64,
}
pub fn drive(builder: &mut dep::Builder) -> u64 {
    let task = Task { id: 4 };
    dep::spawn_local(task);
    let backend: Box<u64> = Box::new(9);
    builder.os_backend(backend);
    let kept = Task { id: 1 };
    dep::inspect(&kept);
    kept.id
}
"#,
    );
    assert!(cpp.contains("    auto task = Task{.id = static_cast<uint64_t>(4)};"), "{cpp}");
    assert!(cpp.contains("dep::spawn_local(std::move(task));"), "{cpp}");
    assert!(
        cpp.contains("    rusty::Box<uint64_t> backend = rusty::Box<uint64_t>::new_("),
        "{cpp}"
    );
    assert!(!cpp.contains("const auto task") && !cpp.contains("const rusty::Box<uint64_t> backend"), "{cpp}");
}

#[test]
fn reference_to_a_smart_pointer_coerces_to_its_pointee() {
    // SRPC's tcp_channel: `let conn: &TcpConnection = &t.conn_;` with
    // `conn_: Arc<TcpConnection>` is Rust's deref coercion; the C++ reference
    // binding needs the pointee (`Arc` does not convert to its target). A
    // reference to the pointer itself stays the pointer.
    let cpp = translate(
        r#"
use std::rc::Rc;
use std::sync::Arc;
pub struct Conn {
    pub n: u64,
}
pub struct Transport {
    pub conn_: Arc<Conn>,
    pub local_: Rc<Conn>,
    pub boxed_: Box<Conn>,
}
pub fn read(t: &Transport) -> u64 {
    let conn: &Conn = &t.conn_;
    let local: &Conn = &t.local_;
    let boxed: &Conn = &t.boxed_;
    let arc: &Arc<Conn> = &t.conn_;
    conn.n + local.n + boxed.n + arc.n
}
pub fn from_local(c: Arc<Conn>) -> u64 {
    let lst: &Conn = &c;
    lst.n
}
"#,
    );
    assert!(cpp.contains("const Conn& conn = (*t.conn_);"), "{cpp}");
    assert!(cpp.contains("const Conn& local = (*t.local_);"), "{cpp}");
    assert!(cpp.contains("const Conn& boxed = (*t.boxed_);"), "{cpp}");
    assert!(cpp.contains("const rusty::Arc<Conn>& arc = t.conn_;"), "{cpp}");
    assert!(cpp.contains("const Conn& lst = (*c);"), "{cpp}");
}

#[test]
fn crate_type_method_called_through_its_path_is_a_typed_method_call() {
    // SRPC's epoll backend: `SrpcEpollBackend::wait(self, &mut batch,
    // timeout)` inside its `OsBackend` impl. The path call is the method call
    // `self.wait(&mut batch, timeout)`, typed by the method's parameters: the
    // `&mut` argument binds the `Vec&` parameter (it emitted the pointer
    // `&batch`), `Some(3)` takes `Option<u64>`, and the argument positions skip
    // the receiver (an untyped `Vec::new()` local was typed by the NEXT
    // parameter). Its known by-value result binds by value, whatever the
    // OnceCell-style `wait` name heuristic said (`auto&` on a prvalue).
    // Compiled and run against rustc: 6 either way.
    let cpp = translate(
        r#"
pub struct Backend {
    pub n: u64,
}
impl Backend {
    pub fn wait(&mut self, events: &mut Vec<u64>, timeout: Option<u64>) -> std::io::Result<()> {
        events.push(timeout.unwrap_or(self.n));
        Ok(())
    }
    pub fn force(&self) -> u64 {
        self.n
    }
    pub fn inner(&mut self) -> usize {
        let mut batch = Vec::new();
        let _ = Backend::wait(self, &mut batch, Some(4));
        let _ = Self::wait(self, &mut batch, None);
        batch.len()
    }
}
pub fn drive(b: &mut Backend) -> usize {
    let mut batch = Vec::new();
    let result = Backend::wait(b, &mut batch, Some(3));
    let forced = Backend::force(b);
    if result.is_ok() { batch.len() + forced as usize } else { 0 }
}
"#,
    );
    assert!(cpp.contains("auto batch = rusty::Vec<uint64_t>::new_();"), "{cpp}");
    assert!(
        cpp.contains(
            "const auto result = b.wait(batch, rusty::Option<uint64_t>(static_cast<uint64_t>(3)));"
        ),
        "{cpp}"
    );
    assert!(cpp.contains("const auto forced = b.force();"), "{cpp}");
    assert!(
        cpp.contains("this->wait(batch, rusty::Option<uint64_t>(static_cast<uint64_t>(4)))"),
        "{cpp}"
    );
    assert!(cpp.contains("this->wait(batch, rusty::Option<uint64_t>{rusty::None})"), "{cpp}");
    assert!(!cpp.contains("wait(&batch"), "{cpp}");
}

#[test]
fn as_mut_if_let_payload_keeps_a_boxed_value() {
    // SRPC's poll_fd_entry_handle_read: `if let Some(p) = (*guard).as_mut()`
    // over an `Option<Box<dyn PollableBase>>` binds `p: &mut Box<dyn ..>`,
    // which the body re-annotates (`let p: &mut Box<dyn PollableBase> = p;`)
    // to hand the emitter the Box. The payload peel reached THROUGH the Box
    // (a pointer-like), so `rusty::Box<..>& p2 = p` bound the trait object and
    // did not compile. With the re-annotation only a raw pointer is peeled;
    // without it the pointer-like peel stays (SRPC's server calls through an
    // unannotated payload of a `Box` alias it relies on).
    let cpp = translate(
        r#"
use std::cell::RefCell;
pub trait Pollable {
    fn handle_read(&mut self) -> u32;
}
pub struct Entry {
    pub proxy: RefCell<Option<Box<dyn Pollable>>>,
}
pub fn handle_read(entry: &Entry) -> u32 {
    let mut guard = entry.proxy.borrow_mut();
    if let Some(p) = (*guard).as_mut() {
        let p: &mut Box<dyn Pollable> = p;
        return p.handle_read();
    }
    0
}
pub struct Other {
    pub slot: Option<u32>,
}
pub fn bump(o: &mut Other) {
    if let Some(v) = o.slot.as_mut() {
        *v += 1;
    }
}
"#,
    );
    assert!(cpp.contains("auto& p = rusty::detail::deref_if_pointer(_iflet_scrutinee.unwrap());"), "{cpp}");
    assert!(cpp.contains("rusty::Box<Pollable>& p_shadow1 = p;"), "{cpp}");
    assert!(cpp.contains("auto& v = rusty::detail::deref_if_pointer_like(_iflet_scrutinee.unwrap());"), "{cpp}");
}

#[test]
fn catch_unwind_over_a_callable_type_parameter_yields_its_bounds_output() {
    // take_mut's take_or_recover: `recover: R` with `R: FnOnce() -> T`, and
    // `let r = catch_unwind(AssertUnwindSafe(|| recover())).unwrap_or_else(..)`.
    // The closure calls a binding typed by the callable parameter, so the
    // result is `Result<T, _>` and `r` a `T`; typed as `R` it declared
    // `R r` and the lambda's `T` did not convert. A call that merely
    // returns a type parameter (`make::<T>()`) keeps it.
    let cpp = translate(
        r#"
use std::panic;
pub fn take_or_recover<T, F, R>(mut_ref: &mut T, recover: R, closure: F)
where
    F: FnOnce(T) -> T,
    R: FnOnce() -> T,
{
    unsafe {
        let old_t = std::ptr::read(mut_ref);
        let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)));
        match new_t {
            Err(err) => {
                let r = panic::catch_unwind(panic::AssertUnwindSafe(|| recover()))
                    .unwrap_or_else(|_| std::process::abort());
                std::ptr::write(mut_ref, r);
                panic::resume_unwind(err);
            }
            Ok(new_t) => std::ptr::write(mut_ref, new_t),
        }
    }
}
fn make<T: Default>() -> T {
    T::default()
}
pub fn make_or_default<T: Default>(fallback: T) -> T {
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| make::<T>()))
        .unwrap_or(fallback);
    r
}
"#,
    );
    assert!(
        cpp.contains("T r = rusty::panic::catch_unwind_std(rusty::panic::AssertUnwindSafe([&]() { return recover(); }))"),
        "{cpp}"
    );
    assert!(!cpp.contains("R r ="), "{cpp}");
    assert!(
        cpp.contains("T r = rusty::panic::catch_unwind_std(rusty::panic::AssertUnwindSafe([&]() { return ::make<T>(); }))"),
        "{cpp}"
    );
}

#[test]
fn inherent_and_trait_method_one_signature_through_an_alias_is_one_member() {
    // SRPC's epoll backend: an inherent `close(&mut self, fd: i32)` and the
    // trait method `close(&mut self, fd: Fd)`, `type Fd = i32`, are one C++
    // signature; keyed by spelling both were members ("class member cannot
    // be redeclared"). The trait one forwards, so it adds nothing. A
    // same-named pair whose bodies differ keeps the inherent body on the
    // plain member (a direct call resolves there in Rust) whichever impl
    // comes first, and the trait body as `rusty_Backend_label`, which trait
    // dispatch reaches: compiled and run, 2117 as rustc prints.
    let cpp = translate(
        r#"
pub type Fd = i32;
pub trait Backend {
    fn close(&mut self, fd: Fd) -> u32;
    fn label(&self) -> u32;
}
pub struct Epoll {
    pub closed: u32,
}
impl Backend for Epoll {
    fn close(&mut self, fd: Fd) -> u32 {
        Epoll::close(self, fd)
    }
    fn label(&self) -> u32 {
        2000
    }
}
impl Epoll {
    pub fn close(&mut self, fd: i32) -> u32 {
        self.closed += fd as u32;
        self.closed
    }
    pub fn label(&self) -> u32 {
        100
    }
}
pub fn run() -> u32 {
    let mut e = Epoll { closed: 1 };
    let direct = e.close(2) + e.label();
    let mut b: Box<dyn Backend> = Box::new(Epoll { closed: 10 });
    direct + b.close(4) + b.label()
}
"#,
    );
    assert_eq!(cpp.matches("    uint32_t close(int32_t fd);").count(), 1, "{cpp}");
    assert!(!cpp.contains("    uint32_t close(Fd fd);"), "{cpp}");
    assert!(cpp.contains("uint32_t label() const;"), "{cpp}");
    assert!(cpp.contains("uint32_t rusty_Backend_label() const;"), "{cpp}");
    assert!(
        cpp.contains("uint32_t Epoll::label() const {\n    return static_cast<uint32_t>(100);"),
        "{cpp}"
    );
    assert!(
        cpp.contains(
            "uint32_t Epoll::rusty_Backend_label() const {\n    return static_cast<uint32_t>(2000);"
        ),
        "{cpp}"
    );
}

#[test]
fn callable_argument_output_deduces_its_type_parameter() {
    // lion-reactor's AsyncFdReadyGuard: `try_io<R>(self, f: impl
    // FnOnce(RawFd) -> io::Result<R>)` takes `R` from the closure's output,
    // which C++ does not deduce, so `guard.try_io(closure)` found no viable
    // member. A forwarding overload invokes the argument's type on the
    // bound's inputs and deduces `R` against `rusty::io::Result<R>`; a free
    // function gets the same, at namespace scope. A parameter C++ deduces
    // itself (`x: T`) needs none.
    let cpp = translate(
        r#"
use std::io;
pub type RawFd = i32;
pub struct ReadyGuard {
    fd: RawFd,
}
impl ReadyGuard {
    pub fn try_io<R>(self, f: impl FnOnce(RawFd) -> io::Result<R>) -> io::Result<R> {
        f(self.fd)
    }
}
pub fn with_fd<R, F>(fd: RawFd, f: F) -> Option<R>
where
    F: FnOnce(RawFd) -> Option<R>,
{
    f(fd)
}
pub fn keep<T>(x: T, f: impl FnOnce(T) -> T) -> T {
    f(x)
}
pub struct OnceCell<T> {
    pub v: Option<T>,
}
impl<T> OnceCell<T> {
    pub fn init(&self, f: impl FnOnce() -> T) -> u32 {
        let _ = f;
        0
    }
    pub fn try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> u32 {
        let _ = f;
        1
    }
}
pub enum Slot<T> {
    Full(T),
    Empty,
}
impl<T> Slot<T> {
    pub fn or_insert_with(self, default: impl FnOnce() -> T) -> T {
        match self {
            Slot::Full(v) => v,
            Slot::Empty => default(),
        }
    }
}
pub fn run(guard: ReadyGuard) -> u32 {
    let a = guard.try_io(|fd: RawFd| -> io::Result<u32> { Ok(fd as u32 + 1) }).unwrap_or(0);
    let b = with_fd(5, |fd: RawFd| -> Option<u32> { Some(fd as u32) }).unwrap_or(0);
    a + b + keep(1, |x| x + 1)
}
"#,
    );
    assert!(
        cpp.contains(
            "template<typename R> static std::type_identity<std::tuple<R>> __rusty_deduce_try_io_1(std::type_identity<rusty::io::Result<R>>);"
        ),
        "{cpp}"
    );
    assert!(
        cpp.contains(
            "template<typename __RustyF1, typename __RustyD1 = decltype(__rusty_deduce_try_io_1(std::type_identity<std::invoke_result_t<__RustyF1&, RawFd>>{}))>"
        ),
        "{cpp}"
    );
    assert!(cpp.contains("decltype(auto) try_io(__RustyF1&& f) {"), "{cpp}");
    assert!(
        cpp.contains(
            "return try_io<std::tuple_element_t<0, typename __RustyD1::type>>(std::forward<__RustyF1>(f));"
        ),
        "{cpp}"
    );
    assert!(cpp.contains("__rusty_deduce_with_fd_1(std::type_identity<rusty::Option<R>>);"), "{cpp}");
    assert!(!cpp.contains("__rusty_deduce_keep"), "{cpp}");
    // A struct's or enum's own parameter belongs to the class, not to the
    // member: no overload for `init` (once_cell's OnceCell::init had one,
    // ambiguous with the primary) or `or_insert_with` (hashbrown's Entry: a
    // helper `template<typename T>` shadowed the class's), and `try_init`
    // deduces only its own `E`.
    assert!(!cpp.contains("__rusty_deduce_init"), "{cpp}");
    assert!(!cpp.contains("__rusty_deduce_or_insert_with"), "{cpp}");
    assert!(
        cpp.contains(
            "template<typename E> static std::type_identity<std::tuple<E>> __rusty_deduce_try_init_1(std::type_identity<rusty::Result<T, E>>);"
        ),
        "{cpp}"
    );
}

#[test]
fn owned_scrutinee_payload_an_arm_uses_is_bound_mutably() {
    // SRPC's lion_fd_consume_ready: `match polled { Poll::Ready(Ok(ready)) =>
    // ready.try_io(..) }`. The guard, an owned scrutinee's nested payload the
    // arm uses by value (try_io takes `self`), was bound through
    // `std::as_const`, where a `self`-by-value member has no viable call. The
    // nested binding now peeks mutably (`rusty::detail::peek_unwrap`), and
    // the scrutinee local stays mutable. With the forwarding overload above,
    // compiled and run with a noop waker: 1084, as rustc prints.
    let cpp = translate(
        r#"
use std::io;
use std::task::{Context, Poll};
pub type RawFd = i32;
pub struct Fd {
    pub fd: RawFd,
    pub ready: std::cell::Cell<bool>,
}
pub struct ReadyGuard<'a> {
    fd: &'a Fd,
}
impl<'a> ReadyGuard<'a> {
    pub fn try_io<R>(self, f: impl FnOnce(RawFd) -> io::Result<R>) -> io::Result<R> {
        let result = f(self.fd.fd);
        if let Err(e) = &result {
            if e.kind() == io::ErrorKind::WouldBlock {
                self.fd.ready.set(false);
            }
        }
        result
    }
}
impl Fd {
    pub fn poll_read_ready(&self, _cx: &mut Context<'_>) -> Poll<io::Result<ReadyGuard<'_>>> {
        if self.ready.get() {
            Poll::Ready(Ok(ReadyGuard { fd: self }))
        } else {
            Poll::Pending
        }
    }
}
pub fn consume_ready(fd: &Fd, cx: &mut Context<'_>) -> bool {
    let polled = fd.poll_read_ready(cx);
    match polled {
        Poll::Ready(Ok(ready)) => {
            let _cleared = ready.try_io(|_fd: RawFd| -> io::Result<()> {
                Err(io::Error::from(io::ErrorKind::WouldBlock))
            });
            true
        }
        Poll::Ready(Err(_misuse)) => false,
        Poll::Pending => false,
    }
}
pub fn value_of(r: io::Result<u32>) -> u32 {
    let polled = Poll::Ready(r);
    match polled {
        Poll::Ready(Ok(n)) => n + 1,
        _ => 0,
    }
}
"#,
    );
    assert!(cpp.contains("    auto polled = fd.poll_read_ready(cx);"), "{cpp}");
    assert!(
        cpp.contains(
            "auto&& ready_bind_tmp = rusty::detail::peek_unwrap(rusty::detail::deref_if_pointer(_mv0));"
        ),
        "{cpp}"
    );
    assert!(!cpp.contains("ready_bind_tmp = std::as_const("), "{cpp}");
    // A payload the arm only reads keeps its const view, and its local
    // stays const.
    assert!(cpp.contains("auto&& n_bind_tmp = std::as_const("), "{cpp}");
    assert!(cpp.contains("const auto polled = rusty::Poll<"), "{cpp}");
}

#[test]
fn async_block_is_an_invoked_coroutine_lambda_owning_its_captures() {
    // `async move { .. }` / `async { .. }` had no lowering: the expression
    // fell to the generic fallback, `rusty::intrinsics::unreachable_panic()`,
    // and the slot manifest still said 0. It is an immediately-invoked
    // coroutine lambda whose explicit object parameter takes the closure by
    // value, so the frame owns the captures; `async move` moves what it
    // names (the local stays mutable to be moved from), `async` borrows, and
    // a unit block ends in `co_return;`. Compiled and run (also under ASan)
    // with rusty::block_on: 42 35 10, as rustc prints.
    let cpp = translate(
        r#"
use std::future::Future;
pub async fn add(a: u64, b: u64) -> u64 {
    a + b
}
pub fn make(x: u64) -> impl Future<Output = u64> {
    let boxed = Box::new(x);
    async move { add(*boxed, 1).await * 2 }
}
pub fn bump(counter: &std::cell::Cell<u64>) -> impl Future<Output = ()> + '_ {
    async {
        counter.set(counter.get() + 5);
    }
}
pub async fn chain(x: u64) -> u64 {
    let first = async move { add(x, 10).await };
    let a = first.await;
    let b = make(a).await;
    a + b
}
"#,
    );
    assert!(cpp.contains("    auto boxed = rusty::Box<uint64_t>::new_(std::move(x));"), "{cpp}");
    assert!(
        cpp.contains("return [=, boxed = std::move(boxed)](this auto) -> rusty::Task<uint64_t> {"),
        "{cpp}"
    );
    assert!(cpp.contains("return [&](this auto) -> rusty::Task<void> {"), "{cpp}");
    assert!(cpp.contains("counter.set(counter.get() + 5);\nco_return;\n}();"), "{cpp}");
    assert!(
        cpp.contains("auto first = [=, x = std::move(x)](this auto) -> rusty::Task<uint64_t> {"),
        "{cpp}"
    );
    assert!(!cpp.contains("unreachable_panic"), "{cpp}");
}

#[test]
fn unlowered_expression_fails_closed_with_a_hand_slot() {
    // An async block whose output nothing types, and an expression kind the
    // emitter has no lowering for (a `loop` as a call argument), became a
    // silent `unreachable_panic()`. Both now carry a `TODO transpiler` marker,
    // which the slot manifest counts.
    let cpp = translate(
        r#"
use std::future::Future;
fn mystery<T: Default>() -> T {
    T::default()
}
pub fn spawn_unknown() -> impl Future<Output = u32> {
    let f = async { mystery() };
    f
}
fn id(x: u32) -> u32 {
    x
}
pub fn looped(n: u32) -> u32 {
    let mut i = 0;
    id(loop {
        i += 1;
        if i > n {
            break i;
        }
    })
}
"#,
    );
    assert!(
        cpp.contains("auto f = /* TODO transpiler: async block whose output type is unknown */"),
        "{cpp}"
    );
    assert!(
        cpp.contains("::id(/* TODO transpiler: unlowered loop expression */ rusty::intrinsics::unreachable_panic())"),
        "{cpp}"
    );
    assert_eq!(crate::slots::detect_slots("t.cppm", &cpp).len(), 2, "{cpp}");
}

#[test]
fn reference_map_closure_return_does_not_leak_onto_a_later_closure() {
    // lion-reactor: `get_slot_mut`'s `.map(|w| &mut w.inner)` forces
    // `-> decltype(auto)` on ITS lambda (an `Option<&mut T>` payload), but the
    // pending flag was not tied to that closure: a codegen copy made before
    // the closure consumed it handed it to later closures, among them
    // `Reactor::enter`'s `NEXT_EPOCH.with(|n| { let e = n.get(); ..; e })`,
    // which then returned `std::move(e)` as `uint64_t&&`, a reference to its
    // own local. Every Lion I/O registration read a garbage epoch at -O2 and
    // reported "no Lion reactor". Compiled at -O2 with
    // -Werror=return-stack-address and run: 261102, as rustc prints
    // (2619d788..dc6e7558: 1).
    let cpp = translate(
        r#"
use std::cell::Cell;
use std::collections::HashMap;
pub struct Wrapper {
    pub inner: u64,
}
pub struct Slab {
    pub inner: HashMap<u64, Wrapper>,
}
impl Slab {
    fn get_slot_mut(&mut self, key: u64) -> Option<&mut u64> {
        self.inner.get_mut(&key).map(|w| &mut w.inner)
    }
    pub fn bump(&mut self, key: u64) -> bool {
        match self.get_slot_mut(key) {
            Some(v) => {
                *v += 1;
                true
            }
            None => false,
        }
    }
}
thread_local! {
    static NEXT_EPOCH: Cell<u64> = const { Cell::new(1) };
}
pub struct Reactor {
    pub id: u64,
}
impl Reactor {
    pub fn enter(&mut self) -> u64 {
        let epoch = NEXT_EPOCH.with(|n| {
            let e = n.get();
            n.set(e + 1);
            e
        });
        epoch + self.id
    }
}
"#,
    );
    assert!(cpp.contains("return this->inner.get(key).map([&](auto&& w) -> decltype(auto) {"), "{cpp}");
    assert!(cpp.contains("NEXT_EPOCH.with([&](auto&& n) {\n"), "{cpp}");
    assert!(!cpp.contains("NEXT_EPOCH.with([&](auto&& n) -> decltype(auto)"), "{cpp}");
}

#[test]
fn decltype_auto_scope_returns_a_bare_local_without_moving_it() {
    // `decltype(std::move(e))` is `T&&`: from a `decltype(auto)` function or
    // lambda it returns a reference to the dying local. `return e;` deduces
    // `T` (or keeps a reference binding's reference) and still moves.
    let mut cg = CodeGen::new();
    cg.push_return_value_scope("decltype(auto)");
    assert_eq!(cg.decltype_auto_safe_return_value("std::move(e)".to_string()), "e");
    assert_eq!(
        cg.decltype_auto_safe_return_value("std::move(a.b)".to_string()),
        "std::move(a.b)"
    );
    cg.push_return_value_scope("auto");
    assert_eq!(cg.decltype_auto_safe_return_value("std::move(e)".to_string()), "std::move(e)");
}

#[test]
fn waker_field_keeps_its_holders_send_and_sync() {
    // std::task::Waker is Send + Sync, but the auto-trait table had no row
    // for it: SRPC's PollDriverWake (a Waker field) derived no `is_send` /
    // `is_sync`, so PollThread, Client, ClientConnection and ClientPool lost
    // both in C++ and SRPC's importer static_asserts failed.
    let cpp = translate(
        r#"
use std::task::Waker;
pub struct PollDriverWake {
    pub waker: Waker,
    pub id: u64,
}
pub struct Qualified {
    pub waker: Option<std::task::Waker>,
}
"#,
    );
    assert_eq!(
        cpp.matches("    static constexpr bool is_send = true;\n    static constexpr bool is_sync = true;")
            .count(),
        2,
        "{cpp}"
    );
}
