use super::*;

fn translate(source: &str) -> String {
    let mut generator = CodeGen::new();
    generator.emit_file(&syn::parse_str(source).unwrap(), None);
    generator.into_output()
}

#[test]
fn standard_future_alias_outputs_and_poll_patterns() {
    let cpp = translate(
        r#"
        use std::future::Future;
        use std::pin::Pin as Pinned;
        use std::task::{Poll as State, Context};
        type FutureBox<'a, T> = Pinned<Box<dyn Future<Output = T> + Send + Sync + 'a>>;
        async fn work() -> i32 { 42 }
        fn boxed() -> FutureBox<'static, i32> { Box::pin(work()) }
        fn unit() -> State<()> { State::Ready(()) }
        fn pending<T>() -> State<T> { State::Pending }
        fn take<T>(state: State<T>) -> Option<T> {
            match state { State::Ready(value) => Some(value), State::Pending => None }
        }
        fn borrowed(state: &mut State<i32>) -> i32 {
            if let State::Ready(value) = state { *value += 1; *value } else { 0 }
        }
        fn poll<T>(future: &mut FutureBox<'_, T>, cx: &mut Context<'_>) -> State<T> {
            future.as_mut().poll(cx)
        }
    "#,
    );
    assert!(cpp.contains("rusty::Task<int32_t> boxed()"), "{cpp}");
    assert!(
        cpp.contains("rusty::future::pin<int32_t>(::work())"),
        "{cpp}"
    );
    assert!(cpp.contains("rusty::Poll<void>::ready_with()"), "{cpp}");
    assert!(cpp.contains("rusty::Poll<T>::pending()"), "{cpp}");
    assert!(
        cpp.contains(".is_ready()") && cpp.contains(".is_pending()"),
        "{cpp}"
    );
    assert!(cpp.contains("_poll_iflet.as_mut().unwrap()"), "{cpp}");
    assert!(
        !cpp.contains("Poll_Ready") && !cpp.contains("Poll_Pending"),
        "{cpp}"
    );
    assert!(cpp.contains("((future)).poll(cx)"), "{cpp}");
}

#[test]
fn standard_future_wake_receivers_retain_the_arc_parameter() {
    let cpp = translate(
        r#"
        use std::task::{Wake as Notify, Waker, Context};
        use std::sync::Arc;
        struct Signal { value: std::sync::atomic::AtomicUsize }
        impl Notify for Signal {
            fn wake(self: Arc<Self>) { self.value.fetch_add(Arc::strong_count(&self), std::sync::atomic::Ordering::Relaxed); }
            fn wake_by_ref(self: &Arc<Self>) { self.value.fetch_add(10, std::sync::atomic::Ordering::Relaxed); }
        }
        fn make(signal: Arc<Signal>) -> Waker { Waker::from(signal) }
        fn retained(cx: &Context<'_>) -> Waker { cx.waker().clone() }
        fn borrowed(waker: &Waker) { waker.wake_by_ref(); }
        fn owned(waker: Waker) { waker.wake(); }
    "#,
    );
    assert!(
        cpp.contains("static void wake(rusty::Arc<Signal> _wake_self)"),
        "{cpp}"
    );
    assert!(
        cpp.contains("static void wake_by_ref(const rusty::Arc<Signal>& _wake_self)"),
        "{cpp}"
    );
    assert!(cpp.contains("strong_count(_wake_self)"), "{cpp}");
    assert!(
        cpp.contains("rusty::Waker::from_arc(std::move(signal))"),
        "{cpp}"
    );
    assert!(cpp.contains("rusty::Waker((*(cx).waker))"), "{cpp}");
    assert!(cpp.contains("std::move(waker).wake()"), "{cpp}");
}

#[test]
fn standard_future_local_task_names_do_not_acquire_runtime_lowering() {
    let cpp = translate(
        r#"
        mod std {
            pub mod task {
                pub enum Poll<T> { Ready(T), Pending }
                pub trait Wake { fn wake(self); }
            }
        }
        struct Signal;
        impl std::task::Wake for Signal { fn wake(self) {} }
        fn local(value: i32) -> std::task::Poll<i32> { std::task::Poll::Ready(value) }
        fn external() -> ::std::task::Poll<()> { ::std::task::Poll::Ready(()) }
    "#,
    );
    assert!(!cpp.contains("_wake_self"), "{cpp}");
    assert!(cpp.contains("std_mod::task::Poll_Ready"), "{cpp}");
    assert!(cpp.contains("rusty::Poll<void>::ready_with()"), "{cpp}");
}

#[test]
#[should_panic(expected = "unsupported standard pinned Future")]
fn standard_future_extra_trait_bounds_fail_closed() {
    translate(
        r#"
        use std::future::Future;
        use std::pin::Pin;
        trait Other {}
        struct Unsupported { future: Pin<Box<dyn Future<Output = i32> + Other>> }
    "#,
    );
}

#[test]
#[should_panic(expected = "unsupported standard pinned Future")]
fn standard_future_higher_ranked_bounds_fail_closed() {
    translate(
        r#"
        use std::future::Future;
        use std::pin::Pin;
        struct Unsupported { future: Pin<Box<dyn for<'a> Future<Output = &'a i32>>> }
    "#,
    );
}

#[test]
fn standard_future_imported_poll_variants_and_core_waker() {
    let cpp = translate(r#"
        use core::task::Poll::{Pending as Waiting, Ready as Done};
        fn ready() -> core::task::Poll<i32> { Done(4) }
        fn pending() -> core::task::Poll<i32> { Waiting }
        fn extract(p: core::task::Poll<i32>) -> i32 {
            match p { Done(value) => value, Waiting => 0 }
        }
        fn clone(w: &core::task::Waker) -> core::task::Waker { w.clone() }
    "#);
    assert!(cpp.contains("rusty::Poll<int32_t>::ready_with(4)"), "{cpp}");
    assert!(cpp.contains("rusty::Poll<int32_t>::pending()"), "{cpp}");
    assert!(cpp.contains(".is_pending()"), "{cpp}");
    assert!(cpp.contains("rusty::Waker(w)"), "{cpp}");
}
