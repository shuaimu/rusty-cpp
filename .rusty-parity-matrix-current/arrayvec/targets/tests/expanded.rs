#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
extern crate arrayvec;
#[macro_use]
extern crate matches;
use arrayvec::ArrayVec;
use arrayvec::ArrayString;
use std::mem;
use arrayvec::CapacityError;
use std::collections::HashMap;
extern crate test;
#[rustc_test_marker = "test_simple"]
#[doc(hidden)]
pub const test_simple: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_simple"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 13usize,
        start_col: 4usize,
        end_line: 13usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_simple()),
    ),
};
fn test_simple() {
    use std::ops::Add;
    let mut vec: ArrayVec<Vec<i32>, 3> = ArrayVec::new();
    vec.push(<[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3, 4])));
    vec.push(<[_]>::into_vec(::alloc::boxed::box_new([10])));
    vec.push(<[_]>::into_vec(::alloc::boxed::box_new([-1, 13, -2])));
    for elt in &vec {
        match (&elt.iter().fold(0, Add::add), &10) {
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
    let sum_len = vec.into_iter().map(|x| x.len()).fold(0, Add::add);
    match (&sum_len, &8) {
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
#[rustc_test_marker = "test_capacity_left"]
#[doc(hidden)]
pub const test_capacity_left: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_capacity_left"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 31usize,
        start_col: 4usize,
        end_line: 31usize,
        end_col: 22usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_capacity_left()),
    ),
};
fn test_capacity_left() {
    let mut vec: ArrayVec<usize, 4> = ArrayVec::new();
    match (&vec.remaining_capacity(), &4) {
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
    vec.push(1);
    match (&vec.remaining_capacity(), &3) {
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
    vec.push(2);
    match (&vec.remaining_capacity(), &2) {
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
    vec.push(3);
    match (&vec.remaining_capacity(), &1) {
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
    vec.push(4);
    match (&vec.remaining_capacity(), &0) {
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
#[rustc_test_marker = "test_extend_from_slice"]
#[doc(hidden)]
pub const test_extend_from_slice: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend_from_slice"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 45usize,
        start_col: 4usize,
        end_line: 45usize,
        end_col: 26usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend_from_slice()),
    ),
};
fn test_extend_from_slice() {
    let mut vec: ArrayVec<usize, 10> = ArrayVec::new();
    vec.try_extend_from_slice(&[1, 2, 3]).unwrap();
    match (&vec.len(), &3) {
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
    match (&&vec[..], &&[1, 2, 3]) {
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
    match (&vec.pop(), &Some(3)) {
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
    match (&&vec[..], &&[1, 2]) {
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
#[rustc_test_marker = "test_extend_from_slice_error"]
#[doc(hidden)]
pub const test_extend_from_slice_error: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend_from_slice_error"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 56usize,
        start_col: 4usize,
        end_line: 56usize,
        end_col: 32usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend_from_slice_error()),
    ),
};
fn test_extend_from_slice_error() {
    let mut vec: ArrayVec<usize, 10> = ArrayVec::new();
    vec.try_extend_from_slice(&[1, 2, 3]).unwrap();
    let res = vec.try_extend_from_slice(&[0; 8]);
    match res {
        Err(_) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(_)",
                ),
            );
        }
    };
    let mut vec: ArrayVec<usize, 0> = ArrayVec::new();
    let res = vec.try_extend_from_slice(&[0; 1]);
    match res {
        Err(_) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(_)",
                ),
            );
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_try_from_slice_error"]
#[doc(hidden)]
pub const test_try_from_slice_error: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_try_from_slice_error"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 69usize,
        start_col: 4usize,
        end_line: 69usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_try_from_slice_error()),
    ),
};
fn test_try_from_slice_error() {
    use arrayvec::ArrayVec;
    use std::convert::TryInto as _;
    let res: Result<ArrayVec<_, 2>, _> = (&[1, 2, 3] as &[_]).try_into();
    match res {
        Err(_) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(_)",
                ),
            );
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_u16_index"]
#[doc(hidden)]
pub const test_u16_index: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_u16_index"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 78usize,
        start_col: 4usize,
        end_line: 78usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_u16_index()),
    ),
};
fn test_u16_index() {
    const N: usize = 4096;
    let mut vec: ArrayVec<_, N> = ArrayVec::new();
    for _ in 0..N {
        if !vec.try_push(1u8).is_ok() {
            ::core::panicking::panic("assertion failed: vec.try_push(1u8).is_ok()")
        }
    }
    if !vec.try_push(0).is_err() {
        ::core::panicking::panic("assertion failed: vec.try_push(0).is_err()")
    }
    match (&vec.len(), &N) {
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
#[rustc_test_marker = "test_iter"]
#[doc(hidden)]
pub const test_iter: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_iter"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 89usize,
        start_col: 4usize,
        end_line: 89usize,
        end_col: 13usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_iter())),
};
fn test_iter() {
    let mut iter = ArrayVec::from([1, 2, 3]).into_iter();
    match (&iter.size_hint(), &(3, Some(3))) {
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
    match (&iter.next_back(), &Some(3)) {
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
    match (&iter.next(), &Some(1)) {
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
    match (&iter.next_back(), &Some(2)) {
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
    match (&iter.size_hint(), &(0, Some(0))) {
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
    match (&iter.next_back(), &None) {
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
#[rustc_test_marker = "test_drop"]
#[doc(hidden)]
pub const test_drop: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drop"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 100usize,
        start_col: 4usize,
        end_line: 100usize,
        end_col: 13usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_drop())),
};
fn test_drop() {
    use std::cell::Cell;
    let flag = &Cell::new(0);
    struct Bump<'a>(&'a Cell<i32>);
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for Bump<'a> {
        #[inline]
        fn clone(&self) -> Bump<'a> {
            Bump(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<'a> Drop for Bump<'a> {
        fn drop(&mut self) {
            let n = self.0.get();
            self.0.set(n + 1);
        }
    }
    {
        let mut array = ArrayVec::<Bump, 128>::new();
        array.push(Bump(flag));
        array.push(Bump(flag));
    }
    match (&flag.get(), &2) {
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
    flag.set(0);
    {
        let mut array = ArrayVec::<_, 3>::new();
        array.push(<[_]>::into_vec(::alloc::boxed::box_new([Bump(flag)])));
        array.push(<[_]>::into_vec(::alloc::boxed::box_new([Bump(flag), Bump(flag)])));
        array.push(::alloc::vec::Vec::new());
        let push4 = array
            .try_push(<[_]>::into_vec(::alloc::boxed::box_new([Bump(flag)])));
        match (&flag.get(), &0) {
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
        drop(push4);
        match (&flag.get(), &1) {
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
        drop(array.pop());
        match (&flag.get(), &1) {
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
        drop(array.pop());
        match (&flag.get(), &3) {
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
    match (&flag.get(), &4) {
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
    flag.set(0);
    {
        let mut array = ArrayVec::<_, 3>::new();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        let inner = array.into_inner();
        if !inner.is_ok() {
            ::core::panicking::panic("assertion failed: inner.is_ok()")
        }
        match (&flag.get(), &0) {
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
        drop(inner);
        match (&flag.get(), &3) {
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
    flag.set(0);
    {
        let mut array1 = ArrayVec::<_, 3>::new();
        array1.push(Bump(flag));
        array1.push(Bump(flag));
        array1.push(Bump(flag));
        let array2 = array1.take();
        match (&flag.get(), &0) {
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
        drop(array1);
        match (&flag.get(), &0) {
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
        drop(array2);
        match (&flag.get(), &3) {
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
    flag.set(0);
    {
        let mut array = ArrayVec::<_, 3>::new();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        let mut iter = array.into_iter();
        match (&flag.get(), &0) {
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
        iter.next();
        match (&flag.get(), &1) {
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
        let clone = iter.clone();
        match (&flag.get(), &1) {
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
        drop(clone);
        match (&flag.get(), &3) {
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
        drop(iter);
        match (&flag.get(), &5) {
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
#[rustc_test_marker = "test_drop_panics"]
#[doc(hidden)]
pub const test_drop_panics: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drop_panics"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 192usize,
        start_col: 4usize,
        end_line: 192usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drop_panics()),
    ),
};
fn test_drop_panics() {
    use std::cell::Cell;
    use std::panic::catch_unwind;
    use std::panic::AssertUnwindSafe;
    let flag = &Cell::new(0);
    struct Bump<'a>(&'a Cell<i32>);
    impl<'a> Drop for Bump<'a> {
        fn drop(&mut self) {
            let n = self.0.get();
            self.0.set(n + 1);
            if n == 0 {
                {
                    ::std::rt::begin_panic("Panic in Bump's drop");
                };
            }
        }
    }
    flag.set(0);
    {
        let array = <[_]>::into_vec(::alloc::boxed::box_new([Bump(flag), Bump(flag)]));
        let res = catch_unwind(
            AssertUnwindSafe(|| {
                drop(array);
            }),
        );
        if !res.is_err() {
            ::core::panicking::panic("assertion failed: res.is_err()")
        }
    }
    if flag.get() != 2 {
        {
            ::std::io::_print(
                format_args!(
                    "test_drop_panics: skip, this version of Rust doesn\'t continue in drop_in_place\n",
                ),
            );
        };
        return;
    }
    flag.set(0);
    {
        let mut array = ArrayVec::<Bump, 128>::new();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        let res = catch_unwind(
            AssertUnwindSafe(|| {
                drop(array);
            }),
        );
        if !res.is_err() {
            ::core::panicking::panic("assertion failed: res.is_err()")
        }
    }
    match (&flag.get(), &3) {
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
    flag.set(0);
    {
        let mut array = ArrayVec::<Bump, 16>::new();
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        array.push(Bump(flag));
        let i = 2;
        let tail_len = array.len() - i;
        let res = catch_unwind(
            AssertUnwindSafe(|| {
                array.truncate(i);
            }),
        );
        if !res.is_err() {
            ::core::panicking::panic("assertion failed: res.is_err()")
        }
        match (&flag.get(), &(tail_len as i32)) {
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
#[rustc_test_marker = "test_extend"]
#[doc(hidden)]
pub const test_extend: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 266usize,
        start_col: 4usize,
        end_line: 266usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend()),
    ),
};
fn test_extend() {
    let mut range = 0..10;
    let mut array: ArrayVec<_, 5> = range.by_ref().take(5).collect();
    match (&&array[..], &&[0, 1, 2, 3, 4]) {
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
    match (&range.next(), &Some(5)) {
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
    array.extend(range.by_ref().take(0));
    match (&range.next(), &Some(6)) {
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
    let mut array: ArrayVec<_, 10> = (0..3).collect();
    match (&&array[..], &&[0, 1, 2]) {
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
    array.extend(3..5);
    match (&&array[..], &&[0, 1, 2, 3, 4]) {
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
#[rustc_test_marker = "test_extend_capacity_panic_1"]
#[doc(hidden)]
pub const test_extend_capacity_panic_1: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend_capacity_panic_1"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 284usize,
        start_col: 4usize,
        end_line: 284usize,
        end_col: 32usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend_capacity_panic_1()),
    ),
};
#[should_panic]
fn test_extend_capacity_panic_1() {
    let mut range = 0..10;
    let _: ArrayVec<_, 5> = range.by_ref().collect();
}
extern crate test;
#[rustc_test_marker = "test_extend_capacity_panic_2"]
#[doc(hidden)]
pub const test_extend_capacity_panic_2: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend_capacity_panic_2"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 292usize,
        start_col: 4usize,
        end_line: 292usize,
        end_col: 32usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend_capacity_panic_2()),
    ),
};
#[should_panic]
fn test_extend_capacity_panic_2() {
    let mut range = 0..10;
    let mut array: ArrayVec<_, 5> = range.by_ref().take(5).collect();
    match (&&array[..], &&[0, 1, 2, 3, 4]) {
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
    match (&range.next(), &Some(5)) {
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
    array.extend(range.by_ref().take(1));
}
extern crate test;
#[rustc_test_marker = "test_is_send_sync"]
#[doc(hidden)]
pub const test_is_send_sync: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_is_send_sync"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 302usize,
        start_col: 4usize,
        end_line: 302usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_is_send_sync()),
    ),
};
fn test_is_send_sync() {
    let data = ArrayVec::<Vec<i32>, 5>::new();
    &data as &dyn Send;
    &data as &dyn Sync;
}
extern crate test;
#[rustc_test_marker = "test_compact_size"]
#[doc(hidden)]
pub const test_compact_size: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_compact_size"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 309usize,
        start_col: 4usize,
        end_line: 309usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_compact_size()),
    ),
};
fn test_compact_size() {
    type ByteArray = ArrayVec<u8, 4>;
    {
        ::std::io::_print(format_args!("{0}\n", mem::size_of::<ByteArray>()));
    };
    if !(mem::size_of::<ByteArray>() <= 2 * mem::size_of::<u32>()) {
        ::core::panicking::panic(
            "assertion failed: mem::size_of::<ByteArray>() <= 2 * mem::size_of::<u32>()",
        )
    }
    type EmptyArray = ArrayVec<u8, 0>;
    {
        ::std::io::_print(format_args!("{0}\n", mem::size_of::<EmptyArray>()));
    };
    if !(mem::size_of::<EmptyArray>() <= mem::size_of::<u32>()) {
        ::core::panicking::panic(
            "assertion failed: mem::size_of::<EmptyArray>() <= mem::size_of::<u32>()",
        )
    }
    type QuadArray = ArrayVec<u32, 3>;
    {
        ::std::io::_print(format_args!("{0}\n", mem::size_of::<QuadArray>()));
    };
    if !(mem::size_of::<QuadArray>() <= 4 * 4 + mem::size_of::<u32>()) {
        ::core::panicking::panic(
            "assertion failed: mem::size_of::<QuadArray>() <= 4 * 4 + mem::size_of::<u32>()",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_still_works_with_option_arrayvec"]
#[doc(hidden)]
pub const test_still_works_with_option_arrayvec: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_still_works_with_option_arrayvec"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 327usize,
        start_col: 4usize,
        end_line: 327usize,
        end_col: 41usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_still_works_with_option_arrayvec()),
    ),
};
fn test_still_works_with_option_arrayvec() {
    type RefArray = ArrayVec<&'static i32, 2>;
    let array = Some(RefArray::new());
    if !array.is_some() {
        ::core::panicking::panic("assertion failed: array.is_some()")
    }
    {
        ::std::io::_print(format_args!("{0:?}\n", array));
    };
}
extern crate test;
#[rustc_test_marker = "test_drain"]
#[doc(hidden)]
pub const test_drain: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drain"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 335usize,
        start_col: 4usize,
        end_line: 335usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drain()),
    ),
};
fn test_drain() {
    let mut v = ArrayVec::from([0; 8]);
    v.pop();
    v.drain(0..7);
    match (&&v[..], &&[]) {
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
    v.extend(0..8);
    v.drain(1..4);
    match (&&v[..], &&[0, 4, 5, 6, 7]) {
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
    let u: ArrayVec<_, 3> = v.drain(1..4).rev().collect();
    match (&&u[..], &&[6, 5, 4]) {
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
    match (&&v[..], &&[0, 7]) {
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
    v.drain(..);
    match (&&v[..], &&[]) {
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
#[rustc_test_marker = "test_drain_range_inclusive"]
#[doc(hidden)]
pub const test_drain_range_inclusive: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drain_range_inclusive"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 352usize,
        start_col: 4usize,
        end_line: 352usize,
        end_col: 30usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drain_range_inclusive()),
    ),
};
fn test_drain_range_inclusive() {
    let mut v = ArrayVec::from([0; 8]);
    v.drain(0..=7);
    match (&&v[..], &&[]) {
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
    v.extend(0..8);
    v.drain(1..=4);
    match (&&v[..], &&[0, 5, 6, 7]) {
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
    let u: ArrayVec<_, 3> = v.drain(1..=2).rev().collect();
    match (&&u[..], &&[6, 5]) {
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
    match (&&v[..], &&[0, 7]) {
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
    v.drain(..);
    match (&&v[..], &&[]) {
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
#[rustc_test_marker = "test_drain_range_inclusive_oob"]
#[doc(hidden)]
pub const test_drain_range_inclusive_oob: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drain_range_inclusive_oob"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 369usize,
        start_col: 4usize,
        end_line: 369usize,
        end_col: 34usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drain_range_inclusive_oob()),
    ),
};
#[should_panic]
fn test_drain_range_inclusive_oob() {
    let mut v = ArrayVec::from([0; 0]);
    v.drain(0..=0);
}
extern crate test;
#[rustc_test_marker = "test_retain"]
#[doc(hidden)]
pub const test_retain: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_retain"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 375usize,
        start_col: 4usize,
        end_line: 375usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_retain()),
    ),
};
fn test_retain() {
    let mut v = ArrayVec::from([0; 8]);
    for (i, elt) in v.iter_mut().enumerate() {
        *elt = i;
    }
    v.retain(|_| true);
    match (&&v[..], &&[0, 1, 2, 3, 4, 5, 6, 7]) {
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
    v.retain(|elt| {
        *elt /= 2;
        *elt % 2 == 0
    });
    match (&&v[..], &&[0, 0, 2, 2]) {
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
    v.retain(|_| false);
    match (&&v[..], &&[]) {
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
#[rustc_test_marker = "test_drain_oob"]
#[doc(hidden)]
pub const test_drain_oob: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drain_oob"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 393usize,
        start_col: 4usize,
        end_line: 393usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drain_oob()),
    ),
};
#[should_panic]
fn test_drain_oob() {
    let mut v = ArrayVec::from([0; 8]);
    v.pop();
    v.drain(0..8);
}
extern crate test;
#[rustc_test_marker = "test_drop_panic"]
#[doc(hidden)]
pub const test_drop_panic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drop_panic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 401usize,
        start_col: 4usize,
        end_line: 401usize,
        end_col: 19usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drop_panic()),
    ),
};
#[should_panic]
fn test_drop_panic() {
    struct DropPanic;
    impl Drop for DropPanic {
        fn drop(&mut self) {
            {
                ::std::rt::begin_panic("drop");
            };
        }
    }
    let mut array = ArrayVec::<DropPanic, 1>::new();
    array.push(DropPanic);
}
extern crate test;
#[rustc_test_marker = "test_drop_panic_into_iter"]
#[doc(hidden)]
pub const test_drop_panic_into_iter: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drop_panic_into_iter"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 416usize,
        start_col: 4usize,
        end_line: 416usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drop_panic_into_iter()),
    ),
};
#[should_panic]
fn test_drop_panic_into_iter() {
    struct DropPanic;
    impl Drop for DropPanic {
        fn drop(&mut self) {
            {
                ::std::rt::begin_panic("drop");
            };
        }
    }
    let mut array = ArrayVec::<DropPanic, 1>::new();
    array.push(DropPanic);
    array.into_iter();
}
extern crate test;
#[rustc_test_marker = "test_insert"]
#[doc(hidden)]
pub const test_insert: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_insert"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 431usize,
        start_col: 4usize,
        end_line: 431usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_insert()),
    ),
};
fn test_insert() {
    let mut v = ArrayVec::from([]);
    match v.try_push(1) {
        Err(_) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(_)",
                ),
            );
        }
    };
    let mut v = ArrayVec::<_, 3>::new();
    v.insert(0, 0);
    v.insert(1, 1);
    match (&&v[..], &&[0, 1]) {
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
    v.insert(2, 2);
    match (&&v[..], &&[0, 1, 2]) {
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
    let ret2 = v.try_insert(1, 9);
    match (&&v[..], &&[0, 1, 2]) {
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
    match ret2 {
        Err(_) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(_)",
                ),
            );
        }
    };
    let mut v = ArrayVec::from([2]);
    match v.try_insert(0, 1) {
        Err(CapacityError { .. }) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(CapacityError { .. })",
                ),
            );
        }
    };
    match v.try_insert(1, 1) {
        Err(CapacityError { .. }) => {}
        ref e => {
            ::std::rt::panic_fmt(
                format_args!(
                    "assertion failed: `{0:?}` does not match `{1}`",
                    e,
                    "Err(CapacityError { .. })",
                ),
            );
        }
    };
}
extern crate test;
#[rustc_test_marker = "test_into_inner_1"]
#[doc(hidden)]
pub const test_into_inner_1: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_into_inner_1"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 455usize,
        start_col: 4usize,
        end_line: 455usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_into_inner_1()),
    ),
};
fn test_into_inner_1() {
    let mut v = ArrayVec::from([1, 2]);
    v.pop();
    let u = v.clone();
    match (&v.into_inner(), &Err(u)) {
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
#[rustc_test_marker = "test_into_inner_2"]
#[doc(hidden)]
pub const test_into_inner_2: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_into_inner_2"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 463usize,
        start_col: 4usize,
        end_line: 463usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_into_inner_2()),
    ),
};
fn test_into_inner_2() {
    let mut v = ArrayVec::<String, 4>::new();
    v.push("a".into());
    v.push("b".into());
    v.push("c".into());
    v.push("d".into());
    match (&v.into_inner().unwrap(), &["a", "b", "c", "d"]) {
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
#[rustc_test_marker = "test_into_inner_3"]
#[doc(hidden)]
pub const test_into_inner_3: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_into_inner_3"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 473usize,
        start_col: 4usize,
        end_line: 473usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_into_inner_3()),
    ),
};
fn test_into_inner_3() {
    let mut v = ArrayVec::<i32, 4>::new();
    v.extend(1..=4);
    match (&v.into_inner().unwrap(), &[1, 2, 3, 4]) {
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
#[rustc_test_marker = "test_take"]
#[doc(hidden)]
pub const test_take: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_take"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 480usize,
        start_col: 4usize,
        end_line: 480usize,
        end_col: 13usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_take())),
};
fn test_take() {
    let mut v1 = ArrayVec::<i32, 4>::new();
    v1.extend(1..=4);
    let v2 = v1.take();
    if !v1.into_inner().is_err() {
        ::core::panicking::panic("assertion failed: v1.into_inner().is_err()")
    }
    match (&v2.into_inner().unwrap(), &[1, 2, 3, 4]) {
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
#[rustc_test_marker = "test_write"]
#[doc(hidden)]
pub const test_write: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_write"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 490usize,
        start_col: 4usize,
        end_line: 490usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_write()),
    ),
};
fn test_write() {
    use std::io::Write;
    let mut v = ArrayVec::<_, 8>::new();
    (&mut v).write_fmt(format_args!("\u{1}\u{2}\u{3}")).unwrap();
    match (&&v[..], &&[1, 2, 3]) {
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
    let r = v.write(&[9; 16]).unwrap();
    match (&r, &5) {
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
    match (&&v[..], &&[1, 2, 3, 9, 9, 9, 9, 9]) {
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
#[rustc_test_marker = "array_clone_from"]
#[doc(hidden)]
pub const array_clone_from: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("array_clone_from"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 501usize,
        start_col: 4usize,
        end_line: 501usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(array_clone_from()),
    ),
};
fn array_clone_from() {
    let mut v = ArrayVec::<_, 4>::new();
    v.push(<[_]>::into_vec(::alloc::boxed::box_new([1, 2])));
    v.push(<[_]>::into_vec(::alloc::boxed::box_new([3, 4, 5])));
    v.push(<[_]>::into_vec(::alloc::boxed::box_new([6])));
    let reference = v.to_vec();
    let mut u = ArrayVec::<_, 4>::new();
    u.clone_from(&v);
    match (&&u, &&reference[..]) {
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
    let mut t = ArrayVec::<_, 4>::new();
    t.push(<[_]>::into_vec(::alloc::boxed::box_new([97])));
    t.push(::alloc::vec::Vec::new());
    t.push(<[_]>::into_vec(::alloc::boxed::box_new([5, 6, 2])));
    t.push(<[_]>::into_vec(::alloc::boxed::box_new([2])));
    t.clone_from(&v);
    match (&&t, &&reference[..]) {
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
    t.clear();
    t.clone_from(&v);
    match (&&t, &&reference[..]) {
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
#[rustc_test_marker = "test_string"]
#[doc(hidden)]
pub const test_string: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 525usize,
        start_col: 4usize,
        end_line: 525usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string()),
    ),
};
fn test_string() {
    use std::error::Error;
    let text = "hello world";
    let mut s = ArrayString::<16>::new();
    s.try_push_str(text).unwrap();
    match (&&s, &text) {
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
    match (&text, &&s) {
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
    let mut map = HashMap::new();
    map.insert(s, 1);
    match (&map[text], &1) {
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
    let mut t = ArrayString::<2>::new();
    if !t.try_push_str(text).is_err() {
        ::core::panicking::panic("assertion failed: t.try_push_str(text).is_err()")
    }
    match (&&t, &"") {
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
    t.push_str("ab");
    let tmut: &mut str = &mut t;
    match (&tmut, &"ab") {
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
    let t = || -> Result<(), Box<dyn Error>> {
        let mut t = ArrayString::<2>::new();
        t.try_push_str(text)?;
        Ok(())
    }();
    if !t.is_err() {
        ::core::panicking::panic("assertion failed: t.is_err()")
    }
}
extern crate test;
#[rustc_test_marker = "test_string_from"]
#[doc(hidden)]
pub const test_string_from: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string_from"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 558usize,
        start_col: 4usize,
        end_line: 558usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string_from()),
    ),
};
fn test_string_from() {
    let text = "hello world";
    let u = ArrayString::<11>::from(text).unwrap();
    match (&&u, &text) {
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
    match (&u.len(), &text.len()) {
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
#[rustc_test_marker = "test_string_parse_from_str"]
#[doc(hidden)]
pub const test_string_parse_from_str: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string_parse_from_str"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 567usize,
        start_col: 4usize,
        end_line: 567usize,
        end_col: 30usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string_parse_from_str()),
    ),
};
fn test_string_parse_from_str() {
    let text = "hello world";
    let u: ArrayString<11> = text.parse().unwrap();
    match (&&u, &text) {
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
    match (&u.len(), &text.len()) {
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
#[rustc_test_marker = "test_string_from_bytes"]
#[doc(hidden)]
pub const test_string_from_bytes: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string_from_bytes"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 575usize,
        start_col: 4usize,
        end_line: 575usize,
        end_col: 26usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string_from_bytes()),
    ),
};
fn test_string_from_bytes() {
    let text = "hello world";
    let u = ArrayString::from_byte_string(b"hello world").unwrap();
    match (&&u, &text) {
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
    match (&u.len(), &text.len()) {
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
#[rustc_test_marker = "test_string_clone"]
#[doc(hidden)]
pub const test_string_clone: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string_clone"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 583usize,
        start_col: 4usize,
        end_line: 583usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string_clone()),
    ),
};
fn test_string_clone() {
    let text = "hi";
    let mut s = ArrayString::<4>::new();
    s.push_str("abcd");
    let t = ArrayString::<4>::from(text).unwrap();
    s.clone_from(&t);
    match (&&t, &&s) {
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
#[rustc_test_marker = "test_string_push"]
#[doc(hidden)]
pub const test_string_push: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_string_push"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 593usize,
        start_col: 4usize,
        end_line: 593usize,
        end_col: 20usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_string_push()),
    ),
};
fn test_string_push() {
    let text = "abcαβγ";
    let mut s = ArrayString::<8>::new();
    for c in text.chars() {
        if let Err(_) = s.try_push(c) {
            break;
        }
    }
    match (&"abcαβ", &&s[..]) {
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
    s.push('x');
    match (&"abcαβx", &&s[..]) {
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
    if !s.try_push('x').is_err() {
        ::core::panicking::panic("assertion failed: s.try_push(\'x\').is_err()")
    }
}
extern crate test;
#[rustc_test_marker = "test_insert_at_length"]
#[doc(hidden)]
pub const test_insert_at_length: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_insert_at_length"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 609usize,
        start_col: 4usize,
        end_line: 609usize,
        end_col: 25usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_insert_at_length()),
    ),
};
fn test_insert_at_length() {
    let mut v = ArrayVec::<_, 8>::new();
    let result1 = v.try_insert(0, "a");
    let result2 = v.try_insert(1, "b");
    if !(result1.is_ok() && result2.is_ok()) {
        ::core::panicking::panic("assertion failed: result1.is_ok() && result2.is_ok()")
    }
    match (&&v[..], &&["a", "b"]) {
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
#[rustc_test_marker = "test_insert_out_of_bounds"]
#[doc(hidden)]
pub const test_insert_out_of_bounds: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_insert_out_of_bounds"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 619usize,
        start_col: 4usize,
        end_line: 619usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::Yes,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_insert_out_of_bounds()),
    ),
};
#[should_panic]
fn test_insert_out_of_bounds() {
    let mut v = ArrayVec::<_, 8>::new();
    let _ = v.try_insert(1, "test");
}
extern crate test;
#[rustc_test_marker = "test_drop_in_insert"]
#[doc(hidden)]
pub const test_drop_in_insert: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_drop_in_insert"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 636usize,
        start_col: 4usize,
        end_line: 636usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_drop_in_insert()),
    ),
};
fn test_drop_in_insert() {
    use std::cell::Cell;
    let flag = &Cell::new(0);
    struct Bump<'a>(&'a Cell<i32>);
    impl<'a> Drop for Bump<'a> {
        fn drop(&mut self) {
            let n = self.0.get();
            self.0.set(n + 1);
        }
    }
    flag.set(0);
    {
        let mut array = ArrayVec::<_, 2>::new();
        array.push(Bump(flag));
        array.insert(0, Bump(flag));
        match (&flag.get(), &0) {
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
        let ret = array.try_insert(1, Bump(flag));
        match (&flag.get(), &0) {
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
        match ret {
            Err(_) => {}
            ref e => {
                ::std::rt::panic_fmt(
                    format_args!(
                        "assertion failed: `{0:?}` does not match `{1}`",
                        e,
                        "Err(_)",
                    ),
                );
            }
        };
        drop(ret);
        match (&flag.get(), &1) {
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
    match (&flag.get(), &3) {
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
#[rustc_test_marker = "test_pop_at"]
#[doc(hidden)]
pub const test_pop_at: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_pop_at"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 667usize,
        start_col: 4usize,
        end_line: 667usize,
        end_col: 15usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_pop_at()),
    ),
};
fn test_pop_at() {
    let mut v = ArrayVec::<String, 4>::new();
    let s = String::from;
    v.push(s("a"));
    v.push(s("b"));
    v.push(s("c"));
    v.push(s("d"));
    match (&v.pop_at(4), &None) {
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
    match (&v.pop_at(1), &Some(s("b"))) {
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
    match (&v.pop_at(1), &Some(s("c"))) {
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
    match (&v.pop_at(2), &None) {
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
    match (&&v[..], &&["a", "d"]) {
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
#[rustc_test_marker = "test_sizes"]
#[doc(hidden)]
pub const test_sizes: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_sizes"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 683usize,
        start_col: 4usize,
        end_line: 683usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_sizes()),
    ),
};
fn test_sizes() {
    let v = ArrayVec::from([0u8; 1 << 16]);
    match (&::alloc::vec::from_elem(0u8, v.len()), &&v[..]) {
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
#[rustc_test_marker = "test_default"]
#[doc(hidden)]
pub const test_default: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_default"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 689usize,
        start_col: 4usize,
        end_line: 689usize,
        end_col: 16usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_default()),
    ),
};
fn test_default() {
    use std::net;
    let s: ArrayString<4> = Default::default();
    let v: ArrayVec<net::TcpStream, 4> = Default::default();
    match (&s.len(), &0) {
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
    match (&v.len(), &0) {
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
#[rustc_test_marker = "test_extend_zst"]
#[doc(hidden)]
pub const test_extend_zst: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_extend_zst"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 699usize,
        start_col: 4usize,
        end_line: 699usize,
        end_col: 19usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_extend_zst()),
    ),
};
fn test_extend_zst() {
    let mut range = 0..10;
    struct Z;
    #[automatically_derived]
    impl ::core::marker::Copy for Z {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Z {}
    #[automatically_derived]
    impl ::core::clone::Clone for Z {
        #[inline]
        fn clone(&self) -> Z {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Z {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Z {
        #[inline]
        fn eq(&self, other: &Z) -> bool {
            true
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Z {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "Z")
        }
    }
    let mut array: ArrayVec<_, 5> = range.by_ref().take(5).map(|_| Z).collect();
    match (&&array[..], &&[Z; 5]) {
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
    match (&range.next(), &Some(5)) {
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
    array.extend(range.by_ref().take(0).map(|_| Z));
    match (&range.next(), &Some(6)) {
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
    let mut array: ArrayVec<_, 10> = (0..3).map(|_| Z).collect();
    match (&&array[..], &&[Z; 3]) {
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
    array.extend((3..5).map(|_| Z));
    match (&&array[..], &&[Z; 5]) {
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
    match (&array.len(), &5) {
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
#[rustc_test_marker = "test_try_from_argument"]
#[doc(hidden)]
pub const test_try_from_argument: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_try_from_argument"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 719usize,
        start_col: 4usize,
        end_line: 719usize,
        end_col: 26usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_try_from_argument()),
    ),
};
fn test_try_from_argument() {
    use core::convert::TryFrom;
    let v = ArrayString::<16>::try_from(format_args!("Hello {0}", 123)).unwrap();
    match (&&v, &"Hello 123") {
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
#[rustc_test_marker = "allow_max_capacity_arrayvec_type"]
#[doc(hidden)]
pub const allow_max_capacity_arrayvec_type: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("allow_max_capacity_arrayvec_type"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 726usize,
        start_col: 4usize,
        end_line: 726usize,
        end_col: 36usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(allow_max_capacity_arrayvec_type()),
    ),
};
fn allow_max_capacity_arrayvec_type() {
    let _v: ArrayVec<(), { usize::MAX }>;
}
extern crate test;
#[rustc_test_marker = "deny_max_capacity_arrayvec_value"]
#[doc(hidden)]
pub const deny_max_capacity_arrayvec_value: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("deny_max_capacity_arrayvec_value"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 733usize,
        start_col: 4usize,
        end_line: 733usize,
        end_col: 36usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::YesWithMessage("largest supported capacity"),
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(deny_max_capacity_arrayvec_value()),
    ),
};
#[should_panic(expected = "largest supported capacity")]
fn deny_max_capacity_arrayvec_value() {
    if mem::size_of::<usize>() <= mem::size_of::<u32>() {
        {
            ::std::rt::begin_panic(
                "This test does not work on this platform. 'largest supported capacity'",
            );
        };
    }
    let _v: ArrayVec<(), { usize::MAX }> = ArrayVec::new();
}
extern crate test;
#[rustc_test_marker = "deny_max_capacity_arrayvec_value_const"]
#[doc(hidden)]
pub const deny_max_capacity_arrayvec_value_const: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("deny_max_capacity_arrayvec_value_const"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 743usize,
        start_col: 4usize,
        end_line: 743usize,
        end_col: 42usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::YesWithMessage("index out of bounds"),
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(deny_max_capacity_arrayvec_value_const()),
    ),
};
#[should_panic(expected = "index out of bounds")]
fn deny_max_capacity_arrayvec_value_const() {
    if mem::size_of::<usize>() <= mem::size_of::<u32>() {
        {
            ::std::rt::begin_panic(
                "This test does not work on this platform. 'index out of bounds'",
            );
        };
    }
    let _v: ArrayVec<(), { usize::MAX }> = ArrayVec::new_const();
}
extern crate test;
#[rustc_test_marker = "test_arrayvec_const_constructible"]
#[doc(hidden)]
pub const test_arrayvec_const_constructible: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_arrayvec_const_constructible"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 752usize,
        start_col: 4usize,
        end_line: 752usize,
        end_col: 37usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_arrayvec_const_constructible()),
    ),
};
fn test_arrayvec_const_constructible() {
    const OF_U8: ArrayVec<Vec<u8>, 10> = ArrayVec::new_const();
    let mut var = OF_U8;
    if !var.is_empty() {
        ::core::panicking::panic("assertion failed: var.is_empty()")
    }
    match (&var, &ArrayVec::new()) {
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
    var.push(<[_]>::into_vec(::alloc::boxed::box_new([3, 5, 8])));
    match (&var[..], &[<[_]>::into_vec(::alloc::boxed::box_new([3, 5, 8]))]) {
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
#[rustc_test_marker = "test_arraystring_const_constructible"]
#[doc(hidden)]
pub const test_arraystring_const_constructible: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_arraystring_const_constructible"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 763usize,
        start_col: 4usize,
        end_line: 763usize,
        end_col: 40usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_arraystring_const_constructible()),
    ),
};
fn test_arraystring_const_constructible() {
    const AS: ArrayString<10> = ArrayString::new_const();
    let mut var = AS;
    if !var.is_empty() {
        ::core::panicking::panic("assertion failed: var.is_empty()")
    }
    match (&var, &ArrayString::new()) {
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
    var.push_str("hello");
    match (&var, &*"hello") {
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
#[rustc_test_marker = "test_arraystring_zero_filled_has_some_sanity_checks"]
#[doc(hidden)]
pub const test_arraystring_zero_filled_has_some_sanity_checks: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName(
            "test_arraystring_zero_filled_has_some_sanity_checks",
        ),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/tests.rs",
        start_line: 775usize,
        start_col: 4usize,
        end_line: 775usize,
        end_col: 55usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(
            test_arraystring_zero_filled_has_some_sanity_checks(),
        ),
    ),
};
fn test_arraystring_zero_filled_has_some_sanity_checks() {
    let string = ArrayString::<4>::zero_filled();
    match (&string.as_str(), &"\0\0\0\0") {
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
    match (&string.len(), &4) {
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
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &allow_max_capacity_arrayvec_type,
            &array_clone_from,
            &deny_max_capacity_arrayvec_value,
            &deny_max_capacity_arrayvec_value_const,
            &test_arraystring_const_constructible,
            &test_arraystring_zero_filled_has_some_sanity_checks,
            &test_arrayvec_const_constructible,
            &test_capacity_left,
            &test_compact_size,
            &test_default,
            &test_drain,
            &test_drain_oob,
            &test_drain_range_inclusive,
            &test_drain_range_inclusive_oob,
            &test_drop,
            &test_drop_in_insert,
            &test_drop_panic,
            &test_drop_panic_into_iter,
            &test_drop_panics,
            &test_extend,
            &test_extend_capacity_panic_1,
            &test_extend_capacity_panic_2,
            &test_extend_from_slice,
            &test_extend_from_slice_error,
            &test_extend_zst,
            &test_insert,
            &test_insert_at_length,
            &test_insert_out_of_bounds,
            &test_into_inner_1,
            &test_into_inner_2,
            &test_into_inner_3,
            &test_is_send_sync,
            &test_iter,
            &test_pop_at,
            &test_retain,
            &test_simple,
            &test_sizes,
            &test_still_works_with_option_arrayvec,
            &test_string,
            &test_string_clone,
            &test_string_from,
            &test_string_from_bytes,
            &test_string_parse_from_str,
            &test_string_push,
            &test_take,
            &test_try_from_argument,
            &test_try_from_slice_error,
            &test_u16_index,
            &test_write,
        ],
    )
}
