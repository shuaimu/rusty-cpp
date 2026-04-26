#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
use std::time::{Duration, Instant};
extern crate test;
#[rustc_test_marker = "basic"]
#[doc(hidden)]
pub const basic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("basic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/basic.rs",
        start_line: 4usize,
        start_col: 4usize,
        end_line: 4usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(basic())),
};
fn basic() {
    let make_fut = || std::future::ready(42);
    match (&pollster::block_on(make_fut()), &42) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let then = Instant::now();
    pollster::block_on(futures_timer::Delay::new(Duration::from_millis(250)));
    if !(Instant::now().duration_since(then) > Duration::from_millis(250)) {
        ::core::panicking::panic(
            "assertion failed: Instant::now().duration_since(then) > Duration::from_millis(250)",
        )
    }
}
extern crate test;
#[rustc_test_marker = "mpsc"]
#[doc(hidden)]
pub const mpsc: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("mpsc"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/basic.rs",
        start_line: 17usize,
        start_col: 4usize,
        end_line: 17usize,
        end_col: 8usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(mpsc())),
};
fn mpsc() {
    use std::{
        sync::atomic::{AtomicUsize, Ordering::SeqCst},
        thread,
    };
    use tokio::sync::mpsc;
    const BOUNDED: usize = 16;
    const MESSAGES: usize = 100_000;
    let (a_tx, mut a_rx) = mpsc::channel(BOUNDED);
    let (b_tx, mut b_rx) = mpsc::channel(BOUNDED);
    let thread_a = thread::spawn(move || {
        pollster::block_on(async {
            while let Some(msg) = a_rx.recv().await {
                b_tx.send(msg).await.expect("send on b");
            }
        });
    });
    let thread_b = thread::spawn(move || {
        pollster::block_on(async move {
            for _ in 0..MESSAGES {
                a_tx.send(()).await.expect("Send on a");
            }
        });
    });
    pollster::block_on(async move {
        let sum = AtomicUsize::new(0);
        while sum.fetch_add(1, SeqCst) < MESSAGES {
            b_rx.recv().await;
        }
        match (&sum.load(SeqCst), &(MESSAGES + 1)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    });
    thread_a.join().expect("join thread_a");
    thread_b.join().expect("join thread_b");
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&basic, &mpsc])
}
