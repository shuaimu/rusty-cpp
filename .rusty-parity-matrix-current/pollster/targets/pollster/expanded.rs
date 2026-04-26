#![feature(prelude_import)]
//!# Pollster
//!
//!Pollster is an incredibly minimal async executor for Rust that lets you block a thread until a future completes.
//!
//![![Cargo](https://img.shields.io/crates/v/pollster.svg)](
//!https://crates.io/crates/pollster)
//![![Documentation](https://docs.rs/pollster/badge.svg)](
//!https://docs.rs/pollster)
//![![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
//!https://github.com/zesterer/pollster)
//![![Actions](https://github.com/zesterer/pollster/actions/workflows/rust.yml/badge.svg)](https://github.com/zesterer/pollster/actions/workflows/rust.yml)
//!
//!```rust
//!use pollster::FutureExt as _;
//!
//!let my_fut = async {};
//!
//!let result = my_fut.block_on();
//!```
//!
//!That's it. That's all it does. Nothing more, nothing less. No need to pull in 50 crates to evaluate a future.
//!
//!## Why is this useful?
//!
//!Now that `async` functions are stable, we're increasingly seeing libraries all over the Rust ecosystem expose `async`
//!APIs. This is great for those wanting to build highly concurrent web applications!
//!
//!However, many of us are *not* building highly concurrent web applications, but end up faced with an `async` function
//!that we can't easily call from synchronous code. If you're in this position, then `pollster` is for you: it allows you
//!to evaluate a future in-place without spinning up a heavyweight runtime like `tokio` or `async_std`.
//!
//!## Minimalism
//!
//!Pollster is built with the [UNIX ethos](https://en.wikipedia.org/wiki/Unix_philosophy#Do_One_Thing_and_Do_It_Well) in
//!mind: do one thing, and do it well. It has no dependencies, compiles quickly, and is composed of only ~100 lines of
//!well-audited code.
//!
//!## Behaviour
//!
//!Pollster will synchronously block the thread until a future completes. It will not spin: instead, it will place the
//!thread into a waiting state until the future has been polled to completion.
//!
//!## Compatibility
//!
//!Unfortunately, `pollster` will not work for *all* futures because some require a specific runtime or reactor. See
//![here](https://rust-lang.github.io/async-book/08_ecosystem/00_chapter.html#determining-ecosystem-compatibility) for more
//!information about when and where `pollster` may be used. However, if you're already pulling in the required dependencies
//!to create such a future in the first place, it's likely that you already have a version of `block_on` in your dependency
//!tree that's designed to poll your future, so use that instead.
//!
//!## Macro
//!
//!When using the `macro` crate feature, an attribute-macro can be used to mark `async fn main()`:
//!```rust,ignore
//!#[pollster::main]
//!async fn main() {
//!    let my_fut = async {};
//!
//!    my_fut.await;
//!}
//!```
//!
//!Additionally if you have re-exported the crate with a different name then `pollster`, you have to specify it:
//!```rust,ignore
//!#[pollster::main(crate = renamed_pollster)]
//!async fn main() {
//!    let my_fut = async {};
//!
//!    my_fut.await;
//!}
//!```
//!
//!You can also use `#[pollster::test]` for tests.
//!
//!## Comparison with `futures::executor::block_on`
//!
//!`pollster` does approximately the same thing as the `block_on` function from the `futures` crate. If you already have `futures` in your dependency tree, you might as well use it instead. `pollster` is primarily for applications that don't care to pull all of `futures` or another runtime like `tokio` into their dependency tree for the sake of evaluating simple futures.
//!
//!## Minimum Supported Rust Version (MSRV) Policy
//!
//!Current MSRV: `1.69.0`
//!
//!`pollster` has a policy of supporting compiler versions that are at least 18 months old. The crate *may* compile with
//!older compilers, but this is not guaranteed.
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;

use std::{
    future::{Future, IntoFuture},
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};

// A local reusable waker for each thread.

// A signal used to wake up the thread for polling as the future moves to completion.
// Create a context to be passed to the future.

// Poll the future to completion.
const LOCAL_WAKER: ::std::thread::LocalKey<Waker> = {
    #[inline]
    fn __rust_std_internal_init_fn() -> Waker {
        {
            let signal = Arc::new(Signal {
                owning_thread: thread::current(),
            });
            Waker::from(signal)
        }
    }
    unsafe {
        ::std::thread::LocalKey::new(
            const {
                if ::std::mem::needs_drop::<Waker>() {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            Waker,
                            (),
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                    }
                } else {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            Waker,
                            !,
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                    }
                }
            },
        )
    }
};
/// An extension trait that allows blocking on a future in suffix position.
pub trait FutureExt: Future {
    /// Block the thread until the future is ready.
    ///
    /// # Example
    ///
    /// ```
    /// use pollster::FutureExt as _;
    ///
    /// let my_fut = async {};
    ///
    /// let result = my_fut.block_on();
    /// ```
    fn block_on(self) -> Self::Output
    where
        Self: Sized,
    {
        block_on(self)
    }
}
impl<F: Future> FutureExt for F {}
struct Signal {
    /// The thread that owns the signal.
    owning_thread: thread::Thread,
}
impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        self.owning_thread.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.owning_thread.unpark();
    }
}
/// Block the thread until the future is ready.
///
/// # Example
///
/// ```
/// let my_fut = async {};
/// let result = pollster::block_on(my_fut);
/// ```
pub fn block_on<F: IntoFuture>(fut: F) -> F::Output {
    let mut fut = {
        super let mut pinned = fut.into_future();
        unsafe { ::core::pin::Pin::new_unchecked(&mut pinned) }
    };
    LOCAL_WAKER.with(|waker| {
        let mut context = Context::from_waker(waker);
        loop {
            match fut.as_mut().poll(&mut context) {
                Poll::Pending => thread::park(),
                Poll::Ready(item) => break item,
            }
        }
    })
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
