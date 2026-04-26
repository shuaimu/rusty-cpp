#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
mod unsync_once_cell {
    use core::{cell::Cell, sync::atomic::{AtomicUsize, Ordering::SeqCst}};
    use once_cell::unsync::OnceCell;
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::once_cell"]
    #[doc(hidden)]
    pub const once_cell: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::once_cell"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 9usize,
            start_col: 4usize,
            end_line: 9usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell()),
        ),
    };
    fn once_cell() {
        let c = OnceCell::new();
        if !c.get().is_none() {
            ::core::panicking::panic("assertion failed: c.get().is_none()")
        }
        c.get_or_init(|| 92);
        match (&c.get(), &Some(&92)) {
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
        c.get_or_init(|| {
            ::core::panicking::panic_fmt(format_args!("Kabom!"));
        });
        match (&c.get(), &Some(&92)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::once_cell_with_value"]
    #[doc(hidden)]
    pub const once_cell_with_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::once_cell_with_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 20usize,
            start_col: 4usize,
            end_line: 20usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_with_value()),
        ),
    };
    fn once_cell_with_value() {
        const CELL: OnceCell<i32> = OnceCell::with_value(12);
        let cell = CELL;
        match (&cell.get(), &Some(&12)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::once_cell_get_mut"]
    #[doc(hidden)]
    pub const once_cell_get_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::once_cell_get_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 27usize,
            start_col: 4usize,
            end_line: 27usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_get_mut()),
        ),
    };
    fn once_cell_get_mut() {
        let mut c = OnceCell::new();
        if !c.get_mut().is_none() {
            ::core::panicking::panic("assertion failed: c.get_mut().is_none()")
        }
        c.set(90).unwrap();
        *c.get_mut().unwrap() += 2;
        match (&c.get_mut(), &Some(&mut 92)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::once_cell_drop"]
    #[doc(hidden)]
    pub const once_cell_drop: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::once_cell_drop"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 36usize,
            start_col: 4usize,
            end_line: 36usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_drop()),
        ),
    };
    fn once_cell_drop() {
        static DROP_CNT: AtomicUsize = AtomicUsize::new(0);
        struct Dropper;
        impl Drop for Dropper {
            fn drop(&mut self) {
                DROP_CNT.fetch_add(1, SeqCst);
            }
        }
        let x = OnceCell::new();
        x.get_or_init(|| Dropper);
        match (&DROP_CNT.load(SeqCst), &0) {
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
        drop(x);
        match (&DROP_CNT.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::once_cell_drop_empty"]
    #[doc(hidden)]
    pub const once_cell_drop_empty: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::once_cell_drop_empty"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 53usize,
            start_col: 4usize,
            end_line: 53usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_drop_empty()),
        ),
    };
    fn once_cell_drop_empty() {
        let x = OnceCell::<String>::new();
        drop(x);
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::clone"]
    #[doc(hidden)]
    pub const clone: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::clone"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 59usize,
            start_col: 4usize,
            end_line: 59usize,
            end_col: 9usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(clone())),
    };
    fn clone() {
        let s = OnceCell::new();
        let c = s.clone();
        if !c.get().is_none() {
            ::core::panicking::panic("assertion failed: c.get().is_none()")
        }
        s.set("hello".to_string()).unwrap();
        let c = s.clone();
        match (&c.get().map(String::as_str), &Some("hello")) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::get_or_try_init"]
    #[doc(hidden)]
    pub const get_or_try_init: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::get_or_try_init"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 70usize,
            start_col: 4usize,
            end_line: 70usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(get_or_try_init()),
        ),
    };
    fn get_or_try_init() {
        let cell: OnceCell<String> = OnceCell::new();
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
        let res = std::panic::catch_unwind(|| {
            cell
                .get_or_try_init(|| -> Result<_, ()> {
                    ::core::panicking::panic("explicit panic")
                })
        });
        if !res.is_err() {
            ::core::panicking::panic("assertion failed: res.is_err()")
        }
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
        match (&cell.get_or_try_init(|| Err(())), &Err(())) {
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
        match (
            &cell.get_or_try_init(|| Ok::<_, ()>("hello".to_string())),
            &Ok(&"hello".to_string()),
        ) {
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
        match (&cell.get(), &Some(&"hello".to_string())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::from_impl"]
    #[doc(hidden)]
    pub const from_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::from_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 85usize,
            start_col: 4usize,
            end_line: 85usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(from_impl()),
        ),
    };
    fn from_impl() {
        match (&OnceCell::from("value").get(), &Some(&"value")) {
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
        match (&OnceCell::from("foo").get(), &Some(&"bar")) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = ::core::panicking::AssertKind::Ne;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::partialeq_impl"]
    #[doc(hidden)]
    pub const partialeq_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::partialeq_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 91usize,
            start_col: 4usize,
            end_line: 91usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(partialeq_impl()),
        ),
    };
    fn partialeq_impl() {
        if !(OnceCell::from("value") == OnceCell::from("value")) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::from(\"value\") == OnceCell::from(\"value\")",
            )
        }
        if !(OnceCell::from("foo") != OnceCell::from("bar")) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::from(\"foo\") != OnceCell::from(\"bar\")",
            )
        }
        if !(OnceCell::<String>::new() == OnceCell::new()) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::<String>::new() == OnceCell::new()",
            )
        }
        if !(OnceCell::<String>::new() != OnceCell::from("value".to_owned())) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::<String>::new() != OnceCell::from(\"value\".to_owned())",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::into_inner"]
    #[doc(hidden)]
    pub const into_inner: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::into_inner"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 100usize,
            start_col: 4usize,
            end_line: 100usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(into_inner()),
        ),
    };
    fn into_inner() {
        let cell: OnceCell<String> = OnceCell::new();
        match (&cell.into_inner(), &None) {
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
        let cell = OnceCell::new();
        cell.set("hello".to_string()).unwrap();
        match (&cell.into_inner(), &Some("hello".to_string())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::debug_impl"]
    #[doc(hidden)]
    pub const debug_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::debug_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 109usize,
            start_col: 4usize,
            end_line: 109usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(debug_impl()),
        ),
    };
    fn debug_impl() {
        let cell = OnceCell::new();
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0:#?}", cell))
            }),
            &"OnceCell(Uninit)",
        ) {
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
        cell.set(<[_]>::into_vec(::alloc::boxed::box_new(["hello", "world"]))).unwrap();
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0:#?}", cell))
            }),
            &r#"OnceCell(
    [
        "hello",
        "world",
    ],
)"#,
        ) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::reentrant_init"]
    #[doc(hidden)]
    pub const reentrant_init: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::reentrant_init"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 126usize,
            start_col: 4usize,
            end_line: 126usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::YesWithMessage("reentrant init"),
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(reentrant_init()),
        ),
    };
    #[should_panic(expected = "reentrant init")]
    fn reentrant_init() {
        let x: OnceCell<Box<i32>> = OnceCell::new();
        let dangling_ref: Cell<Option<&i32>> = Cell::new(None);
        x.get_or_init(|| {
            let r = x.get_or_init(|| Box::new(92));
            dangling_ref.set(Some(r));
            Box::new(62)
        });
        {
            ::std::io::_eprint(
                format_args!("use after free: {0:?}\n", dangling_ref.get().unwrap()),
            );
        };
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::aliasing_in_get"]
    #[doc(hidden)]
    pub const aliasing_in_get: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::aliasing_in_get"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 138usize,
            start_col: 4usize,
            end_line: 138usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(aliasing_in_get()),
        ),
    };
    fn aliasing_in_get() {
        let x = OnceCell::new();
        x.set(42).unwrap();
        let at_x = x.get().unwrap();
        let _ = x.set(27);
        {
            ::std::io::_print(format_args!("{0}\n", at_x));
        };
    }
    extern crate test;
    #[rustc_test_marker = "unsync_once_cell::arrrrrrrrrrrrrrrrrrrrrr"]
    #[doc(hidden)]
    pub const arrrrrrrrrrrrrrrrrrrrrr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_once_cell::arrrrrrrrrrrrrrrrrrrrrr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_once_cell.rs",
            start_line: 148usize,
            start_col: 4usize,
            end_line: 148usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(arrrrrrrrrrrrrrrrrrrrrr()),
        ),
    };
    fn arrrrrrrrrrrrrrrrrrrrrr() {
        let cell = OnceCell::new();
        {
            let s = String::new();
            cell.set(&s).unwrap();
        }
    }
}
mod sync_once_cell {
    use std::{
        sync::atomic::{AtomicUsize, Ordering::SeqCst},
        thread::scope,
    };
    use std::sync::Barrier;
    use once_cell::sync::{Lazy, OnceCell};
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell"]
    #[doc(hidden)]
    pub const once_cell: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 15usize,
            start_col: 4usize,
            end_line: 15usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell()),
        ),
    };
    fn once_cell() {
        let c = OnceCell::new();
        if !c.get().is_none() {
            ::core::panicking::panic("assertion failed: c.get().is_none()")
        }
        scope(|s| {
            s.spawn(|| {
                c.get_or_init(|| 92);
                match (&c.get(), &Some(&92)) {
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
        });
        c.get_or_init(|| {
            ::core::panicking::panic_fmt(format_args!("Kabom!"));
        });
        match (&c.get(), &Some(&92)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_with_value"]
    #[doc(hidden)]
    pub const once_cell_with_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_with_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 29usize,
            start_col: 4usize,
            end_line: 29usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_with_value()),
        ),
    };
    fn once_cell_with_value() {
        static CELL: OnceCell<i32> = OnceCell::with_value(12);
        match (&CELL.get(), &Some(&12)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_get_mut"]
    #[doc(hidden)]
    pub const once_cell_get_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_get_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 35usize,
            start_col: 4usize,
            end_line: 35usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_get_mut()),
        ),
    };
    fn once_cell_get_mut() {
        let mut c = OnceCell::new();
        if !c.get_mut().is_none() {
            ::core::panicking::panic("assertion failed: c.get_mut().is_none()")
        }
        c.set(90).unwrap();
        *c.get_mut().unwrap() += 2;
        match (&c.get_mut(), &Some(&mut 92)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_get_unchecked"]
    #[doc(hidden)]
    pub const once_cell_get_unchecked: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_get_unchecked"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 44usize,
            start_col: 4usize,
            end_line: 44usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_get_unchecked()),
        ),
    };
    fn once_cell_get_unchecked() {
        let c = OnceCell::new();
        c.set(92).unwrap();
        unsafe {
            match (&c.get_unchecked(), &&92) {
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
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_drop"]
    #[doc(hidden)]
    pub const once_cell_drop: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_drop"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 53usize,
            start_col: 4usize,
            end_line: 53usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_drop()),
        ),
    };
    fn once_cell_drop() {
        static DROP_CNT: AtomicUsize = AtomicUsize::new(0);
        struct Dropper;
        impl Drop for Dropper {
            fn drop(&mut self) {
                DROP_CNT.fetch_add(1, SeqCst);
            }
        }
        let x = OnceCell::new();
        scope(|s| {
            s.spawn(|| {
                x.get_or_init(|| Dropper);
                match (&DROP_CNT.load(SeqCst), &0) {
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
                drop(x);
            });
        });
        match (&DROP_CNT.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_drop_empty"]
    #[doc(hidden)]
    pub const once_cell_drop_empty: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_drop_empty"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 74usize,
            start_col: 4usize,
            end_line: 74usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_drop_empty()),
        ),
    };
    fn once_cell_drop_empty() {
        let x = OnceCell::<String>::new();
        drop(x);
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::clone"]
    #[doc(hidden)]
    pub const clone: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::clone"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 80usize,
            start_col: 4usize,
            end_line: 80usize,
            end_col: 9usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(clone())),
    };
    fn clone() {
        let s = OnceCell::new();
        let c = s.clone();
        if !c.get().is_none() {
            ::core::panicking::panic("assertion failed: c.get().is_none()")
        }
        s.set("hello".to_string()).unwrap();
        let c = s.clone();
        match (&c.get().map(String::as_str), &Some("hello")) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::get_or_try_init"]
    #[doc(hidden)]
    pub const get_or_try_init: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::get_or_try_init"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 91usize,
            start_col: 4usize,
            end_line: 91usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(get_or_try_init()),
        ),
    };
    fn get_or_try_init() {
        let cell: OnceCell<String> = OnceCell::new();
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
        let res = std::panic::catch_unwind(|| {
            cell
                .get_or_try_init(|| -> Result<_, ()> {
                    ::core::panicking::panic("explicit panic")
                })
        });
        if !res.is_err() {
            ::core::panicking::panic("assertion failed: res.is_err()")
        }
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
        match (&cell.get_or_try_init(|| Err(())), &Err(())) {
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
        match (
            &cell.get_or_try_init(|| Ok::<_, ()>("hello".to_string())),
            &Ok(&"hello".to_string()),
        ) {
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
        match (&cell.get(), &Some(&"hello".to_string())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::wait"]
    #[doc(hidden)]
    pub const wait: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::wait"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 107usize,
            start_col: 4usize,
            end_line: 107usize,
            end_col: 8usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(wait())),
    };
    fn wait() {
        let cell: OnceCell<String> = OnceCell::new();
        scope(|s| {
            s.spawn(|| cell.set("hello".to_string()));
            let greeting = cell.wait();
            match (&greeting, &"hello") {
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
            }
        });
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::wait_panic"]
    #[doc(hidden)]
    pub const wait_panic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::wait_panic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 118usize,
            start_col: 4usize,
            end_line: 118usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(wait_panic()),
        ),
    };
    fn wait_panic() {
        let cell: OnceCell<String> = OnceCell::new();
        scope(|s| {
            let h1 = s
                .spawn(|| {
                    cell.get_or_try_init(|| -> Result<String, ()> {
                            ::core::panicking::panic("explicit panic")
                        })
                        .unwrap();
                });
            let h2 = s
                .spawn(|| {
                    if !h1.join().is_err() {
                        ::core::panicking::panic("assertion failed: h1.join().is_err()")
                    }
                    cell.get_or_try_init(|| -> Result<String, ()> {
                            Ok("hello".to_string())
                        })
                        .unwrap();
                });
            let greeting = cell.wait();
            match (&greeting, &"hello") {
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
            if !h2.join().is_ok() {
                ::core::panicking::panic("assertion failed: h2.join().is_ok()")
            }
        });
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::get_or_init_stress"]
    #[doc(hidden)]
    pub const get_or_init_stress: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::get_or_init_stress"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 137usize,
            start_col: 4usize,
            end_line: 137usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(get_or_init_stress()),
        ),
    };
    fn get_or_init_stress() {
        let n_threads = if false { 30 } else { 1_000 };
        let n_cells = if false { 30 } else { 1_000 };
        let cells: Vec<_> = std::iter::repeat_with(|| (
                Barrier::new(n_threads),
                OnceCell::new(),
            ))
            .take(n_cells)
            .collect();
        scope(|s| {
            for t in 0..n_threads {
                let cells = &cells;
                s.spawn(move || {
                    for (i, (b, s)) in cells.iter().enumerate() {
                        b.wait();
                        let j = if t % 2 == 0 { s.wait() } else { s.get_or_init(|| i) };
                        match (&*j, &i) {
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
                    }
                });
            }
        });
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::from_impl"]
    #[doc(hidden)]
    pub const from_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::from_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 158usize,
            start_col: 4usize,
            end_line: 158usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(from_impl()),
        ),
    };
    fn from_impl() {
        match (&OnceCell::from("value").get(), &Some(&"value")) {
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
        match (&OnceCell::from("foo").get(), &Some(&"bar")) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    let kind = ::core::panicking::AssertKind::Ne;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::partialeq_impl"]
    #[doc(hidden)]
    pub const partialeq_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::partialeq_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 164usize,
            start_col: 4usize,
            end_line: 164usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(partialeq_impl()),
        ),
    };
    fn partialeq_impl() {
        if !(OnceCell::from("value") == OnceCell::from("value")) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::from(\"value\") == OnceCell::from(\"value\")",
            )
        }
        if !(OnceCell::from("foo") != OnceCell::from("bar")) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::from(\"foo\") != OnceCell::from(\"bar\")",
            )
        }
        if !(OnceCell::<String>::new() == OnceCell::new()) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::<String>::new() == OnceCell::new()",
            )
        }
        if !(OnceCell::<String>::new() != OnceCell::from("value".to_owned())) {
            ::core::panicking::panic(
                "assertion failed: OnceCell::<String>::new() != OnceCell::from(\"value\".to_owned())",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::into_inner"]
    #[doc(hidden)]
    pub const into_inner: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::into_inner"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 173usize,
            start_col: 4usize,
            end_line: 173usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(into_inner()),
        ),
    };
    fn into_inner() {
        let cell: OnceCell<String> = OnceCell::new();
        match (&cell.into_inner(), &None) {
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
        let cell = OnceCell::new();
        cell.set("hello".to_string()).unwrap();
        match (&cell.into_inner(), &Some("hello".to_string())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::debug_impl"]
    #[doc(hidden)]
    pub const debug_impl: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::debug_impl"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 182usize,
            start_col: 4usize,
            end_line: 182usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(debug_impl()),
        ),
    };
    fn debug_impl() {
        let cell = OnceCell::new();
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0:#?}", cell))
            }),
            &"OnceCell(Uninit)",
        ) {
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
        cell.set(<[_]>::into_vec(::alloc::boxed::box_new(["hello", "world"]))).unwrap();
        match (
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0:#?}", cell))
            }),
            &r#"OnceCell(
    [
        "hello",
        "world",
    ],
)"#,
        ) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::reentrant_init"]
    #[doc(hidden)]
    pub const reentrant_init: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::reentrant_init"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 200usize,
            start_col: 4usize,
            end_line: 200usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(reentrant_init()),
        ),
    };
    fn reentrant_init() {
        let examples_dir = {
            let mut exe = std::env::current_exe().unwrap();
            exe.pop();
            exe.pop();
            exe.push("examples");
            exe
        };
        let bin = examples_dir
            .join("reentrant_init_deadlocks")
            .with_extension(std::env::consts::EXE_EXTENSION);
        let mut guard = Guard {
            child: std::process::Command::new(bin).spawn().unwrap(),
        };
        std::thread::sleep(std::time::Duration::from_secs(2));
        let status = guard.child.try_wait().unwrap();
        if !status.is_none() {
            ::core::panicking::panic("assertion failed: status.is_none()")
        }
        struct Guard {
            child: std::process::Child,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = self.child.kill();
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::eval_once_macro"]
    #[doc(hidden)]
    pub const eval_once_macro: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::eval_once_macro"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 242usize,
            start_col: 4usize,
            end_line: 242usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(eval_once_macro()),
        ),
    };
    fn eval_once_macro() {
        let fib: &'static Vec<i32> = {
            static ONCE_CELL: OnceCell<Vec<i32>> = OnceCell::new();
            fn init() -> Vec<i32> {
                let mut res = <[_]>::into_vec(::alloc::boxed::box_new([1, 1]));
                for i in 0..10 {
                    let next = res[i] + res[i + 1];
                    res.push(next);
                }
                res
            }
            ONCE_CELL.get_or_init(init)
        };
        match (&fib[5], &8) {
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
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_does_not_leak_partially_constructed_boxes"]
    #[doc(hidden)]
    pub const once_cell_does_not_leak_partially_constructed_boxes: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName(
                "sync_once_cell::once_cell_does_not_leak_partially_constructed_boxes",
            ),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 269usize,
            start_col: 4usize,
            end_line: 269usize,
            end_col: 55usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(
                once_cell_does_not_leak_partially_constructed_boxes(),
            ),
        ),
    };
    fn once_cell_does_not_leak_partially_constructed_boxes() {
        let n_tries = if false { 10 } else { 100 };
        let n_readers = 10;
        let n_writers = 3;
        const MSG: &str = "Hello, World";
        for _ in 0..n_tries {
            let cell: OnceCell<String> = OnceCell::new();
            scope(|scope| {
                for _ in 0..n_readers {
                    scope
                        .spawn(|| {
                            loop {
                                if let Some(msg) = cell.get() {
                                    match (&msg, &MSG) {
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
                                    break;
                                }
                            }
                        });
                }
                for _ in 0..n_writers {
                    let _ = scope.spawn(|| cell.set(MSG.to_owned()));
                }
            });
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::get_does_not_block"]
    #[doc(hidden)]
    pub const get_does_not_block: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::get_does_not_block"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 295usize,
            start_col: 4usize,
            end_line: 295usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(get_does_not_block()),
        ),
    };
    fn get_does_not_block() {
        let cell = OnceCell::new();
        let barrier = Barrier::new(2);
        scope(|scope| {
            scope
                .spawn(|| {
                    cell.get_or_init(|| {
                        barrier.wait();
                        barrier.wait();
                        "hello".to_string()
                    });
                });
            barrier.wait();
            match (&cell.get(), &None) {
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
            barrier.wait();
        });
        match (&cell.get(), &Some(&"hello".to_string())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::arrrrrrrrrrrrrrrrrrrrrr"]
    #[doc(hidden)]
    pub const arrrrrrrrrrrrrrrrrrrrrr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::arrrrrrrrrrrrrrrrrrrrrr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 315usize,
            start_col: 4usize,
            end_line: 315usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(arrrrrrrrrrrrrrrrrrrrrr()),
        ),
    };
    fn arrrrrrrrrrrrrrrrrrrrrr() {
        let cell = OnceCell::new();
        {
            let s = String::new();
            cell.set(&s).unwrap();
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_once_cell::once_cell_is_sync_send"]
    #[doc(hidden)]
    pub const once_cell_is_sync_send: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_once_cell::once_cell_is_sync_send"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_once_cell.rs",
            start_line: 324usize,
            start_col: 4usize,
            end_line: 324usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_cell_is_sync_send()),
        ),
    };
    fn once_cell_is_sync_send() {
        fn assert_traits<T: Send + Sync>() {}
        assert_traits::<OnceCell<String>>();
        assert_traits::<Lazy<String>>();
    }
}
mod unsync_lazy {
    use core::{cell::Cell, sync::atomic::{AtomicUsize, Ordering::SeqCst}};
    use once_cell::unsync::Lazy;
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_new"]
    #[doc(hidden)]
    pub const lazy_new: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_new"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 9usize,
            start_col: 4usize,
            end_line: 9usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_new()),
        ),
    };
    fn lazy_new() {
        let called = Cell::new(0);
        let x = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        let y = *x - 30;
        match (&y, &62) {
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
        match (&called.get(), &1) {
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
        let y = *x - 30;
        match (&y, &62) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_deref_mut"]
    #[doc(hidden)]
    pub const lazy_deref_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_deref_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 28usize,
            start_col: 4usize,
            end_line: 28usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_deref_mut()),
        ),
    };
    fn lazy_deref_mut() {
        let called = Cell::new(0);
        let mut x = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        let y = *x - 30;
        match (&y, &62) {
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
        match (&called.get(), &1) {
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
        *x /= 2;
        match (&*x, &46) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_force_mut"]
    #[doc(hidden)]
    pub const lazy_force_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_force_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 47usize,
            start_col: 4usize,
            end_line: 47usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_force_mut()),
        ),
    };
    fn lazy_force_mut() {
        let called = Cell::new(0);
        let mut x = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        let v = Lazy::force_mut(&mut x);
        match (&called.get(), &1) {
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
        *v /= 2;
        match (&*x, &46) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_get_mut"]
    #[doc(hidden)]
    pub const lazy_get_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_get_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 63usize,
            start_col: 4usize,
            end_line: 63usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_get_mut()),
        ),
    };
    fn lazy_get_mut() {
        let called = Cell::new(0);
        let mut x: Lazy<u32, _> = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        match (&*x, &92) {
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
        let mut_ref: &mut u32 = Lazy::get_mut(&mut x).unwrap();
        match (&called.get(), &1) {
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
        *mut_ref /= 2;
        match (&*x, &46) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_default"]
    #[doc(hidden)]
    pub const lazy_default: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_default"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 82usize,
            start_col: 4usize,
            end_line: 82usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_default()),
        ),
    };
    fn lazy_default() {
        static CALLED: AtomicUsize = AtomicUsize::new(0);
        struct Foo(u8);
        impl Default for Foo {
            fn default() -> Self {
                CALLED.fetch_add(1, SeqCst);
                Foo(42)
            }
        }
        let lazy: Lazy<std::sync::Mutex<Foo>> = <_>::default();
        match (&CALLED.load(SeqCst), &0) {
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
        match (&lazy.lock().unwrap().0, &42) {
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
        match (&CALLED.load(SeqCst), &1) {
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
        lazy.lock().unwrap().0 = 21;
        match (&lazy.lock().unwrap().0, &21) {
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
        match (&CALLED.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_into_value"]
    #[doc(hidden)]
    pub const lazy_into_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_into_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 107usize,
            start_col: 4usize,
            end_line: 107usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_into_value()),
        ),
    };
    fn lazy_into_value() {
        let l: Lazy<i32, _> = Lazy::new(|| ::core::panicking::panic("explicit panic"));
        if !#[allow(non_exhaustive_omitted_patterns)]
        match Lazy::into_value(l) {
            Err(_) => true,
            _ => false,
        } {
            ::core::panicking::panic(
                "assertion failed: matches!(Lazy::into_value(l), Err(_))",
            )
        }
        let l = Lazy::new(|| -> i32 { 92 });
        Lazy::force(&l);
        if !#[allow(non_exhaustive_omitted_patterns)]
        match Lazy::into_value(l) {
            Ok(92) => true,
            _ => false,
        } {
            ::core::panicking::panic(
                "assertion failed: matches!(Lazy::into_value(l), Ok(92))",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::lazy_poisoning"]
    #[doc(hidden)]
    pub const lazy_poisoning: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::lazy_poisoning"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 117usize,
            start_col: 4usize,
            end_line: 117usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_poisoning()),
        ),
    };
    fn lazy_poisoning() {
        let x: Lazy<String> = Lazy::new(|| {
            ::core::panicking::panic_fmt(format_args!("kaboom"));
        });
        for _ in 0..2 {
            let res = std::panic::catch_unwind(|| x.len());
            if !res.is_err() {
                ::core::panicking::panic("assertion failed: res.is_err()")
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "unsync_lazy::arrrrrrrrrrrrrrrrrrrrrr"]
    #[doc(hidden)]
    pub const arrrrrrrrrrrrrrrrrrrrrr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("unsync_lazy::arrrrrrrrrrrrrrrrrrrrrr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/unsync_lazy.rs",
            start_line: 127usize,
            start_col: 4usize,
            end_line: 127usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(arrrrrrrrrrrrrrrrrrrrrr()),
        ),
    };
    fn arrrrrrrrrrrrrrrrrrrrrr() {
        let lazy: Lazy<&String, _>;
        {
            let s = String::new();
            lazy = Lazy::new(|| &s);
            _ = *lazy;
        }
    }
}
mod sync_lazy {
    use std::{
        cell::Cell, sync::atomic::{AtomicUsize, Ordering::SeqCst},
        thread::scope,
    };
    use once_cell::sync::{Lazy, OnceCell};
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_new"]
    #[doc(hidden)]
    pub const lazy_new: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_new"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 10usize,
            start_col: 4usize,
            end_line: 10usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_new()),
        ),
    };
    fn lazy_new() {
        let called = AtomicUsize::new(0);
        let x = Lazy::new(|| {
            called.fetch_add(1, SeqCst);
            92
        });
        match (&called.load(SeqCst), &0) {
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
        scope(|s| {
            s.spawn(|| {
                let y = *x - 30;
                match (&y, &62) {
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
                match (&called.load(SeqCst), &1) {
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
        });
        let y = *x - 30;
        match (&y, &62) {
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
        match (&called.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_deref_mut"]
    #[doc(hidden)]
    pub const lazy_deref_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_deref_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 33usize,
            start_col: 4usize,
            end_line: 33usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_deref_mut()),
        ),
    };
    fn lazy_deref_mut() {
        let called = AtomicUsize::new(0);
        let mut x = Lazy::new(|| {
            called.fetch_add(1, SeqCst);
            92
        });
        match (&called.load(SeqCst), &0) {
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
        let y = *x - 30;
        match (&y, &62) {
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
        match (&called.load(SeqCst), &1) {
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
        *x /= 2;
        match (&*x, &46) {
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
        match (&called.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_force_mut"]
    #[doc(hidden)]
    pub const lazy_force_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_force_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 52usize,
            start_col: 4usize,
            end_line: 52usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_force_mut()),
        ),
    };
    fn lazy_force_mut() {
        let called = Cell::new(0);
        let mut x = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        let v = Lazy::force_mut(&mut x);
        match (&called.get(), &1) {
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
        *v /= 2;
        match (&*x, &46) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_get_mut"]
    #[doc(hidden)]
    pub const lazy_get_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_get_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 68usize,
            start_col: 4usize,
            end_line: 68usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_get_mut()),
        ),
    };
    fn lazy_get_mut() {
        let called = Cell::new(0);
        let mut x: Lazy<u32, _> = Lazy::new(|| {
            called.set(called.get() + 1);
            92
        });
        match (&called.get(), &0) {
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
        match (&*x, &92) {
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
        let mut_ref: &mut u32 = Lazy::get_mut(&mut x).unwrap();
        match (&called.get(), &1) {
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
        *mut_ref /= 2;
        match (&*x, &46) {
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
        match (&called.get(), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_default"]
    #[doc(hidden)]
    pub const lazy_default: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_default"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 87usize,
            start_col: 4usize,
            end_line: 87usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_default()),
        ),
    };
    fn lazy_default() {
        static CALLED: AtomicUsize = AtomicUsize::new(0);
        struct Foo(u8);
        impl Default for Foo {
            fn default() -> Self {
                CALLED.fetch_add(1, SeqCst);
                Foo(42)
            }
        }
        let lazy: Lazy<std::sync::Mutex<Foo>> = <_>::default();
        match (&CALLED.load(SeqCst), &0) {
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
        match (&lazy.lock().unwrap().0, &42) {
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
        match (&CALLED.load(SeqCst), &1) {
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
        lazy.lock().unwrap().0 = 21;
        match (&lazy.lock().unwrap().0, &21) {
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
        match (&CALLED.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::static_lazy"]
    #[doc(hidden)]
    pub const static_lazy: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::static_lazy"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 112usize,
            start_col: 4usize,
            end_line: 112usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(static_lazy()),
        ),
    };
    fn static_lazy() {
        static XS: Lazy<Vec<i32>> = Lazy::new(|| {
            let mut xs = Vec::new();
            xs.push(1);
            xs.push(2);
            xs.push(3);
            xs
        });
        scope(|s| {
            s.spawn(|| {
                match (&&*XS, &&<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3]))) {
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
        });
        match (&&*XS, &&<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3]))) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::static_lazy_via_fn"]
    #[doc(hidden)]
    pub const static_lazy_via_fn: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::static_lazy_via_fn"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 129usize,
            start_col: 4usize,
            end_line: 129usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(static_lazy_via_fn()),
        ),
    };
    fn static_lazy_via_fn() {
        fn xs() -> &'static Vec<i32> {
            static XS: OnceCell<Vec<i32>> = OnceCell::new();
            XS.get_or_init(|| {
                let mut xs = Vec::new();
                xs.push(1);
                xs.push(2);
                xs.push(3);
                xs
            })
        }
        match (&xs(), &&<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3]))) {
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
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_into_value"]
    #[doc(hidden)]
    pub const lazy_into_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_into_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 144usize,
            start_col: 4usize,
            end_line: 144usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_into_value()),
        ),
    };
    fn lazy_into_value() {
        let l: Lazy<i32, _> = Lazy::new(|| ::core::panicking::panic("explicit panic"));
        if !#[allow(non_exhaustive_omitted_patterns)]
        match Lazy::into_value(l) {
            Err(_) => true,
            _ => false,
        } {
            ::core::panicking::panic(
                "assertion failed: matches!(Lazy::into_value(l), Err(_))",
            )
        }
        let l = Lazy::new(|| -> i32 { 92 });
        Lazy::force(&l);
        if !#[allow(non_exhaustive_omitted_patterns)]
        match Lazy::into_value(l) {
            Ok(92) => true,
            _ => false,
        } {
            ::core::panicking::panic(
                "assertion failed: matches!(Lazy::into_value(l), Ok(92))",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_poisoning"]
    #[doc(hidden)]
    pub const lazy_poisoning: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_poisoning"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 153usize,
            start_col: 4usize,
            end_line: 153usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_poisoning()),
        ),
    };
    fn lazy_poisoning() {
        let x: Lazy<String> = Lazy::new(|| {
            ::core::panicking::panic_fmt(format_args!("kaboom"));
        });
        for _ in 0..2 {
            let res = std::panic::catch_unwind(|| x.len());
            if !res.is_err() {
                ::core::panicking::panic("assertion failed: res.is_err()")
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::arrrrrrrrrrrrrrrrrrrrrr"]
    #[doc(hidden)]
    pub const arrrrrrrrrrrrrrrrrrrrrr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::arrrrrrrrrrrrrrrrrrrrrr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 163usize,
            start_col: 4usize,
            end_line: 163usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(arrrrrrrrrrrrrrrrrrrrrr()),
        ),
    };
    fn arrrrrrrrrrrrrrrrrrrrrr() {
        let lazy: Lazy<&String, _>;
        {
            let s = String::new();
            lazy = Lazy::new(|| &s);
            _ = *lazy;
        }
    }
    extern crate test;
    #[rustc_test_marker = "sync_lazy::lazy_is_sync_send"]
    #[doc(hidden)]
    pub const lazy_is_sync_send: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("sync_lazy::lazy_is_sync_send"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/sync_lazy.rs",
            start_line: 173usize,
            start_col: 4usize,
            end_line: 173usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(lazy_is_sync_send()),
        ),
    };
    fn lazy_is_sync_send() {
        fn assert_traits<T: Send + Sync>() {}
        assert_traits::<Lazy<String>>();
    }
}
mod race {
    use std::sync::Barrier;
    use std::{
        num::NonZeroUsize, sync::atomic::{AtomicUsize, Ordering::SeqCst},
        thread::scope,
    };
    use once_cell::race::{OnceBool, OnceNonZeroUsize, OnceRef};
    extern crate test;
    #[rustc_test_marker = "race::once_non_zero_usize_smoke_test"]
    #[doc(hidden)]
    pub const once_non_zero_usize_smoke_test: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_non_zero_usize_smoke_test"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 12usize,
            start_col: 4usize,
            end_line: 12usize,
            end_col: 34usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_non_zero_usize_smoke_test()),
        ),
    };
    fn once_non_zero_usize_smoke_test() {
        let cnt = AtomicUsize::new(0);
        let cell = OnceNonZeroUsize::new();
        let val = NonZeroUsize::new(92).unwrap();
        scope(|s| {
            s.spawn(|| {
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            val
                        }),
                    &val,
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            val
                        }),
                    &val,
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
        });
        match (&cell.get(), &Some(val)) {
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
        match (&cnt.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_non_zero_usize_set"]
    #[doc(hidden)]
    pub const once_non_zero_usize_set: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_non_zero_usize_set"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 42usize,
            start_col: 4usize,
            end_line: 42usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_non_zero_usize_set()),
        ),
    };
    fn once_non_zero_usize_set() {
        let val1 = NonZeroUsize::new(92).unwrap();
        let val2 = NonZeroUsize::new(62).unwrap();
        let cell = OnceNonZeroUsize::new();
        if !cell.set(val1).is_ok() {
            ::core::panicking::panic("assertion failed: cell.set(val1).is_ok()")
        }
        match (&cell.get(), &Some(val1)) {
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
        if !cell.set(val2).is_err() {
            ::core::panicking::panic("assertion failed: cell.set(val2).is_err()")
        }
        match (&cell.get(), &Some(val1)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_non_zero_usize_first_wins"]
    #[doc(hidden)]
    pub const once_non_zero_usize_first_wins: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_non_zero_usize_first_wins"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 57usize,
            start_col: 4usize,
            end_line: 57usize,
            end_col: 34usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_non_zero_usize_first_wins()),
        ),
    };
    fn once_non_zero_usize_first_wins() {
        let val1 = NonZeroUsize::new(92).unwrap();
        let val2 = NonZeroUsize::new(62).unwrap();
        let cell = OnceNonZeroUsize::new();
        let b1 = Barrier::new(2);
        let b2 = Barrier::new(2);
        let b3 = Barrier::new(2);
        scope(|s| {
            s.spawn(|| {
                let r1 = cell
                    .get_or_init(|| {
                        b1.wait();
                        b2.wait();
                        val1
                    });
                match (&r1, &val1) {
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
                b3.wait();
            });
            b1.wait();
            s.spawn(|| {
                let r2 = cell
                    .get_or_init(|| {
                        b2.wait();
                        b3.wait();
                        val2
                    });
                match (&r2, &val1) {
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
        });
        match (&cell.get(), &Some(val1)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_bool_smoke_test"]
    #[doc(hidden)]
    pub const once_bool_smoke_test: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_bool_smoke_test"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 91usize,
            start_col: 4usize,
            end_line: 91usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_bool_smoke_test()),
        ),
    };
    fn once_bool_smoke_test() {
        let cnt = AtomicUsize::new(0);
        let cell = OnceBool::new();
        scope(|s| {
            s.spawn(|| {
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            false
                        }),
                    &false,
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            false
                        }),
                    &false,
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
        });
        match (&cell.get(), &Some(false)) {
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
        match (&cnt.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_bool_set"]
    #[doc(hidden)]
    pub const once_bool_set: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_bool_set"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 120usize,
            start_col: 4usize,
            end_line: 120usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_bool_set()),
        ),
    };
    fn once_bool_set() {
        let cell = OnceBool::new();
        if !cell.set(false).is_ok() {
            ::core::panicking::panic("assertion failed: cell.set(false).is_ok()")
        }
        match (&cell.get(), &Some(false)) {
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
        if !cell.set(true).is_err() {
            ::core::panicking::panic("assertion failed: cell.set(true).is_err()")
        }
        match (&cell.get(), &Some(false)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_bool_get_or_try_init"]
    #[doc(hidden)]
    pub const once_bool_get_or_try_init: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_bool_get_or_try_init"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 131usize,
            start_col: 4usize,
            end_line: 131usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_bool_get_or_try_init()),
        ),
    };
    fn once_bool_get_or_try_init() {
        let cell = OnceBool::new();
        let result1: Result<bool, ()> = cell.get_or_try_init(|| Ok(true));
        let result2: Result<bool, ()> = cell.get_or_try_init(|| Ok(false));
        match (&result1, &Ok(true)) {
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
        match (&result2, &Ok(true)) {
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
        let cell = OnceBool::new();
        let result3: Result<bool, ()> = cell.get_or_try_init(|| Err(()));
        match (&result3, &Err(())) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_ref_smoke_test"]
    #[doc(hidden)]
    pub const once_ref_smoke_test: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_ref_smoke_test"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 146usize,
            start_col: 4usize,
            end_line: 146usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_ref_smoke_test()),
        ),
    };
    fn once_ref_smoke_test() {
        let cnt: AtomicUsize = AtomicUsize::new(0);
        let cell: OnceRef<'_, &str> = OnceRef::new();
        scope(|s| {
            s.spawn(|| {
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            &"false"
                        }),
                    &&"false",
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
                match (
                    &cell
                        .get_or_init(|| {
                            cnt.fetch_add(1, SeqCst);
                            &"false"
                        }),
                    &&"false",
                ) {
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
                match (&cnt.load(SeqCst), &1) {
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
        });
        match (&cell.get(), &Some(&"false")) {
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
        match (&cnt.load(SeqCst), &1) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::once_ref_set"]
    #[doc(hidden)]
    pub const once_ref_set: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::once_ref_set"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 175usize,
            start_col: 4usize,
            end_line: 175usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_ref_set()),
        ),
    };
    fn once_ref_set() {
        let cell: OnceRef<'_, &str> = OnceRef::new();
        if !cell.set(&"false").is_ok() {
            ::core::panicking::panic("assertion failed: cell.set(&\"false\").is_ok()")
        }
        match (&cell.get(), &Some(&"false")) {
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
        if !cell.set(&"true").is_err() {
            ::core::panicking::panic("assertion failed: cell.set(&\"true\").is_err()")
        }
        match (&cell.get(), &Some(&"false")) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race::get_unchecked"]
    #[doc(hidden)]
    pub const get_unchecked: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race::get_unchecked"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race.rs",
            start_line: 186usize,
            start_col: 4usize,
            end_line: 186usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(get_unchecked()),
        ),
    };
    fn get_unchecked() {
        let cell = OnceNonZeroUsize::new();
        cell.set(NonZeroUsize::new(92).unwrap()).unwrap();
        let value = unsafe { cell.get_unchecked() };
        match (&value, &NonZeroUsize::new(92).unwrap()) {
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
    }
}
mod race_once_box {
    use std::sync::Barrier;
    use std::sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    };
    use once_cell::race::OnceBox;
    struct Heap {
        total: Arc<AtomicUsize>,
    }
    #[automatically_derived]
    impl ::core::default::Default for Heap {
        #[inline]
        fn default() -> Heap {
            Heap {
                total: ::core::default::Default::default(),
            }
        }
    }
    struct Pebble<T> {
        val: T,
        total: Arc<AtomicUsize>,
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Pebble<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Pebble",
                "val",
                &self.val,
                "total",
                &&self.total,
            )
        }
    }
    impl<T> Drop for Pebble<T> {
        fn drop(&mut self) {
            self.total.fetch_sub(1, SeqCst);
        }
    }
    impl Heap {
        fn total(&self) -> usize {
            self.total.load(SeqCst)
        }
        fn new_pebble<T>(&self, val: T) -> Pebble<T> {
            self.total.fetch_add(1, SeqCst);
            Pebble {
                val,
                total: Arc::clone(&self.total),
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::once_box_smoke_test"]
    #[doc(hidden)]
    pub const once_box_smoke_test: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::once_box_smoke_test"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 39usize,
            start_col: 4usize,
            end_line: 39usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_box_smoke_test()),
        ),
    };
    fn once_box_smoke_test() {
        use std::thread::scope;
        let heap = Heap::default();
        let global_cnt = AtomicUsize::new(0);
        let cell = OnceBox::new();
        let b = Barrier::new(128);
        scope(|s| {
            for _ in 0..128 {
                s.spawn(|| {
                    let local_cnt = AtomicUsize::new(0);
                    cell.get_or_init(|| {
                        global_cnt.fetch_add(1, SeqCst);
                        local_cnt.fetch_add(1, SeqCst);
                        b.wait();
                        Box::new(heap.new_pebble(()))
                    });
                    match (&local_cnt.load(SeqCst), &1) {
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
                    cell.get_or_init(|| {
                        global_cnt.fetch_add(1, SeqCst);
                        local_cnt.fetch_add(1, SeqCst);
                        Box::new(heap.new_pebble(()))
                    });
                    match (&local_cnt.load(SeqCst), &1) {
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
            }
        });
        if !cell.get().is_some() {
            ::core::panicking::panic("assertion failed: cell.get().is_some()")
        }
        if !(global_cnt.load(SeqCst) > 10) {
            ::core::panicking::panic("assertion failed: global_cnt.load(SeqCst) > 10")
        }
        match (&heap.total(), &1) {
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
        drop(cell);
        match (&heap.total(), &0) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::once_box_set"]
    #[doc(hidden)]
    pub const once_box_set: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::once_box_set"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 76usize,
            start_col: 4usize,
            end_line: 76usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_box_set()),
        ),
    };
    fn once_box_set() {
        let heap = Heap::default();
        let cell = OnceBox::new();
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
        if !cell.set(Box::new(heap.new_pebble("hello"))).is_ok() {
            ::core::panicking::panic(
                "assertion failed: cell.set(Box::new(heap.new_pebble(\"hello\"))).is_ok()",
            )
        }
        match (&cell.get().unwrap().val, &"hello") {
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
        match (&heap.total(), &1) {
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
        if !cell.set(Box::new(heap.new_pebble("world"))).is_err() {
            ::core::panicking::panic(
                "assertion failed: cell.set(Box::new(heap.new_pebble(\"world\"))).is_err()",
            )
        }
        match (&cell.get().unwrap().val, &"hello") {
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
        match (&heap.total(), &1) {
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
        drop(cell);
        match (&heap.total(), &0) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::once_box_first_wins"]
    #[doc(hidden)]
    pub const once_box_first_wins: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::once_box_first_wins"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 95usize,
            start_col: 4usize,
            end_line: 95usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_box_first_wins()),
        ),
    };
    fn once_box_first_wins() {
        use std::thread::scope;
        let cell = OnceBox::new();
        let val1 = 92;
        let val2 = 62;
        let b1 = Barrier::new(2);
        let b2 = Barrier::new(2);
        let b3 = Barrier::new(2);
        scope(|s| {
            s.spawn(|| {
                let r1 = cell
                    .get_or_init(|| {
                        b1.wait();
                        b2.wait();
                        Box::new(val1)
                    });
                match (&*r1, &val1) {
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
                b3.wait();
            });
            b1.wait();
            s.spawn(|| {
                let r2 = cell
                    .get_or_init(|| {
                        b2.wait();
                        b3.wait();
                        Box::new(val2)
                    });
                match (&*r2, &val1) {
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
        });
        match (&cell.get(), &Some(&val1)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::once_box_reentrant"]
    #[doc(hidden)]
    pub const once_box_reentrant: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::once_box_reentrant"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 130usize,
            start_col: 4usize,
            end_line: 130usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_box_reentrant()),
        ),
    };
    fn once_box_reentrant() {
        let cell = OnceBox::new();
        let res = cell
            .get_or_init(|| {
                cell.get_or_init(|| Box::new("hello".to_string()));
                Box::new("world".to_string())
            });
        match (&res, &"hello") {
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
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::once_box_default"]
    #[doc(hidden)]
    pub const once_box_default: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::once_box_default"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 140usize,
            start_col: 4usize,
            end_line: 140usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(once_box_default()),
        ),
    };
    fn once_box_default() {
        struct Foo;
        let cell: OnceBox<Foo> = Default::default();
        if !cell.get().is_none() {
            ::core::panicking::panic("assertion failed: cell.get().is_none()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::onece_box_with_value"]
    #[doc(hidden)]
    pub const onece_box_with_value: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::onece_box_with_value"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 148usize,
            start_col: 4usize,
            end_line: 148usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(onece_box_with_value()),
        ),
    };
    fn onece_box_with_value() {
        let cell = OnceBox::with_value(Box::new(92));
        match (&cell.get(), &Some(&92)) {
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
    }
    extern crate test;
    #[rustc_test_marker = "race_once_box::onece_box_clone"]
    #[doc(hidden)]
    pub const onece_box_clone: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("race_once_box::onece_box_clone"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "tests/it/race_once_box.rs",
            start_line: 154usize,
            start_col: 4usize,
            end_line: 154usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(onece_box_clone()),
        ),
    };
    fn onece_box_clone() {
        let cell1 = OnceBox::new();
        let cell2 = cell1.clone();
        cell1.set(Box::new(92)).unwrap();
        let cell3 = cell1.clone();
        match (&cell1.get(), &Some(&92)) {
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
        match (&cell2.get(), &None) {
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
        match (&cell3.get(), &Some(&92)) {
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
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &get_unchecked,
            &once_bool_get_or_try_init,
            &once_bool_set,
            &once_bool_smoke_test,
            &once_non_zero_usize_first_wins,
            &once_non_zero_usize_set,
            &once_non_zero_usize_smoke_test,
            &once_ref_set,
            &once_ref_smoke_test,
            &once_box_default,
            &once_box_first_wins,
            &once_box_reentrant,
            &once_box_set,
            &once_box_smoke_test,
            &onece_box_clone,
            &onece_box_with_value,
            &arrrrrrrrrrrrrrrrrrrrrr,
            &lazy_default,
            &lazy_deref_mut,
            &lazy_force_mut,
            &lazy_get_mut,
            &lazy_into_value,
            &lazy_is_sync_send,
            &lazy_new,
            &lazy_poisoning,
            &static_lazy,
            &static_lazy_via_fn,
            &arrrrrrrrrrrrrrrrrrrrrr,
            &clone,
            &debug_impl,
            &eval_once_macro,
            &from_impl,
            &get_does_not_block,
            &get_or_init_stress,
            &get_or_try_init,
            &into_inner,
            &once_cell,
            &once_cell_does_not_leak_partially_constructed_boxes,
            &once_cell_drop,
            &once_cell_drop_empty,
            &once_cell_get_mut,
            &once_cell_get_unchecked,
            &once_cell_is_sync_send,
            &once_cell_with_value,
            &partialeq_impl,
            &reentrant_init,
            &wait,
            &wait_panic,
            &arrrrrrrrrrrrrrrrrrrrrr,
            &lazy_default,
            &lazy_deref_mut,
            &lazy_force_mut,
            &lazy_get_mut,
            &lazy_into_value,
            &lazy_new,
            &lazy_poisoning,
            &aliasing_in_get,
            &arrrrrrrrrrrrrrrrrrrrrr,
            &clone,
            &debug_impl,
            &from_impl,
            &get_or_try_init,
            &into_inner,
            &once_cell,
            &once_cell_drop,
            &once_cell_drop_empty,
            &once_cell_get_mut,
            &once_cell_with_value,
            &partialeq_impl,
            &reentrant_init,
        ],
    )
}
