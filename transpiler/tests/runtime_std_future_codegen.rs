use std::path::Path;
use std::process::Command;

#[test]
fn standard_future_source_runs_with_rust_and_cpp_ownership() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("standard_future.rs");
    let generated = directory.path().join("standard_future.cpp");
    std::fs::write(&source, SOURCE).unwrap();
    let translated = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"))
        .arg(&source)
        .arg("-o")
        .arg(&generated)
        .output()
        .unwrap();
    assert!(
        translated.status.success(),
        "{}",
        String::from_utf8_lossy(&translated.stderr)
    );
    let mut cpp = std::fs::read_to_string(&generated).unwrap();
    assert!(cpp.contains("rusty::Task<rusty::Box<int32_t>>"), "{cpp}");
    assert!(
        cpp.contains("static void wake(rusty::Arc<Signal> _wake_self)"),
        "{cpp}"
    );
    cpp.push_str("\nint main() { return check_waker() + check_future() + check_patterns(); }\n");
    std::fs::write(&generated, cpp).unwrap();
    let compiler = std::env::var("CXX").unwrap_or_else(|_| "clang++".to_string());
    let cpp_binary = directory.path().join("cpp_check");
    let compiled = Command::new(compiler)
        .args([
            "-std=c++23",
            "-DRUSTY_PORTABLE_INTRINSICS=1",
            "-pthread",
            "-I",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../include"))
        .arg(&generated)
        .arg("-o")
        .arg(&cpp_binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(cpp_binary).output().unwrap();
    assert!(
        ran.status.success(),
        "C++ status: {}\n{}",
        ran.status,
        String::from_utf8_lossy(&ran.stderr)
    );

    std::fs::write(&source, format!("{SOURCE}\nfn main() {{ assert_eq!(check_waker(), 0); assert_eq!(check_future(), 0); assert_eq!(check_patterns(), 0); }}\n")).unwrap();
    let rust_binary = directory.path().join("rust_check");
    let compiled = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source)
        .arg("-o")
        .arg(&rust_binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(rust_binary).output().unwrap();
    assert!(
        ran.status.success(),
        "Rust status: {}\n{}",
        ran.status,
        String::from_utf8_lossy(&ran.stderr)
    );
}

const SOURCE: &str = r#"
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

pub struct Signal { count: AtomicUsize }
impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.count.fetch_add(Arc::strong_count(&self), Ordering::Relaxed);
    }
    fn wake_by_ref(self: &Arc<Self>) { self.count.fetch_add(10, Ordering::Relaxed); }
}
pub struct ForwardSignal { count: AtomicUsize }
impl Wake for ForwardSignal {
    fn wake(self: Arc<Self>) { self.wake_by_ref(); }
    fn wake_by_ref(self: &Arc<Self>) { self.count.fetch_add(1, Ordering::Relaxed); }
}
pub fn check_waker() -> i32 {
    let signal = Arc::new(Signal { count: AtomicUsize::new(0) });
    let waker = Waker::from(signal.clone());
    if Arc::strong_count(&signal) != 2 { return 1; }
    let clone = waker.clone();
    if Arc::strong_count(&signal) != 3 { return 2; }
    clone.wake();
    if signal.count.load(Ordering::Relaxed) != 3 { return 3; }
    if Arc::strong_count(&signal) != 2 { return 4; }
    waker.wake_by_ref();
    if signal.count.load(Ordering::Relaxed) != 13 { return 5; }
    drop(waker);
    if Arc::strong_count(&signal) != 1 { return 6; }
    let forwarded = Arc::new(ForwardSignal { count: AtomicUsize::new(0) });
    forwarded.clone().wake();
    Wake::wake_by_ref(&forwarded);
    if forwarded.count.load(Ordering::Relaxed) != 2 { return 7; }
    0
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;
pub struct Deferred { polled: bool, retained: Arc<RefCell<Option<Waker>>> }
impl Future for Deferred {
    type Output = Box<i32>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Box<i32>> {
        if !self.polled {
            self.polled = true;
            self.retained.replace(Some(cx.waker().clone()));
            return Poll::Pending;
        }
        Poll::Ready(Box::new(42))
    }
}
pub async fn immediate() -> i32 { 7 }
pub fn boxed_immediate() -> BoxFuture<i32> { Box::pin(immediate()) }
pub fn complete<T, F: FnMut(T)>(mut task: Pin<Box<dyn Future<Output = T>>>, mut callback: F, cx: &mut Context<'_>) {
    let result = { task.as_mut().poll(cx) };
    if let Poll::Ready(value) = result { callback(value); }
}
pub fn check_future() -> i32 {
    let signal = Arc::new(Signal { count: AtomicUsize::new(0) });
    let waker = Waker::from(signal.clone());
    let retained: Arc<RefCell<Option<Waker>>> = Arc::new(RefCell::new(None));
    let mut future: BoxFuture<Box<i32>> = Box::pin(Deferred { polled: false, retained: retained.clone() });
    {
        let mut context = Context::from_waker(&waker);
        if future.as_mut().poll(&mut context).is_ready() { return 11; }
    }
    drop(future);
    let saved = retained.replace(None).unwrap();
    saved.wake_by_ref();
    if signal.count.load(Ordering::Relaxed) != 10 { return 12; }
    let mut future: BoxFuture<Box<i32>> = Box::pin(Deferred { polled: true, retained });
    let mut context = Context::from_waker(&waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => { if *value != 42 { return 13; } },
        Poll::Pending => return 14,
    }
    let mut immediate = boxed_immediate();
    match immediate.as_mut().poll(&mut context) {
        Poll::Ready(value) => { if value != 7 { return 15; } },
        Poll::Pending => return 16,
    }
    let mut result = 0;
    complete(Box::pin(Deferred { polled: true, retained: Arc::new(RefCell::new(None)) }),
        |value| { result = *value; }, &mut context);
    if result != 42 { return 17; }
    0
}
pub fn take<T>(poll: Poll<T>) -> Option<T> {
    match poll { Poll::Ready(value) => Some(value), Poll::Pending => None }
}
pub fn increment(poll: &mut Poll<i32>) {
    if let Poll::Ready(value) = poll { *value += 1; }
}
pub fn check_patterns() -> i32 {
    let mut poll = Poll::Ready(41);
    increment(&mut poll);
    if take(poll).unwrap() != 42 { return 21; }
    let owned: Poll<Box<i32>> = Poll::Ready(Box::new(7));
    if *take(owned).unwrap() != 7 { return 22; }
    let pending: Poll<Box<i32>> = Poll::Pending;
    if take(pending).is_some() { return 23; }
    let unit: Poll<()> = Poll::Ready(());
    if !unit.is_ready() { return 24; }
    0
}
"#;
