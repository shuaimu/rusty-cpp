#![feature(prelude_import)]
//! Small vectors in various sizes. These store a certain number of elements inline, and fall back
//! to the heap for larger allocations.  This can be a useful optimization for improving cache
//! locality and reducing allocator traffic for workloads that fit within the inline buffer.
//!
//! ## `no_std` support
//!
//! By default, `smallvec` does not depend on `std`.  However, the optional
//! `write` feature implements the `std::io::Write` trait for vectors of `u8`.
//! When this feature is enabled, `smallvec` depends on `std`.
//!
//! ## Optional features
//!
//! ### `serde`
//!
//! When this optional dependency is enabled, `SmallVec` implements the `serde::Serialize` and
//! `serde::Deserialize` traits.
//!
//! ### `write`
//!
//! When this feature is enabled, `SmallVec<[u8; _]>` implements the `std::io::Write` trait.
//! This feature is not compatible with `#![no_std]` programs.
//!
//! ### `union`
//!
//! **This feature requires Rust 1.49.**
//!
//! When the `union` feature is enabled `smallvec` will track its state (inline or spilled)
//! without the use of an enum tag, reducing the size of the `smallvec` by one machine word.
//! This means that there is potentially no space overhead compared to `Vec`.
//! Note that `smallvec` can still be larger than `Vec` if the inline buffer is larger than two
//! machine words.
//!
//! To use this feature add `features = ["union"]` in the `smallvec` section of Cargo.toml.
//! Note that this feature requires Rust 1.49.
//!
//! Tracking issue: [rust-lang/rust#55149](https://github.com/rust-lang/rust/issues/55149)
//!
//! ### `const_generics`
//!
//! **This feature requires Rust 1.51.**
//!
//! When this feature is enabled, `SmallVec` works with any arrays of any size, not just a fixed
//! list of sizes.
//!
//! ### `const_new`
//!
//! **This feature requires Rust 1.51.**
//!
//! This feature exposes the functions [`SmallVec::new_const`], [`SmallVec::from_const`], and [`smallvec_inline`] which enables the `SmallVec` to be initialized from a const context.
//! For details, see the
//! [Rust Reference](https://doc.rust-lang.org/reference/const_eval.html#const-functions).
//!
//! ### `drain_filter`
//!
//! **This feature is unstable.** It may change to match the unstable `drain_filter` method in libstd.
//!
//! Enables the `drain_filter` method, which produces an iterator that calls a user-provided
//! closure to determine which elements of the vector to remove and yield from the iterator.
//!
//! ### `drain_keep_rest`
//!
//! **This feature is unstable.** It may change to match the unstable `drain_keep_rest` method in libstd.
//!
//! Enables the `DrainFilter::keep_rest` method.
//!
//! ### `specialization`
//!
//! **This feature is unstable and requires a nightly build of the Rust toolchain.**
//!
//! When this feature is enabled, `SmallVec::from(slice)` has improved performance for slices
//! of `Copy` types.  (Without this feature, you can use `SmallVec::from_slice` to get optimal
//! performance for `Copy` types.)
//!
//! Tracking issue: [rust-lang/rust#31844](https://github.com/rust-lang/rust/issues/31844)
//!
//! ### `may_dangle`
//!
//! **This feature is unstable and requires a nightly build of the Rust toolchain.**
//!
//! This feature makes the Rust compiler less strict about use of vectors that contain borrowed
//! references. For details, see the
//! [Rustonomicon](https://doc.rust-lang.org/1.42.0/nomicon/dropck.html#an-escape-hatch).
//!
//! Tracking issue: [rust-lang/rust#34761](https://github.com/rust-lang/rust/issues/34761)
#![no_std]
#![deny(missing_docs)]
extern crate core;
#[prelude_import]
use core::prelude::rust_2018::*;
#[doc(hidden)]
pub extern crate alloc;
extern crate std;
mod tests {
    use crate::{smallvec, SmallVec};
    use std::iter::FromIterator;
    use alloc::borrow::ToOwned;
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use alloc::{vec, vec::Vec};
    extern crate test;
    #[rustc_test_marker = "tests::test_zero"]
    #[doc(hidden)]
    pub const test_zero: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_zero"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 11usize,
            start_col: 8usize,
            end_line: 11usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_zero()),
        ),
    };
    pub fn test_zero() {
        let mut v = SmallVec::<[_; 0]>::new();
        if !!v.spilled() {
            ::core::panicking::panic("assertion failed: !v.spilled()")
        }
        v.push(0usize);
        if !v.spilled() {
            ::core::panicking::panic("assertion failed: v.spilled()")
        }
        match (&&*v, &&[0]) {
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
    #[rustc_test_marker = "tests::test_inline"]
    #[doc(hidden)]
    pub const test_inline: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_inline"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 22usize,
            start_col: 8usize,
            end_line: 22usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_inline()),
        ),
    };
    pub fn test_inline() {
        let mut v = SmallVec::<[_; 16]>::new();
        v.push("hello".to_owned());
        v.push("there".to_owned());
        match (&&*v, &&["hello".to_owned(), "there".to_owned()][..]) {
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
    #[rustc_test_marker = "tests::test_spill"]
    #[doc(hidden)]
    pub const test_spill: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_spill"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 30usize,
            start_col: 8usize,
            end_line: 30usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_spill()),
        ),
    };
    pub fn test_spill() {
        let mut v = SmallVec::<[_; 2]>::new();
        v.push("hello".to_owned());
        match (&v[0], &"hello") {
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
        v.push("there".to_owned());
        v.push("burma".to_owned());
        match (&v[0], &"hello") {
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
        v.push("shave".to_owned());
        match (
            &&*v,
            &&[
                "hello".to_owned(),
                "there".to_owned(),
                "burma".to_owned(),
                "shave".to_owned(),
            ][..],
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
    #[rustc_test_marker = "tests::test_double_spill"]
    #[doc(hidden)]
    pub const test_double_spill: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_double_spill"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 50usize,
            start_col: 8usize,
            end_line: 50usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_double_spill()),
        ),
    };
    pub fn test_double_spill() {
        let mut v = SmallVec::<[_; 2]>::new();
        v.push("hello".to_owned());
        v.push("there".to_owned());
        v.push("burma".to_owned());
        v.push("shave".to_owned());
        v.push("hello".to_owned());
        v.push("there".to_owned());
        v.push("burma".to_owned());
        v.push("shave".to_owned());
        match (
            &&*v,
            &&[
                "hello".to_owned(),
                "there".to_owned(),
                "burma".to_owned(),
                "shave".to_owned(),
                "hello".to_owned(),
                "there".to_owned(),
                "burma".to_owned(),
                "shave".to_owned(),
            ][..],
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
    #[rustc_test_marker = "tests::issue_4"]
    #[doc(hidden)]
    pub const issue_4: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::issue_4"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 77usize,
            start_col: 4usize,
            end_line: 77usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(issue_4()),
        ),
    };
    fn issue_4() {
        SmallVec::<[Box<u32>; 2]>::new();
    }
    extern crate test;
    #[rustc_test_marker = "tests::issue_5"]
    #[doc(hidden)]
    pub const issue_5: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::issue_5"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 83usize,
            start_col: 4usize,
            end_line: 83usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(issue_5()),
        ),
    };
    fn issue_5() {
        if !Some(SmallVec::<[&u32; 2]>::new()).is_some() {
            ::core::panicking::panic(
                "assertion failed: Some(SmallVec::<[&u32; 2]>::new()).is_some()",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_with_capacity"]
    #[doc(hidden)]
    pub const test_with_capacity: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_with_capacity"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 88usize,
            start_col: 4usize,
            end_line: 88usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_with_capacity()),
        ),
    };
    fn test_with_capacity() {
        let v: SmallVec<[u8; 3]> = SmallVec::with_capacity(1);
        if !v.is_empty() {
            ::core::panicking::panic("assertion failed: v.is_empty()")
        }
        if !!v.spilled() {
            ::core::panicking::panic("assertion failed: !v.spilled()")
        }
        match (&v.capacity(), &3) {
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
        let v: SmallVec<[u8; 3]> = SmallVec::with_capacity(10);
        if !v.is_empty() {
            ::core::panicking::panic("assertion failed: v.is_empty()")
        }
        if !v.spilled() {
            ::core::panicking::panic("assertion failed: v.spilled()")
        }
        match (&v.capacity(), &10) {
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
    #[rustc_test_marker = "tests::drain"]
    #[doc(hidden)]
    pub const drain: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::drain"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 101usize,
            start_col: 4usize,
            end_line: 101usize,
            end_col: 9usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(drain())),
    };
    fn drain() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        match (&v.drain(..).collect::<Vec<_>>(), &&[3]) {
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
        v.push(3);
        v.push(4);
        v.push(5);
        let old_capacity = v.capacity();
        match (&v.drain(1..).collect::<Vec<_>>(), &&[4, 5]) {
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
        match (&v.capacity(), &old_capacity) {
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
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(1);
        v.push(2);
        match (&v.drain(..1).collect::<Vec<_>>(), &&[1]) {
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
    #[rustc_test_marker = "tests::drain_rev"]
    #[doc(hidden)]
    pub const drain_rev: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::drain_rev"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 124usize,
            start_col: 4usize,
            end_line: 124usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(drain_rev()),
        ),
    };
    fn drain_rev() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        match (&v.drain(..).rev().collect::<Vec<_>>(), &&[3]) {
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
        v.push(3);
        v.push(4);
        v.push(5);
        match (&v.drain(..).rev().collect::<Vec<_>>(), &&[5, 4, 3]) {
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
    #[rustc_test_marker = "tests::drain_forget"]
    #[doc(hidden)]
    pub const drain_forget: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::drain_forget"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 137usize,
            start_col: 4usize,
            end_line: 137usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(drain_forget()),
        ),
    };
    fn drain_forget() {
        let mut v: SmallVec<[u8; 1]> = {
            let count = 0usize + 1usize + 1usize + 1usize + 1usize + 1usize + 1usize
                + 1usize + 1usize;
            let mut vec = crate::SmallVec::new();
            if count <= vec.inline_size() {
                vec.push(0);
                vec.push(1);
                vec.push(2);
                vec.push(3);
                vec.push(4);
                vec.push(5);
                vec.push(6);
                vec.push(7);
                vec
            } else {
                crate::SmallVec::from_vec(
                    <[_]>::into_vec(::alloc::boxed::box_new([0, 1, 2, 3, 4, 5, 6, 7])),
                )
            }
        };
        std::mem::forget(v.drain(2..5));
        match (&v.len(), &2) {
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
    #[rustc_test_marker = "tests::into_iter"]
    #[doc(hidden)]
    pub const into_iter: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::into_iter"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 144usize,
            start_col: 4usize,
            end_line: 144usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(into_iter()),
        ),
    };
    fn into_iter() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        match (&v.into_iter().collect::<Vec<_>>(), &&[3]) {
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
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        v.push(4);
        v.push(5);
        match (&v.into_iter().collect::<Vec<_>>(), &&[3, 4, 5]) {
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
    #[rustc_test_marker = "tests::into_iter_rev"]
    #[doc(hidden)]
    pub const into_iter_rev: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::into_iter_rev"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 158usize,
            start_col: 4usize,
            end_line: 158usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(into_iter_rev()),
        ),
    };
    fn into_iter_rev() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        match (&v.into_iter().rev().collect::<Vec<_>>(), &&[3]) {
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
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(3);
        v.push(4);
        v.push(5);
        match (&v.into_iter().rev().collect::<Vec<_>>(), &&[5, 4, 3]) {
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
    #[rustc_test_marker = "tests::into_iter_drop"]
    #[doc(hidden)]
    pub const into_iter_drop: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::into_iter_drop"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 172usize,
            start_col: 4usize,
            end_line: 172usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(into_iter_drop()),
        ),
    };
    fn into_iter_drop() {
        use std::cell::Cell;
        struct DropCounter<'a>(&'a Cell<i32>);
        impl<'a> Drop for DropCounter<'a> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        {
            let cell = Cell::new(0);
            let mut v: SmallVec<[DropCounter<'_>; 2]> = SmallVec::new();
            v.push(DropCounter(&cell));
            v.into_iter();
            match (&cell.get(), &1) {
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
        {
            let cell = Cell::new(0);
            let mut v: SmallVec<[DropCounter<'_>; 2]> = SmallVec::new();
            v.push(DropCounter(&cell));
            v.push(DropCounter(&cell));
            if !v.into_iter().next().is_some() {
                ::core::panicking::panic(
                    "assertion failed: v.into_iter().next().is_some()",
                )
            }
            match (&cell.get(), &2) {
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
        {
            let cell = Cell::new(0);
            let mut v: SmallVec<[DropCounter<'_>; 2]> = SmallVec::new();
            v.push(DropCounter(&cell));
            v.push(DropCounter(&cell));
            v.push(DropCounter(&cell));
            if !v.into_iter().next().is_some() {
                ::core::panicking::panic(
                    "assertion failed: v.into_iter().next().is_some()",
                )
            }
            match (&cell.get(), &3) {
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
        {
            let cell = Cell::new(0);
            let mut v: SmallVec<[DropCounter<'_>; 2]> = SmallVec::new();
            v.push(DropCounter(&cell));
            v.push(DropCounter(&cell));
            v.push(DropCounter(&cell));
            {
                let mut it = v.into_iter();
                if !it.next().is_some() {
                    ::core::panicking::panic("assertion failed: it.next().is_some()")
                }
                if !it.next_back().is_some() {
                    ::core::panicking::panic(
                        "assertion failed: it.next_back().is_some()",
                    )
                }
            }
            match (&cell.get(), &3) {
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
    #[rustc_test_marker = "tests::test_capacity"]
    #[doc(hidden)]
    pub const test_capacity: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_capacity"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 225usize,
            start_col: 4usize,
            end_line: 225usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_capacity()),
        ),
    };
    fn test_capacity() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.reserve(1);
        match (&v.capacity(), &2) {
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
        if !!v.spilled() {
            ::core::panicking::panic("assertion failed: !v.spilled()")
        }
        v.reserve_exact(0x100);
        if !(v.capacity() >= 0x100) {
            ::core::panicking::panic("assertion failed: v.capacity() >= 0x100")
        }
        v.push(0);
        v.push(1);
        v.push(2);
        v.push(3);
        v.shrink_to_fit();
        if !(v.capacity() < 0x100) {
            ::core::panicking::panic("assertion failed: v.capacity() < 0x100")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_truncate"]
    #[doc(hidden)]
    pub const test_truncate: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_truncate"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 244usize,
            start_col: 4usize,
            end_line: 244usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_truncate()),
        ),
    };
    fn test_truncate() {
        let mut v: SmallVec<[Box<u8>; 8]> = SmallVec::new();
        for x in 0..8 {
            v.push(Box::new(x));
        }
        v.truncate(4);
        match (&v.len(), &4) {
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
        if !!v.spilled() {
            ::core::panicking::panic("assertion failed: !v.spilled()")
        }
        match (&*v.swap_remove(1), &1) {
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
        match (&*v.remove(1), &3) {
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
        v.insert(1, Box::new(3));
        match (&&v.iter().map(|v| **v).collect::<Vec<_>>(), &&[0, 3, 2]) {
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
    #[rustc_test_marker = "tests::test_insert_many"]
    #[doc(hidden)]
    pub const test_insert_many: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_many"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 263usize,
            start_col: 4usize,
            end_line: 263usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_many()),
        ),
    };
    fn test_insert_many() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        for x in 0..4 {
            v.push(x);
        }
        match (&v.len(), &4) {
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
        v.insert_many(1, [5, 6].iter().cloned());
        match (&&v.iter().map(|v| *v).collect::<Vec<_>>(), &&[0, 5, 6, 1, 2, 3]) {
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
    struct MockHintIter<T: Iterator> {
        x: T,
        hint: usize,
    }
    impl<T: Iterator> Iterator for MockHintIter<T> {
        type Item = T::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.x.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.hint, None)
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_insert_many_short_hint"]
    #[doc(hidden)]
    pub const test_insert_many_short_hint: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_many_short_hint"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 291usize,
            start_col: 4usize,
            end_line: 291usize,
            end_col: 31usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_many_short_hint()),
        ),
    };
    fn test_insert_many_short_hint() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        for x in 0..4 {
            v.push(x);
        }
        match (&v.len(), &4) {
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
        v.insert_many(
            1,
            MockHintIter {
                x: [5, 6].iter().cloned(),
                hint: 5,
            },
        );
        match (&&v.iter().map(|v| *v).collect::<Vec<_>>(), &&[0, 5, 6, 1, 2, 3]) {
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
    #[rustc_test_marker = "tests::test_insert_many_long_hint"]
    #[doc(hidden)]
    pub const test_insert_many_long_hint: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_many_long_hint"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 311usize,
            start_col: 4usize,
            end_line: 311usize,
            end_col: 30usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_many_long_hint()),
        ),
    };
    fn test_insert_many_long_hint() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        for x in 0..4 {
            v.push(x);
        }
        match (&v.len(), &4) {
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
        v.insert_many(
            1,
            MockHintIter {
                x: [5, 6].iter().cloned(),
                hint: 1,
            },
        );
        match (&&v.iter().map(|v| *v).collect::<Vec<_>>(), &&[0, 5, 6, 1, 2, 3]) {
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
    mod insert_many_panic {
        use crate::{smallvec, SmallVec};
        use alloc::boxed::Box;
        struct PanicOnDoubleDrop {
            dropped: Box<bool>,
        }
        impl PanicOnDoubleDrop {
            fn new() -> Self {
                Self { dropped: Box::new(false) }
            }
        }
        impl Drop for PanicOnDoubleDrop {
            fn drop(&mut self) {
                if !!*self.dropped {
                    ::core::panicking::panic("already dropped")
                }
                *self.dropped = true;
            }
        }
        /// Claims to yield `hint` items, but actually yields `count`, then panics.
        struct BadIter {
            hint: usize,
            count: usize,
        }
        impl Iterator for BadIter {
            type Item = PanicOnDoubleDrop;
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.hint, None)
            }
            fn next(&mut self) -> Option<Self::Item> {
                if self.count == 0 {
                    ::core::panicking::panic("explicit panic")
                }
                self.count -= 1;
                Some(PanicOnDoubleDrop::new())
            }
        }
        extern crate test;
        #[rustc_test_marker = "tests::insert_many_panic::panic_early_at_start"]
        #[doc(hidden)]
        pub const panic_early_at_start: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName(
                    "tests::insert_many_panic::panic_early_at_start",
                ),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests.rs",
                start_line: 375usize,
                start_col: 8usize,
                end_line: 375usize,
                end_col: 28usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(panic_early_at_start()),
            ),
        };
        fn panic_early_at_start() {
            let mut vec: SmallVec<[PanicOnDoubleDrop; 0]> = {
                let count = 0usize + 1usize + 1usize;
                let mut vec = crate::SmallVec::new();
                if count <= vec.inline_size() {
                    vec.push(PanicOnDoubleDrop::new());
                    vec.push(PanicOnDoubleDrop::new());
                    vec
                } else {
                    crate::SmallVec::from_vec(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                PanicOnDoubleDrop::new(),
                                PanicOnDoubleDrop::new(),
                            ]),
                        ),
                    )
                }
            };
            let result = ::std::panic::catch_unwind(move || {
                vec.insert_many(0, BadIter { hint: 1, count: 0 });
            });
            if !result.is_err() {
                ::core::panicking::panic("assertion failed: result.is_err()")
            }
        }
        extern crate test;
        #[rustc_test_marker = "tests::insert_many_panic::panic_early_in_middle"]
        #[doc(hidden)]
        pub const panic_early_in_middle: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName(
                    "tests::insert_many_panic::panic_early_in_middle",
                ),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests.rs",
                start_line: 385usize,
                start_col: 8usize,
                end_line: 385usize,
                end_col: 29usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(panic_early_in_middle()),
            ),
        };
        fn panic_early_in_middle() {
            let mut vec: SmallVec<[PanicOnDoubleDrop; 0]> = {
                let count = 0usize + 1usize + 1usize;
                let mut vec = crate::SmallVec::new();
                if count <= vec.inline_size() {
                    vec.push(PanicOnDoubleDrop::new());
                    vec.push(PanicOnDoubleDrop::new());
                    vec
                } else {
                    crate::SmallVec::from_vec(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                PanicOnDoubleDrop::new(),
                                PanicOnDoubleDrop::new(),
                            ]),
                        ),
                    )
                }
            };
            let result = ::std::panic::catch_unwind(move || {
                vec.insert_many(1, BadIter { hint: 4, count: 2 });
            });
            if !result.is_err() {
                ::core::panicking::panic("assertion failed: result.is_err()")
            }
        }
        extern crate test;
        #[rustc_test_marker = "tests::insert_many_panic::panic_early_at_end"]
        #[doc(hidden)]
        pub const panic_early_at_end: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName(
                    "tests::insert_many_panic::panic_early_at_end",
                ),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests.rs",
                start_line: 395usize,
                start_col: 8usize,
                end_line: 395usize,
                end_col: 26usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(panic_early_at_end()),
            ),
        };
        fn panic_early_at_end() {
            let mut vec: SmallVec<[PanicOnDoubleDrop; 0]> = {
                let count = 0usize + 1usize + 1usize;
                let mut vec = crate::SmallVec::new();
                if count <= vec.inline_size() {
                    vec.push(PanicOnDoubleDrop::new());
                    vec.push(PanicOnDoubleDrop::new());
                    vec
                } else {
                    crate::SmallVec::from_vec(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                PanicOnDoubleDrop::new(),
                                PanicOnDoubleDrop::new(),
                            ]),
                        ),
                    )
                }
            };
            let result = ::std::panic::catch_unwind(move || {
                vec.insert_many(2, BadIter { hint: 3, count: 1 });
            });
            if !result.is_err() {
                ::core::panicking::panic("assertion failed: result.is_err()")
            }
        }
        extern crate test;
        #[rustc_test_marker = "tests::insert_many_panic::panic_late_at_start"]
        #[doc(hidden)]
        pub const panic_late_at_start: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName(
                    "tests::insert_many_panic::panic_late_at_start",
                ),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests.rs",
                start_line: 405usize,
                start_col: 8usize,
                end_line: 405usize,
                end_col: 27usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(panic_late_at_start()),
            ),
        };
        fn panic_late_at_start() {
            let mut vec: SmallVec<[PanicOnDoubleDrop; 0]> = {
                let count = 0usize + 1usize + 1usize;
                let mut vec = crate::SmallVec::new();
                if count <= vec.inline_size() {
                    vec.push(PanicOnDoubleDrop::new());
                    vec.push(PanicOnDoubleDrop::new());
                    vec
                } else {
                    crate::SmallVec::from_vec(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                PanicOnDoubleDrop::new(),
                                PanicOnDoubleDrop::new(),
                            ]),
                        ),
                    )
                }
            };
            let result = ::std::panic::catch_unwind(move || {
                vec.insert_many(0, BadIter { hint: 3, count: 5 });
            });
            if !result.is_err() {
                ::core::panicking::panic("assertion failed: result.is_err()")
            }
        }
        extern crate test;
        #[rustc_test_marker = "tests::insert_many_panic::panic_late_at_end"]
        #[doc(hidden)]
        pub const panic_late_at_end: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName(
                    "tests::insert_many_panic::panic_late_at_end",
                ),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/tests.rs",
                start_line: 415usize,
                start_col: 8usize,
                end_line: 415usize,
                end_col: 25usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(panic_late_at_end()),
            ),
        };
        fn panic_late_at_end() {
            let mut vec: SmallVec<[PanicOnDoubleDrop; 0]> = {
                let count = 0usize + 1usize + 1usize;
                let mut vec = crate::SmallVec::new();
                if count <= vec.inline_size() {
                    vec.push(PanicOnDoubleDrop::new());
                    vec.push(PanicOnDoubleDrop::new());
                    vec
                } else {
                    crate::SmallVec::from_vec(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                PanicOnDoubleDrop::new(),
                                PanicOnDoubleDrop::new(),
                            ]),
                        ),
                    )
                }
            };
            let result = ::std::panic::catch_unwind(move || {
                vec.insert_many(2, BadIter { hint: 3, count: 5 });
            });
            if !result.is_err() {
                ::core::panicking::panic("assertion failed: result.is_err()")
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_invalid_grow"]
    #[doc(hidden)]
    pub const test_invalid_grow: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_invalid_grow"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 427usize,
            start_col: 4usize,
            end_line: 427usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_invalid_grow()),
        ),
    };
    #[should_panic]
    fn test_invalid_grow() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        v.extend(0..8);
        v.grow(5);
    }
    extern crate test;
    #[rustc_test_marker = "tests::drain_overflow"]
    #[doc(hidden)]
    pub const drain_overflow: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::drain_overflow"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 435usize,
            start_col: 4usize,
            end_line: 435usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(drain_overflow()),
        ),
    };
    #[should_panic]
    fn drain_overflow() {
        let mut v: SmallVec<[u8; 8]> = {
            let count = 0usize + 1usize;
            let mut vec = crate::SmallVec::new();
            if count <= vec.inline_size() {
                vec.push(0);
                vec
            } else {
                crate::SmallVec::from_vec(<[_]>::into_vec(::alloc::boxed::box_new([0])))
            }
        };
        v.drain(..=std::usize::MAX);
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_insert_from_slice"]
    #[doc(hidden)]
    pub const test_insert_from_slice: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_from_slice"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 441usize,
            start_col: 4usize,
            end_line: 441usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_from_slice()),
        ),
    };
    fn test_insert_from_slice() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        for x in 0..4 {
            v.push(x);
        }
        match (&v.len(), &4) {
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
        v.insert_from_slice(1, &[5, 6]);
        match (&&v.iter().map(|v| *v).collect::<Vec<_>>(), &&[0, 5, 6, 1, 2, 3]) {
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
    #[rustc_test_marker = "tests::test_extend_from_slice"]
    #[doc(hidden)]
    pub const test_extend_from_slice: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_extend_from_slice"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 455usize,
            start_col: 4usize,
            end_line: 455usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_extend_from_slice()),
        ),
    };
    fn test_extend_from_slice() {
        let mut v: SmallVec<[u8; 8]> = SmallVec::new();
        for x in 0..4 {
            v.push(x);
        }
        match (&v.len(), &4) {
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
        v.extend_from_slice(&[5, 6]);
        match (&&v.iter().map(|v| *v).collect::<Vec<_>>(), &&[0, 1, 2, 3, 5, 6]) {
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
    #[rustc_test_marker = "tests::test_drop_panic_smallvec"]
    #[doc(hidden)]
    pub const test_drop_panic_smallvec: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_drop_panic_smallvec"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 470usize,
            start_col: 4usize,
            end_line: 470usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_drop_panic_smallvec()),
        ),
    };
    #[should_panic]
    fn test_drop_panic_smallvec() {
        struct DropPanic;
        impl Drop for DropPanic {
            fn drop(&mut self) {
                ::core::panicking::panic("drop");
            }
        }
        let mut v = SmallVec::<[_; 1]>::new();
        v.push(DropPanic);
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_eq"]
    #[doc(hidden)]
    pub const test_eq: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_eq"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 486usize,
            start_col: 4usize,
            end_line: 486usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_eq()),
        ),
    };
    fn test_eq() {
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        let mut b: SmallVec<[u32; 2]> = SmallVec::new();
        let mut c: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        a.push(2);
        b.push(1);
        b.push(2);
        c.push(3);
        c.push(4);
        if !(a == b) {
            ::core::panicking::panic("assertion failed: a == b")
        }
        if !(a != c) {
            ::core::panicking::panic("assertion failed: a != c")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_ord"]
    #[doc(hidden)]
    pub const test_ord: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_ord"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 505usize,
            start_col: 4usize,
            end_line: 505usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_ord()),
        ),
    };
    fn test_ord() {
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        let mut b: SmallVec<[u32; 2]> = SmallVec::new();
        let mut c: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        b.push(1);
        b.push(1);
        c.push(1);
        c.push(2);
        if !(a < b) {
            ::core::panicking::panic("assertion failed: a < b")
        }
        if !(b > a) {
            ::core::panicking::panic("assertion failed: b > a")
        }
        if !(b < c) {
            ::core::panicking::panic("assertion failed: b < c")
        }
        if !(c > b) {
            ::core::panicking::panic("assertion failed: c > b")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_hash"]
    #[doc(hidden)]
    pub const test_hash: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_hash"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 525usize,
            start_col: 4usize,
            end_line: 525usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_hash()),
        ),
    };
    fn test_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        {
            let mut a: SmallVec<[u32; 2]> = SmallVec::new();
            let b = [1, 2];
            a.extend(b.iter().cloned());
            let mut hasher = DefaultHasher::new();
            match (&a.hash(&mut hasher), &b.hash(&mut hasher)) {
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
        {
            let mut a: SmallVec<[u32; 2]> = SmallVec::new();
            let b = [1, 2, 11, 12];
            a.extend(b.iter().cloned());
            let mut hasher = DefaultHasher::new();
            match (&a.hash(&mut hasher), &b.hash(&mut hasher)) {
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
    #[rustc_test_marker = "tests::test_as_ref"]
    #[doc(hidden)]
    pub const test_as_ref: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_as_ref"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 546usize,
            start_col: 4usize,
            end_line: 546usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_as_ref()),
        ),
    };
    fn test_as_ref() {
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        match (&a.as_ref(), &[1]) {
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
        a.push(2);
        match (&a.as_ref(), &[1, 2]) {
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
        a.push(3);
        match (&a.as_ref(), &[1, 2, 3]) {
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
    #[rustc_test_marker = "tests::test_as_mut"]
    #[doc(hidden)]
    pub const test_as_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_as_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 557usize,
            start_col: 4usize,
            end_line: 557usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_as_mut()),
        ),
    };
    fn test_as_mut() {
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        match (&a.as_mut(), &[1]) {
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
        a.push(2);
        match (&a.as_mut(), &[1, 2]) {
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
        a.push(3);
        match (&a.as_mut(), &[1, 2, 3]) {
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
        a.as_mut()[1] = 4;
        match (&a.as_mut(), &[1, 4, 3]) {
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
    #[rustc_test_marker = "tests::test_borrow"]
    #[doc(hidden)]
    pub const test_borrow: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_borrow"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 570usize,
            start_col: 4usize,
            end_line: 570usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_borrow()),
        ),
    };
    fn test_borrow() {
        use std::borrow::Borrow;
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        match (&a.borrow(), &[1]) {
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
        a.push(2);
        match (&a.borrow(), &[1, 2]) {
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
        a.push(3);
        match (&a.borrow(), &[1, 2, 3]) {
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
    #[rustc_test_marker = "tests::test_borrow_mut"]
    #[doc(hidden)]
    pub const test_borrow_mut: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_borrow_mut"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 583usize,
            start_col: 4usize,
            end_line: 583usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_borrow_mut()),
        ),
    };
    fn test_borrow_mut() {
        use std::borrow::BorrowMut;
        let mut a: SmallVec<[u32; 2]> = SmallVec::new();
        a.push(1);
        match (&a.borrow_mut(), &[1]) {
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
        a.push(2);
        match (&a.borrow_mut(), &[1, 2]) {
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
        a.push(3);
        match (&a.borrow_mut(), &[1, 2, 3]) {
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
        BorrowMut::<[u32]>::borrow_mut(&mut a)[1] = 4;
        match (&a.borrow_mut(), &[1, 4, 3]) {
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
    #[rustc_test_marker = "tests::test_from"]
    #[doc(hidden)]
    pub const test_from: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_from"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 598usize,
            start_col: 4usize,
            end_line: 598usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_from()),
        ),
    };
    fn test_from() {
        match (&&SmallVec::<[u32; 2]>::from(&[1][..])[..], &[1]) {
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
        match (&&SmallVec::<[u32; 2]>::from(&[1, 2, 3][..])[..], &[1, 2, 3]) {
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
        let vec = ::alloc::vec::Vec::new();
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from(vec);
        match (&&*small_vec, &&[]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3, 4, 5]));
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from(vec);
        match (&&*small_vec, &&[1, 2, 3, 4, 5]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3, 4, 5]));
        let small_vec: SmallVec<[u8; 1]> = SmallVec::from(vec);
        match (&&*small_vec, &&[1, 2, 3, 4, 5]) {
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
        drop(small_vec);
        let array = [1];
        let small_vec: SmallVec<[u8; 1]> = SmallVec::from(array);
        match (&&*small_vec, &&[1]) {
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
        drop(small_vec);
        let array = [99; 128];
        let small_vec: SmallVec<[u8; 128]> = SmallVec::from(array);
        match (&&*small_vec, &::alloc::vec::from_elem(99u8, 128).as_slice()) {
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
        drop(small_vec);
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_from_slice"]
    #[doc(hidden)]
    pub const test_from_slice: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_from_slice"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 629usize,
            start_col: 4usize,
            end_line: 629usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_from_slice()),
        ),
    };
    fn test_from_slice() {
        match (&&SmallVec::<[u32; 2]>::from_slice(&[1][..])[..], &[1]) {
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
        match (&&SmallVec::<[u32; 2]>::from_slice(&[1, 2, 3][..])[..], &[1, 2, 3]) {
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
    #[rustc_test_marker = "tests::test_exact_size_iterator"]
    #[doc(hidden)]
    pub const test_exact_size_iterator: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_exact_size_iterator"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 638usize,
            start_col: 4usize,
            end_line: 638usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_exact_size_iterator()),
        ),
    };
    fn test_exact_size_iterator() {
        let mut vec = SmallVec::<[u32; 2]>::from(&[1, 2, 3][..]);
        match (&vec.clone().into_iter().len(), &3) {
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
        match (&vec.drain(..2).len(), &2) {
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
        match (&vec.into_iter().len(), &1) {
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
    #[rustc_test_marker = "tests::test_into_iter_as_slice"]
    #[doc(hidden)]
    pub const test_into_iter_as_slice: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_into_iter_as_slice"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 646usize,
            start_col: 4usize,
            end_line: 646usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_into_iter_as_slice()),
        ),
    };
    fn test_into_iter_as_slice() {
        let vec = SmallVec::<[u32; 2]>::from(&[1, 2, 3][..]);
        let mut iter = vec.clone().into_iter();
        match (&iter.as_slice(), &&[1, 2, 3]) {
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
        match (&iter.as_mut_slice(), &&[1, 2, 3]) {
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
        match (&iter.as_slice(), &&[2, 3]) {
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
        match (&iter.as_mut_slice(), &&[2, 3]) {
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
        iter.next_back();
        match (&iter.as_slice(), &&[2]) {
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
        match (&iter.as_mut_slice(), &&[2]) {
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
    #[rustc_test_marker = "tests::test_into_iter_clone"]
    #[doc(hidden)]
    pub const test_into_iter_clone: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_into_iter_clone"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 660usize,
            start_col: 4usize,
            end_line: 660usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_into_iter_clone()),
        ),
    };
    fn test_into_iter_clone() {
        let mut iter = SmallVec::<[u8; 2]>::from_iter(0..3).into_iter();
        let mut clone_iter = iter.clone();
        while let Some(x) = iter.next() {
            match (&x, &clone_iter.next().unwrap()) {
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
        match (&clone_iter.next(), &None) {
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
    #[rustc_test_marker = "tests::test_into_iter_clone_partially_consumed_iterator"]
    #[doc(hidden)]
    pub const test_into_iter_clone_partially_consumed_iterator: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName(
                "tests::test_into_iter_clone_partially_consumed_iterator",
            ),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 672usize,
            start_col: 4usize,
            end_line: 672usize,
            end_col: 52usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(
                test_into_iter_clone_partially_consumed_iterator(),
            ),
        ),
    };
    fn test_into_iter_clone_partially_consumed_iterator() {
        let mut iter = SmallVec::<[u8; 2]>::from_iter(0..3).into_iter().skip(1);
        let mut clone_iter = iter.clone();
        while let Some(x) = iter.next() {
            match (&x, &clone_iter.next().unwrap()) {
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
        match (&clone_iter.next(), &None) {
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
    #[rustc_test_marker = "tests::test_into_iter_clone_empty_smallvec"]
    #[doc(hidden)]
    pub const test_into_iter_clone_empty_smallvec: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_into_iter_clone_empty_smallvec"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 683usize,
            start_col: 4usize,
            end_line: 683usize,
            end_col: 39usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_into_iter_clone_empty_smallvec()),
        ),
    };
    fn test_into_iter_clone_empty_smallvec() {
        let mut iter = SmallVec::<[u8; 2]>::new().into_iter();
        let mut clone_iter = iter.clone();
        match (&iter.next(), &None) {
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
        match (&clone_iter.next(), &None) {
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
    #[rustc_test_marker = "tests::shrink_to_fit_unspill"]
    #[doc(hidden)]
    pub const shrink_to_fit_unspill: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::shrink_to_fit_unspill"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 691usize,
            start_col: 4usize,
            end_line: 691usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(shrink_to_fit_unspill()),
        ),
    };
    fn shrink_to_fit_unspill() {
        let mut vec = SmallVec::<[u8; 2]>::from_iter(0..3);
        vec.pop();
        if !vec.spilled() {
            ::core::panicking::panic("assertion failed: vec.spilled()")
        }
        vec.shrink_to_fit();
        if !!vec.spilled() {
            ::core::panicking::panic("shrink_to_fit will un-spill if possible")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_into_vec"]
    #[doc(hidden)]
    pub const test_into_vec: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_into_vec"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 700usize,
            start_col: 4usize,
            end_line: 700usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_into_vec()),
        ),
    };
    fn test_into_vec() {
        let vec = SmallVec::<[u8; 2]>::from_iter(0..2);
        match (&vec.into_vec(), &<[_]>::into_vec(::alloc::boxed::box_new([0, 1]))) {
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
        let vec = SmallVec::<[u8; 2]>::from_iter(0..3);
        match (&vec.into_vec(), &<[_]>::into_vec(::alloc::boxed::box_new([0, 1, 2]))) {
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
    #[rustc_test_marker = "tests::test_into_inner"]
    #[doc(hidden)]
    pub const test_into_inner: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_into_inner"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 709usize,
            start_col: 4usize,
            end_line: 709usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_into_inner()),
        ),
    };
    fn test_into_inner() {
        let vec = SmallVec::<[u8; 2]>::from_iter(0..2);
        match (&vec.into_inner(), &Ok([0, 1])) {
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
        let vec = SmallVec::<[u8; 2]>::from_iter(0..1);
        match (&vec.clone().into_inner(), &Err(vec)) {
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
        let vec = SmallVec::<[u8; 2]>::from_iter(0..3);
        match (&vec.clone().into_inner(), &Err(vec)) {
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
    #[rustc_test_marker = "tests::test_from_vec"]
    #[doc(hidden)]
    pub const test_from_vec: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_from_vec"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 721usize,
            start_col: 4usize,
            end_line: 721usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_from_vec()),
        ),
    };
    fn test_from_vec() {
        let vec = ::alloc::vec::Vec::new();
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[]) {
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
        drop(small_vec);
        let vec = ::alloc::vec::Vec::new();
        let small_vec: SmallVec<[u8; 1]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1]));
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[1]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3]));
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[1, 2, 3]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3, 4, 5]));
        let small_vec: SmallVec<[u8; 3]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[1, 2, 3, 4, 5]) {
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
        drop(small_vec);
        let vec = <[_]>::into_vec(::alloc::boxed::box_new([1, 2, 3, 4, 5]));
        let small_vec: SmallVec<[u8; 1]> = SmallVec::from_vec(vec);
        match (&&*small_vec, &&[1, 2, 3, 4, 5]) {
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
        drop(small_vec);
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_retain"]
    #[doc(hidden)]
    pub const test_retain: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_retain"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 754usize,
            start_col: 4usize,
            end_line: 754usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_retain()),
        ),
    };
    fn test_retain() {
        let mut sv: SmallVec<[i32; 5]> = SmallVec::from_slice(&[1, 2, 3, 3, 4]);
        sv.retain(|&mut i| i != 3);
        match (&sv.pop(), &Some(4)) {
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
        match (&sv.pop(), &Some(2)) {
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
        match (&sv.pop(), &Some(1)) {
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
        match (&sv.pop(), &None) {
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
        let mut sv: SmallVec<[i32; 3]> = SmallVec::from_slice(&[1, 2, 3, 3, 4]);
        sv.retain(|&mut i| i != 3);
        match (&sv.pop(), &Some(4)) {
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
        match (&sv.pop(), &Some(2)) {
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
        match (&sv.pop(), &Some(1)) {
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
        match (&sv.pop(), &None) {
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
        let one = Rc::new(1);
        let mut sv: SmallVec<[Rc<i32>; 3]> = SmallVec::new();
        sv.push(Rc::clone(&one));
        match (&Rc::strong_count(&one), &2) {
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
        sv.retain(|_| false);
        match (&Rc::strong_count(&one), &1) {
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
        let mut sv: SmallVec<[Rc<i32>; 1]> = SmallVec::new();
        sv.push(Rc::clone(&one));
        sv.push(Rc::new(2));
        match (&Rc::strong_count(&one), &2) {
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
        sv.retain(|_| false);
        match (&Rc::strong_count(&one), &1) {
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
    #[rustc_test_marker = "tests::test_dedup"]
    #[doc(hidden)]
    pub const test_dedup: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_dedup"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 789usize,
            start_col: 4usize,
            end_line: 789usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_dedup()),
        ),
    };
    fn test_dedup() {
        let mut dupes: SmallVec<[i32; 5]> = SmallVec::from_slice(&[1, 1, 2, 3, 3]);
        dupes.dedup();
        match (&&*dupes, &&[1, 2, 3]) {
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
        let mut empty: SmallVec<[i32; 5]> = SmallVec::new();
        empty.dedup();
        if !empty.is_empty() {
            ::core::panicking::panic("assertion failed: empty.is_empty()")
        }
        let mut all_ones: SmallVec<[i32; 5]> = SmallVec::from_slice(&[1, 1, 1, 1, 1]);
        all_ones.dedup();
        match (&all_ones.len(), &1) {
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
        let mut no_dupes: SmallVec<[i32; 5]> = SmallVec::from_slice(&[1, 2, 3, 4, 5]);
        no_dupes.dedup();
        match (&no_dupes.len(), &5) {
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
    #[rustc_test_marker = "tests::test_resize"]
    #[doc(hidden)]
    pub const test_resize: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_resize"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 808usize,
            start_col: 4usize,
            end_line: 808usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_resize()),
        ),
    };
    fn test_resize() {
        let mut v: SmallVec<[i32; 8]> = SmallVec::new();
        v.push(1);
        v.resize(5, 0);
        match (&v[..], &[1, 0, 0, 0, 0][..]) {
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
        v.resize(2, -1);
        match (&v[..], &[1, 0][..]) {
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
    #[rustc_test_marker = "tests::grow_to_shrink"]
    #[doc(hidden)]
    pub const grow_to_shrink: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::grow_to_shrink"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 855usize,
            start_col: 4usize,
            end_line: 855usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(grow_to_shrink()),
        ),
    };
    fn grow_to_shrink() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        if !v.spilled() {
            ::core::panicking::panic("assertion failed: v.spilled()")
        }
        v.clear();
        v.grow(2);
        if !!v.spilled() {
            ::core::panicking::panic("assertion failed: !v.spilled()")
        }
        match (&v.capacity(), &2) {
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
        v.push(4);
        match (&v[..], &[4]) {
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
    #[rustc_test_marker = "tests::resumable_extend"]
    #[doc(hidden)]
    pub const resumable_extend: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::resumable_extend"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 872usize,
            start_col: 4usize,
            end_line: 872usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(resumable_extend()),
        ),
    };
    fn resumable_extend() {
        let s = "a b c";
        let it = s
            .chars()
            .scan(0, |_, ch| if ch.is_whitespace() { None } else { Some(ch) });
        let mut v: SmallVec<[char; 4]> = SmallVec::new();
        v.extend(it);
        match (&v[..], &['a']) {
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
    #[rustc_test_marker = "tests::uninhabited"]
    #[doc(hidden)]
    pub const uninhabited: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::uninhabited"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 885usize,
            start_col: 4usize,
            end_line: 885usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(uninhabited()),
        ),
    };
    fn uninhabited() {
        enum Void {}
        let _sv = SmallVec::<[Void; 8]>::new();
    }
    extern crate test;
    #[rustc_test_marker = "tests::grow_spilled_same_size"]
    #[doc(hidden)]
    pub const grow_spilled_same_size: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::grow_spilled_same_size"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 891usize,
            start_col: 4usize,
            end_line: 891usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(grow_spilled_same_size()),
        ),
    };
    fn grow_spilled_same_size() {
        let mut v: SmallVec<[u8; 2]> = SmallVec::new();
        v.push(0);
        v.push(1);
        v.push(2);
        if !v.spilled() {
            ::core::panicking::panic("assertion failed: v.spilled()")
        }
        match (&v.capacity(), &4) {
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
        v.grow(4);
        match (&v.capacity(), &4) {
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
        match (&v[..], &[0, 1, 2]) {
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
    #[rustc_test_marker = "tests::empty_macro"]
    #[doc(hidden)]
    pub const empty_macro: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::empty_macro"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 952usize,
            start_col: 4usize,
            end_line: 952usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(empty_macro()),
        ),
    };
    fn empty_macro() {
        let _v: SmallVec<[u8; 1]> = crate::SmallVec::new();
    }
    extern crate test;
    #[rustc_test_marker = "tests::zero_size_items"]
    #[doc(hidden)]
    pub const zero_size_items: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::zero_size_items"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 957usize,
            start_col: 4usize,
            end_line: 957usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(zero_size_items()),
        ),
    };
    fn zero_size_items() {
        SmallVec::<[(); 0]>::new().push(());
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_insert_many_overflow"]
    #[doc(hidden)]
    pub const test_insert_many_overflow: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_many_overflow"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 962usize,
            start_col: 4usize,
            end_line: 962usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_many_overflow()),
        ),
    };
    fn test_insert_many_overflow() {
        let mut v: SmallVec<[u8; 1]> = SmallVec::new();
        v.push(123);
        let iter = (0u8..5).filter(|n| n % 2 == 0);
        match (&iter.size_hint().0, &0) {
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
        v.insert_many(0, iter);
        match (&&*v, &&[0, 2, 4, 123]) {
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
    #[rustc_test_marker = "tests::test_clone_from"]
    #[doc(hidden)]
    pub const test_clone_from: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_clone_from"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 975usize,
            start_col: 4usize,
            end_line: 975usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_clone_from()),
        ),
    };
    fn test_clone_from() {
        let mut a: SmallVec<[u8; 2]> = SmallVec::new();
        a.push(1);
        a.push(2);
        a.push(3);
        let mut b: SmallVec<[u8; 2]> = SmallVec::new();
        b.push(10);
        let mut c: SmallVec<[u8; 2]> = SmallVec::new();
        c.push(20);
        c.push(21);
        c.push(22);
        a.clone_from(&b);
        match (&&*a, &&[10]) {
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
        b.clone_from(&c);
        match (&&*b, &&[20, 21, 22]) {
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
    #[rustc_test_marker = "tests::test_size"]
    #[doc(hidden)]
    pub const test_size: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_size"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 997usize,
            start_col: 4usize,
            end_line: 997usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_size()),
        ),
    };
    fn test_size() {
        use core::mem::size_of;
        const PTR_SIZE: usize = size_of::<usize>();
        {
            match (&(3 * PTR_SIZE), &size_of::<SmallVec<[u8; 0]>>()) {
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
            match (&(3 * PTR_SIZE), &size_of::<SmallVec<[u8; 1]>>()) {
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
            match (&(3 * PTR_SIZE), &size_of::<SmallVec<[u8; PTR_SIZE]>>()) {
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
            match (&(4 * PTR_SIZE), &size_of::<SmallVec<[u8; PTR_SIZE + 1]>>()) {
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
    #[rustc_test_marker = "tests::max_dont_panic"]
    #[doc(hidden)]
    pub const max_dont_panic: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::max_dont_panic"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 1049usize,
            start_col: 4usize,
            end_line: 1049usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(max_dont_panic()),
        ),
    };
    /// This assortment of tests, in combination with miri, verifies we handle UB on fishy arguments
    /// given to SmallVec. Draining and extending the allocation are fairly well-tested earlier, but
    /// `smallvec.insert(usize::MAX, val)` once slipped by!
    ///
    /// All code that indexes into SmallVecs should be tested with such "trivially wrong" args.
    fn max_dont_panic() {
        let mut sv: SmallVec<[i32; 2]> = {
            let count = 0usize + 1usize;
            let mut vec = crate::SmallVec::new();
            if count <= vec.inline_size() {
                vec.push(0);
                vec
            } else {
                crate::SmallVec::from_vec(<[_]>::into_vec(::alloc::boxed::box_new([0])))
            }
        };
        let _ = sv.get(usize::MAX);
        sv.truncate(usize::MAX);
    }
    extern crate test;
    #[rustc_test_marker = "tests::max_remove"]
    #[doc(hidden)]
    pub const max_remove: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::max_remove"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 1057usize,
            start_col: 4usize,
            end_line: 1057usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(max_remove()),
        ),
    };
    #[should_panic]
    fn max_remove() {
        let mut sv: SmallVec<[i32; 2]> = {
            let count = 0usize + 1usize;
            let mut vec = crate::SmallVec::new();
            if count <= vec.inline_size() {
                vec.push(0);
                vec
            } else {
                crate::SmallVec::from_vec(<[_]>::into_vec(::alloc::boxed::box_new([0])))
            }
        };
        sv.remove(usize::MAX);
    }
    extern crate test;
    #[rustc_test_marker = "tests::max_swap_remove"]
    #[doc(hidden)]
    pub const max_swap_remove: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::max_swap_remove"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 1064usize,
            start_col: 4usize,
            end_line: 1064usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(max_swap_remove()),
        ),
    };
    #[should_panic]
    fn max_swap_remove() {
        let mut sv: SmallVec<[i32; 2]> = {
            let count = 0usize + 1usize;
            let mut vec = crate::SmallVec::new();
            if count <= vec.inline_size() {
                vec.push(0);
                vec
            } else {
                crate::SmallVec::from_vec(<[_]>::into_vec(::alloc::boxed::box_new([0])))
            }
        };
        sv.swap_remove(usize::MAX);
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_insert_out_of_bounds"]
    #[doc(hidden)]
    pub const test_insert_out_of_bounds: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_insert_out_of_bounds"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/tests.rs",
            start_line: 1071usize,
            start_col: 4usize,
            end_line: 1071usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::Yes,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_insert_out_of_bounds()),
        ),
    };
    #[should_panic]
    fn test_insert_out_of_bounds() {
        let mut v: SmallVec<[i32; 4]> = SmallVec::new();
        v.insert(10, 6);
    }
}
#[allow(deprecated)]
use alloc::alloc::{Layout, LayoutErr};
use alloc::boxed::Box;
use alloc::{vec, vec::Vec};
use core::borrow::{Borrow, BorrowMut};
use core::cmp;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::hint::unreachable_unchecked;
use core::iter::{repeat, FromIterator, FusedIterator, IntoIterator};
use core::mem;
use core::mem::MaybeUninit;
use core::ops::{self, Range, RangeBounds};
use core::ptr::{self, NonNull};
use core::slice::{self, SliceIndex};
/// Trait to be implemented by a collection that can be extended from a slice
///
/// ## Example
///
/// ```rust
/// use smallvec::{ExtendFromSlice, SmallVec};
///
/// fn initialize<V: ExtendFromSlice<u8>>(v: &mut V) {
///     v.extend_from_slice(b"Test!");
/// }
///
/// let mut vec = Vec::new();
/// initialize(&mut vec);
/// assert_eq!(&vec, b"Test!");
///
/// let mut small_vec = SmallVec::<[u8; 8]>::new();
/// initialize(&mut small_vec);
/// assert_eq!(&small_vec as &[_], b"Test!");
/// ```
#[doc(hidden)]
#[deprecated]
pub trait ExtendFromSlice<T> {
    /// Extends a collection from a slice of its element type
    fn extend_from_slice(&mut self, other: &[T]);
}
#[allow(deprecated)]
impl<T: Clone> ExtendFromSlice<T> for Vec<T> {
    fn extend_from_slice(&mut self, other: &[T]) {
        Vec::extend_from_slice(self, other)
    }
}
/// Error type for APIs with fallible heap allocation
pub enum CollectionAllocErr {
    /// Overflow `usize::MAX` or other error during size computation
    CapacityOverflow,
    /// The allocator return an error
    AllocErr {
        /// The layout that was passed to the allocator
        layout: Layout,
    },
}
#[automatically_derived]
impl ::core::fmt::Debug for CollectionAllocErr {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            CollectionAllocErr::CapacityOverflow => {
                ::core::fmt::Formatter::write_str(f, "CapacityOverflow")
            }
            CollectionAllocErr::AllocErr { layout: __self_0 } => {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "AllocErr",
                    "layout",
                    &__self_0,
                )
            }
        }
    }
}
impl fmt::Display for CollectionAllocErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Allocation error: {0:?}", self))
    }
}
#[allow(deprecated)]
impl From<LayoutErr> for CollectionAllocErr {
    fn from(_: LayoutErr) -> Self {
        CollectionAllocErr::CapacityOverflow
    }
}
fn infallible<T>(result: Result<T, CollectionAllocErr>) -> T {
    match result {
        Ok(x) => x,
        Err(CollectionAllocErr::CapacityOverflow) => {
            ::core::panicking::panic("capacity overflow")
        }
        Err(CollectionAllocErr::AllocErr { layout }) => {
            alloc::alloc::handle_alloc_error(layout)
        }
    }
}
/// FIXME: use `Layout::array` when we require a Rust version where it’s stable
/// <https://github.com/rust-lang/rust/issues/55724>
fn layout_array<T>(n: usize) -> Result<Layout, CollectionAllocErr> {
    let size = mem::size_of::<T>()
        .checked_mul(n)
        .ok_or(CollectionAllocErr::CapacityOverflow)?;
    let align = mem::align_of::<T>();
    Layout::from_size_align(size, align)
        .map_err(|_| CollectionAllocErr::CapacityOverflow)
}
unsafe fn deallocate<T>(ptr: NonNull<T>, capacity: usize) {
    let layout = layout_array::<T>(capacity).unwrap();
    alloc::alloc::dealloc(ptr.as_ptr() as *mut u8, layout)
}
/// An iterator that removes the items from a `SmallVec` and yields them by value.
///
/// Returned from [`SmallVec::drain`][1].
///
/// [1]: struct.SmallVec.html#method.drain
pub struct Drain<'a, T: 'a + Array> {
    tail_start: usize,
    tail_len: usize,
    iter: slice::Iter<'a, T::Item>,
    vec: NonNull<SmallVec<T>>,
}
impl<'a, T: 'a + Array> fmt::Debug for Drain<'a, T>
where
    T::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.iter.as_slice()).finish()
    }
}
unsafe impl<'a, T: Sync + Array> Sync for Drain<'a, T> {}
unsafe impl<'a, T: Send + Array> Send for Drain<'a, T> {}
impl<'a, T: 'a + Array> Iterator for Drain<'a, T> {
    type Item = T::Item;
    #[inline]
    fn next(&mut self) -> Option<T::Item> {
        self.iter.next().map(|reference| unsafe { ptr::read(reference) })
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'a, T: 'a + Array> DoubleEndedIterator for Drain<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T::Item> {
        self.iter.next_back().map(|reference| unsafe { ptr::read(reference) })
    }
}
impl<'a, T: Array> ExactSizeIterator for Drain<'a, T> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}
impl<'a, T: Array> FusedIterator for Drain<'a, T> {}
impl<'a, T: 'a + Array> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        self.for_each(drop);
        if self.tail_len > 0 {
            unsafe {
                let source_vec = self.vec.as_mut();
                let start = source_vec.len();
                let tail = self.tail_start;
                if tail != start {
                    let ptr = source_vec.as_mut_ptr();
                    let src = ptr.add(tail);
                    let dst = ptr.add(start);
                    ptr::copy(src, dst, self.tail_len);
                }
                source_vec.set_len(start + self.tail_len);
            }
        }
    }
}
enum SmallVecData<A: Array> {
    Inline(MaybeUninit<A>),
    Heap { ptr: NonNull<A::Item>, len: usize },
}
impl<A: Array> SmallVecData<A> {
    #[inline]
    unsafe fn inline(&self) -> ConstNonNull<A::Item> {
        match self {
            SmallVecData::Inline(a) => {
                ConstNonNull::new(a.as_ptr() as *const A::Item).unwrap()
            }
            _ => {
                if true {
                    ::core::panicking::panic("entered unreachable code");
                } else {
                    unreachable_unchecked();
                }
            }
        }
    }
    #[inline]
    unsafe fn inline_mut(&mut self) -> NonNull<A::Item> {
        match self {
            SmallVecData::Inline(a) => {
                NonNull::new(a.as_mut_ptr() as *mut A::Item).unwrap()
            }
            _ => {
                if true {
                    ::core::panicking::panic("entered unreachable code");
                } else {
                    unreachable_unchecked();
                }
            }
        }
    }
    #[inline]
    fn from_inline(inline: MaybeUninit<A>) -> SmallVecData<A> {
        SmallVecData::Inline(inline)
    }
    #[inline]
    unsafe fn into_inline(self) -> MaybeUninit<A> {
        match self {
            SmallVecData::Inline(a) => a,
            _ => {
                if true {
                    ::core::panicking::panic("entered unreachable code");
                } else {
                    unreachable_unchecked();
                }
            }
        }
    }
    #[inline]
    unsafe fn heap(&self) -> (ConstNonNull<A::Item>, usize) {
        match self {
            SmallVecData::Heap { ptr, len } => (ConstNonNull(*ptr), *len),
            _ => {
                if true {
                    ::core::panicking::panic("entered unreachable code");
                } else {
                    unreachable_unchecked();
                }
            }
        }
    }
    #[inline]
    unsafe fn heap_mut(&mut self) -> (NonNull<A::Item>, &mut usize) {
        match self {
            SmallVecData::Heap { ptr, len } => (*ptr, len),
            _ => {
                if true {
                    ::core::panicking::panic("entered unreachable code");
                } else {
                    unreachable_unchecked();
                }
            }
        }
    }
    #[inline]
    fn from_heap(ptr: NonNull<A::Item>, len: usize) -> SmallVecData<A> {
        SmallVecData::Heap { ptr, len }
    }
}
unsafe impl<A: Array + Send> Send for SmallVecData<A> {}
unsafe impl<A: Array + Sync> Sync for SmallVecData<A> {}
/// A `Vec`-like container that can store a small number of elements inline.
///
/// `SmallVec` acts like a vector, but can store a limited amount of data inline within the
/// `SmallVec` struct rather than in a separate allocation.  If the data exceeds this limit, the
/// `SmallVec` will "spill" its data onto the heap, allocating a new buffer to hold it.
///
/// The amount of data that a `SmallVec` can store inline depends on its backing store. The backing
/// store can be any type that implements the `Array` trait; usually it is a small fixed-sized
/// array.  For example a `SmallVec<[u64; 8]>` can hold up to eight 64-bit integers inline.
///
/// ## Example
///
/// ```rust
/// use smallvec::SmallVec;
/// let mut v = SmallVec::<[u8; 4]>::new(); // initialize an empty vector
///
/// // The vector can hold up to 4 items without spilling onto the heap.
/// v.extend(0..4);
/// assert_eq!(v.len(), 4);
/// assert!(!v.spilled());
///
/// // Pushing another element will force the buffer to spill:
/// v.push(4);
/// assert_eq!(v.len(), 5);
/// assert!(v.spilled());
/// ```
pub struct SmallVec<A: Array> {
    capacity: usize,
    data: SmallVecData<A>,
}
impl<A: Array> SmallVec<A> {
    /// Construct an empty vector
    #[inline]
    pub fn new() -> SmallVec<A> {
        if !(mem::size_of::<A>() == A::size() * mem::size_of::<A::Item>()
            && mem::align_of::<A>() >= mem::align_of::<A::Item>())
        {
            ::core::panicking::panic(
                "assertion failed: mem::size_of::<A>() == A::size() * mem::size_of::<A::Item>() &&\n    mem::align_of::<A>() >= mem::align_of::<A::Item>()",
            )
        }
        SmallVec {
            capacity: 0,
            data: SmallVecData::from_inline(MaybeUninit::uninit()),
        }
    }
    /// Construct an empty vector with enough capacity pre-allocated to store at least `n`
    /// elements.
    ///
    /// Will create a heap allocation only if `n` is larger than the inline capacity.
    ///
    /// ```
    /// # use smallvec::SmallVec;
    ///
    /// let v: SmallVec<[u8; 3]> = SmallVec::with_capacity(100);
    ///
    /// assert!(v.is_empty());
    /// assert!(v.capacity() >= 100);
    /// ```
    #[inline]
    pub fn with_capacity(n: usize) -> Self {
        let mut v = SmallVec::new();
        v.reserve_exact(n);
        v
    }
    /// Construct a new `SmallVec` from a `Vec<A::Item>`.
    ///
    /// Elements will be copied to the inline buffer if `vec.capacity() <= Self::inline_capacity()`.
    ///
    /// ```rust
    /// use smallvec::SmallVec;
    ///
    /// let vec = vec![1, 2, 3, 4, 5];
    /// let small_vec: SmallVec<[_; 3]> = SmallVec::from_vec(vec);
    ///
    /// assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    pub fn from_vec(mut vec: Vec<A::Item>) -> SmallVec<A> {
        if vec.capacity() <= Self::inline_capacity() {
            unsafe {
                let mut data = SmallVecData::<A>::from_inline(MaybeUninit::uninit());
                let len = vec.len();
                vec.set_len(0);
                ptr::copy_nonoverlapping(vec.as_ptr(), data.inline_mut().as_ptr(), len);
                SmallVec { capacity: len, data }
            }
        } else {
            let (ptr, cap, len) = (vec.as_mut_ptr(), vec.capacity(), vec.len());
            mem::forget(vec);
            let ptr = NonNull::new(ptr).expect("Cannot be null by `Vec` invariant");
            SmallVec {
                capacity: cap,
                data: SmallVecData::from_heap(ptr, len),
            }
        }
    }
    /// Constructs a new `SmallVec` on the stack from an `A` without
    /// copying elements.
    ///
    /// ```rust
    /// use smallvec::SmallVec;
    ///
    /// let buf = [1, 2, 3, 4, 5];
    /// let small_vec: SmallVec<_> = SmallVec::from_buf(buf);
    ///
    /// assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    pub fn from_buf(buf: A) -> SmallVec<A> {
        SmallVec {
            capacity: A::size(),
            data: SmallVecData::from_inline(MaybeUninit::new(buf)),
        }
    }
    /// Constructs a new `SmallVec` on the stack from an `A` without
    /// copying elements. Also sets the length, which must be less or
    /// equal to the size of `buf`.
    ///
    /// ```rust
    /// use smallvec::SmallVec;
    ///
    /// let buf = [1, 2, 3, 4, 5, 0, 0, 0];
    /// let small_vec: SmallVec<_> = SmallVec::from_buf_and_len(buf, 5);
    ///
    /// assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    pub fn from_buf_and_len(buf: A, len: usize) -> SmallVec<A> {
        if !(len <= A::size()) {
            ::core::panicking::panic("assertion failed: len <= A::size()")
        }
        unsafe { SmallVec::from_buf_and_len_unchecked(MaybeUninit::new(buf), len) }
    }
    /// Constructs a new `SmallVec` on the stack from an `A` without
    /// copying elements. Also sets the length. The user is responsible
    /// for ensuring that `len <= A::size()`.
    ///
    /// ```rust
    /// use smallvec::SmallVec;
    /// use std::mem::MaybeUninit;
    ///
    /// let buf = [1, 2, 3, 4, 5, 0, 0, 0];
    /// let small_vec: SmallVec<_> = unsafe {
    ///     SmallVec::from_buf_and_len_unchecked(MaybeUninit::new(buf), 5)
    /// };
    ///
    /// assert_eq!(&*small_vec, &[1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    pub unsafe fn from_buf_and_len_unchecked(
        buf: MaybeUninit<A>,
        len: usize,
    ) -> SmallVec<A> {
        SmallVec {
            capacity: len,
            data: SmallVecData::from_inline(buf),
        }
    }
    /// Sets the length of a vector.
    ///
    /// This will explicitly set the size of the vector, without actually
    /// modifying its buffers, so it is up to the caller to ensure that the
    /// vector is actually the specified size.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        let (_, len_ptr, _) = self.triple_mut();
        *len_ptr = new_len;
    }
    /// The maximum number of elements this vector can hold inline
    #[inline]
    fn inline_capacity() -> usize {
        if mem::size_of::<A::Item>() > 0 { A::size() } else { core::usize::MAX }
    }
    /// The maximum number of elements this vector can hold inline
    #[inline]
    pub fn inline_size(&self) -> usize {
        Self::inline_capacity()
    }
    /// The number of elements stored in the vector
    #[inline]
    pub fn len(&self) -> usize {
        self.triple().1
    }
    /// Returns `true` if the vector is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The number of items the vector can hold without reallocating
    #[inline]
    pub fn capacity(&self) -> usize {
        self.triple().2
    }
    /// Returns a tuple with (data ptr, len, capacity)
    /// Useful to get all `SmallVec` properties with a single check of the current storage variant.
    #[inline]
    fn triple(&self) -> (ConstNonNull<A::Item>, usize, usize) {
        unsafe {
            if self.spilled() {
                let (ptr, len) = self.data.heap();
                (ptr, len, self.capacity)
            } else {
                (self.data.inline(), self.capacity, Self::inline_capacity())
            }
        }
    }
    /// Returns a tuple with (data ptr, len ptr, capacity)
    #[inline]
    fn triple_mut(&mut self) -> (NonNull<A::Item>, &mut usize, usize) {
        unsafe {
            if self.spilled() {
                let (ptr, len_ptr) = self.data.heap_mut();
                (ptr, len_ptr, self.capacity)
            } else {
                (self.data.inline_mut(), &mut self.capacity, Self::inline_capacity())
            }
        }
    }
    /// Returns `true` if the data has spilled into a separate heap-allocated buffer.
    #[inline]
    pub fn spilled(&self) -> bool {
        self.capacity > Self::inline_capacity()
    }
    /// Creates a draining iterator that removes the specified range in the vector
    /// and yields the removed items.
    ///
    /// Note 1: The element range is removed even if the iterator is only
    /// partially consumed or not consumed at all.
    ///
    /// Note 2: It is unspecified how many elements are removed from the vector
    /// if the `Drain` value is leaked.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, A>
    where
        R: RangeBounds<usize>,
    {
        use core::ops::Bound::*;
        let len = self.len();
        let start = match range.start_bound() {
            Included(&n) => n,
            Excluded(&n) => n.checked_add(1).expect("Range start out of bounds"),
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(&n) => n.checked_add(1).expect("Range end out of bounds"),
            Excluded(&n) => n,
            Unbounded => len,
        };
        if !(start <= end) {
            ::core::panicking::panic("assertion failed: start <= end")
        }
        if !(end <= len) {
            ::core::panicking::panic("assertion failed: end <= len")
        }
        unsafe {
            self.set_len(start);
            let range_slice = slice::from_raw_parts(
                self.as_ptr().add(start),
                end - start,
            );
            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                vec: NonNull::new_unchecked(self as *mut _),
            }
        }
    }
    /// Append an item to the vector.
    #[inline]
    pub fn push(&mut self, value: A::Item) {
        unsafe {
            let (mut ptr, mut len, cap) = self.triple_mut();
            if *len == cap {
                self.reserve_one_unchecked();
                let (heap_ptr, heap_len) = self.data.heap_mut();
                ptr = heap_ptr;
                len = heap_len;
            }
            ptr::write(ptr.as_ptr().add(*len), value);
            *len += 1;
        }
    }
    /// Remove an item from the end of the vector and return it, or None if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<A::Item> {
        unsafe {
            let (ptr, len_ptr, _) = self.triple_mut();
            let ptr: *const _ = ptr.as_ptr();
            if *len_ptr == 0 {
                return None;
            }
            let last_index = *len_ptr - 1;
            *len_ptr = last_index;
            Some(ptr::read(ptr.add(last_index)))
        }
    }
    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Example
    ///
    /// ```
    /// # use smallvec::{SmallVec, smallvec};
    /// let mut v0: SmallVec<[u8; 16]> = smallvec![1, 2, 3];
    /// let mut v1: SmallVec<[u8; 32]> = smallvec![4, 5, 6];
    /// v0.append(&mut v1);
    /// assert_eq!(*v0, [1, 2, 3, 4, 5, 6]);
    /// assert_eq!(*v1, []);
    /// ```
    pub fn append<B>(&mut self, other: &mut SmallVec<B>)
    where
        B: Array<Item = A::Item>,
    {
        self.extend(other.drain(..))
    }
    /// Re-allocate to set the capacity to `max(new_cap, inline_size())`.
    ///
    /// Panics if `new_cap` is less than the vector's length
    /// or if the capacity computation overflows `usize`.
    pub fn grow(&mut self, new_cap: usize) {
        infallible(self.try_grow(new_cap))
    }
    /// Re-allocate to set the capacity to `max(new_cap, inline_size())`.
    ///
    /// Panics if `new_cap` is less than the vector's length
    pub fn try_grow(&mut self, new_cap: usize) -> Result<(), CollectionAllocErr> {
        unsafe {
            let unspilled = !self.spilled();
            let (ptr, &mut len, cap) = self.triple_mut();
            if !(new_cap >= len) {
                ::core::panicking::panic("assertion failed: new_cap >= len")
            }
            if new_cap <= Self::inline_capacity() {
                if unspilled {
                    return Ok(());
                }
                self.data = SmallVecData::from_inline(MaybeUninit::uninit());
                ptr::copy_nonoverlapping(
                    ptr.as_ptr(),
                    self.data.inline_mut().as_ptr(),
                    len,
                );
                self.capacity = len;
                deallocate(ptr, cap);
            } else if new_cap != cap {
                let layout = layout_array::<A::Item>(new_cap)?;
                if true {
                    if !(layout.size() > 0) {
                        ::core::panicking::panic("assertion failed: layout.size() > 0")
                    }
                }
                let new_alloc;
                if unspilled {
                    new_alloc = NonNull::new(alloc::alloc::alloc(layout))
                        .ok_or(CollectionAllocErr::AllocErr {
                            layout,
                        })?
                        .cast();
                    ptr::copy_nonoverlapping(ptr.as_ptr(), new_alloc.as_ptr(), len);
                } else {
                    let old_layout = layout_array::<A::Item>(cap)?;
                    let new_ptr = alloc::alloc::realloc(
                        ptr.as_ptr() as *mut u8,
                        old_layout,
                        layout.size(),
                    );
                    new_alloc = NonNull::new(new_ptr)
                        .ok_or(CollectionAllocErr::AllocErr {
                            layout,
                        })?
                        .cast();
                }
                self.data = SmallVecData::from_heap(new_alloc, len);
                self.capacity = new_cap;
            }
            Ok(())
        }
    }
    /// Reserve capacity for `additional` more elements to be inserted.
    ///
    /// May reserve more space to avoid frequent reallocations.
    ///
    /// Panics if the capacity computation overflows `usize`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        infallible(self.try_reserve(additional))
    }
    /// Internal method used to grow in push() and insert(), where we know already we have to grow.
    #[cold]
    fn reserve_one_unchecked(&mut self) {
        if true {
            match (&self.len(), &self.capacity()) {
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
        let new_cap = self
            .len()
            .checked_add(1)
            .and_then(usize::checked_next_power_of_two)
            .expect("capacity overflow");
        infallible(self.try_grow(new_cap))
    }
    /// Reserve capacity for `additional` more elements to be inserted.
    ///
    /// May reserve more space to avoid frequent reallocations.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), CollectionAllocErr> {
        let (_, &mut len, cap) = self.triple_mut();
        if cap - len >= additional {
            return Ok(());
        }
        let new_cap = len
            .checked_add(additional)
            .and_then(usize::checked_next_power_of_two)
            .ok_or(CollectionAllocErr::CapacityOverflow)?;
        self.try_grow(new_cap)
    }
    /// Reserve the minimum capacity for `additional` more elements to be inserted.
    ///
    /// Panics if the new capacity overflows `usize`.
    pub fn reserve_exact(&mut self, additional: usize) {
        infallible(self.try_reserve_exact(additional))
    }
    /// Reserve the minimum capacity for `additional` more elements to be inserted.
    pub fn try_reserve_exact(
        &mut self,
        additional: usize,
    ) -> Result<(), CollectionAllocErr> {
        let (_, &mut len, cap) = self.triple_mut();
        if cap - len >= additional {
            return Ok(());
        }
        let new_cap = len
            .checked_add(additional)
            .ok_or(CollectionAllocErr::CapacityOverflow)?;
        self.try_grow(new_cap)
    }
    /// Shrink the capacity of the vector as much as possible.
    ///
    /// When possible, this will move data from an external heap buffer to the vector's inline
    /// storage.
    pub fn shrink_to_fit(&mut self) {
        if !self.spilled() {
            return;
        }
        let len = self.len();
        if self.inline_size() >= len {
            unsafe {
                let (ptr, len) = self.data.heap();
                self.data = SmallVecData::from_inline(MaybeUninit::uninit());
                ptr::copy_nonoverlapping(
                    ptr.as_ptr(),
                    self.data.inline_mut().as_ptr(),
                    len,
                );
                deallocate(ptr.0, self.capacity);
                self.capacity = len;
            }
        } else if self.capacity() > len {
            self.grow(len);
        }
    }
    /// Shorten the vector, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than or equal to the vector's current length, this has no
    /// effect.
    ///
    /// This does not re-allocate.  If you want the vector's capacity to shrink, call
    /// `shrink_to_fit` after truncating.
    pub fn truncate(&mut self, len: usize) {
        unsafe {
            let (ptr, len_ptr, _) = self.triple_mut();
            let ptr = ptr.as_ptr();
            while len < *len_ptr {
                let last_index = *len_ptr - 1;
                *len_ptr = last_index;
                ptr::drop_in_place(ptr.add(last_index));
            }
        }
    }
    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    pub fn as_slice(&self) -> &[A::Item] {
        self
    }
    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    pub fn as_mut_slice(&mut self) -> &mut [A::Item] {
        self
    }
    /// Remove the element at position `index`, replacing it with the last element.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// Panics if `index` is out of bounds.
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> A::Item {
        let len = self.len();
        self.swap(len - 1, index);
        self.pop().unwrap_or_else(|| unsafe { unreachable_unchecked() })
    }
    /// Remove all elements from the vector.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
    }
    /// Remove and return the element at position `index`, shifting all elements after it to the
    /// left.
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> A::Item {
        unsafe {
            let (ptr, len_ptr, _) = self.triple_mut();
            let len = *len_ptr;
            if !(index < len) {
                ::core::panicking::panic("assertion failed: index < len")
            }
            *len_ptr = len - 1;
            let ptr = ptr.as_ptr().add(index);
            let item = ptr::read(ptr);
            ptr::copy(ptr.add(1), ptr, len - index - 1);
            item
        }
    }
    /// Insert an element at position `index`, shifting all elements after it to the right.
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, element: A::Item) {
        unsafe {
            let (mut ptr, mut len_ptr, cap) = self.triple_mut();
            if *len_ptr == cap {
                self.reserve_one_unchecked();
                let (heap_ptr, heap_len_ptr) = self.data.heap_mut();
                ptr = heap_ptr;
                len_ptr = heap_len_ptr;
            }
            let mut ptr = ptr.as_ptr();
            let len = *len_ptr;
            if index > len {
                ::core::panicking::panic("index exceeds length");
            }
            ptr = ptr.add(index);
            if index < len {
                ptr::copy(ptr, ptr.add(1), len - index);
            }
            *len_ptr = len + 1;
            ptr::write(ptr, element);
        }
    }
    /// Insert multiple elements at position `index`, shifting all following elements toward the
    /// back.
    pub fn insert_many<I: IntoIterator<Item = A::Item>>(
        &mut self,
        index: usize,
        iterable: I,
    ) {
        let mut iter = iterable.into_iter();
        if index == self.len() {
            return self.extend(iter);
        }
        let (lower_size_bound, _) = iter.size_hint();
        if !(lower_size_bound <= core::isize::MAX as usize) {
            ::core::panicking::panic(
                "assertion failed: lower_size_bound <= core::isize::MAX as usize",
            )
        }
        if !(index + lower_size_bound >= index) {
            ::core::panicking::panic(
                "assertion failed: index + lower_size_bound >= index",
            )
        }
        let mut num_added = 0;
        let old_len = self.len();
        if !(index <= old_len) {
            ::core::panicking::panic("assertion failed: index <= old_len")
        }
        unsafe {
            self.reserve(lower_size_bound);
            let start = self.as_mut_ptr();
            let ptr = start.add(index);
            ptr::copy(ptr, ptr.add(lower_size_bound), old_len - index);
            self.set_len(0);
            let mut guard = DropOnPanic {
                start,
                skip: index..(index + lower_size_bound),
                len: old_len + lower_size_bound,
            };
            let start = self.as_mut_ptr();
            let ptr = start.add(index);
            while num_added < lower_size_bound {
                let element = match iter.next() {
                    Some(x) => x,
                    None => break,
                };
                let cur = ptr.add(num_added);
                ptr::write(cur, element);
                guard.skip.start += 1;
                num_added += 1;
            }
            if num_added < lower_size_bound {
                ptr::copy(
                    ptr.add(lower_size_bound),
                    ptr.add(num_added),
                    old_len - index,
                );
            }
            self.set_len(old_len + num_added);
            mem::forget(guard);
        }
        for element in iter {
            self.insert(index + num_added, element);
            num_added += 1;
        }
        struct DropOnPanic<T> {
            start: *mut T,
            skip: Range<usize>,
            len: usize,
        }
        impl<T> Drop for DropOnPanic<T> {
            fn drop(&mut self) {
                for i in 0..self.len {
                    if !self.skip.contains(&i) {
                        unsafe {
                            ptr::drop_in_place(self.start.add(i));
                        }
                    }
                }
            }
        }
    }
    /// Convert a `SmallVec` to a `Vec`, without reallocating if the `SmallVec` has already spilled onto
    /// the heap.
    pub fn into_vec(mut self) -> Vec<A::Item> {
        if self.spilled() {
            unsafe {
                let (ptr, &mut len) = self.data.heap_mut();
                let v = Vec::from_raw_parts(ptr.as_ptr(), len, self.capacity);
                mem::forget(self);
                v
            }
        } else {
            self.into_iter().collect()
        }
    }
    /// Converts a `SmallVec` into a `Box<[T]>` without reallocating if the `SmallVec` has already spilled
    /// onto the heap.
    ///
    /// Note that this will drop any excess capacity.
    pub fn into_boxed_slice(self) -> Box<[A::Item]> {
        self.into_vec().into_boxed_slice()
    }
    /// Convert the `SmallVec` into an `A` if possible. Otherwise return `Err(Self)`.
    ///
    /// This method returns `Err(Self)` if the `SmallVec` is too short (and the `A` contains uninitialized elements),
    /// or if the `SmallVec` is too long (and all the elements were spilled to the heap).
    pub fn into_inner(self) -> Result<A, Self> {
        if self.spilled() || self.len() != A::size() {
            Err(self)
        } else {
            unsafe {
                let data = ptr::read(&self.data);
                mem::forget(self);
                Ok(data.into_inline().assume_init())
            }
        }
    }
    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` such that `f(&e)` returns `false`.
    /// This method operates in place and preserves the order of the retained
    /// elements.
    pub fn retain<F: FnMut(&mut A::Item) -> bool>(&mut self, mut f: F) {
        let mut del = 0;
        let len = self.len();
        for i in 0..len {
            if !f(&mut self[i]) {
                del += 1;
            } else if del > 0 {
                self.swap(i - del, i);
            }
        }
        self.truncate(len - del);
    }
    /// Retains only the elements specified by the predicate.
    ///
    /// This method is identical in behaviour to [`retain`]; it is included only
    /// to maintain api-compatibility with `std::Vec`, where the methods are
    /// separate for historical reasons.
    pub fn retain_mut<F: FnMut(&mut A::Item) -> bool>(&mut self, f: F) {
        self.retain(f)
    }
    /// Removes consecutive duplicate elements.
    pub fn dedup(&mut self)
    where
        A::Item: PartialEq<A::Item>,
    {
        self.dedup_by(|a, b| a == b);
    }
    /// Removes consecutive duplicate elements using the given equality relation.
    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut A::Item, &mut A::Item) -> bool,
    {
        let len = self.len();
        if len <= 1 {
            return;
        }
        let ptr = self.as_mut_ptr();
        let mut w: usize = 1;
        unsafe {
            for r in 1..len {
                let p_r = ptr.add(r);
                let p_wm1 = ptr.add(w - 1);
                if !same_bucket(&mut *p_r, &mut *p_wm1) {
                    if r != w {
                        let p_w = p_wm1.add(1);
                        mem::swap(&mut *p_r, &mut *p_w);
                    }
                    w += 1;
                }
            }
        }
        self.truncate(w);
    }
    /// Removes consecutive elements that map to the same key.
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut A::Item) -> K,
        K: PartialEq<K>,
    {
        self.dedup_by(|a, b| key(a) == key(b));
    }
    /// Resizes the `SmallVec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `SmallVec` is extended by the difference, with each
    /// additional slot filled with the result of calling the closure `f`. The return values from `f`
    /// will end up in the `SmallVec` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `SmallVec` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If you'd rather `Clone` a given
    /// value, use `resize`. If you want to use the `Default` trait to generate values, you can pass
    /// `Default::default()` as the second argument.
    ///
    /// Added for `std::vec::Vec` compatibility (added in Rust 1.33.0)
    ///
    /// ```
    /// # use smallvec::{smallvec, SmallVec};
    /// let mut vec : SmallVec<[_; 4]> = smallvec![1, 2, 3];
    /// vec.resize_with(5, Default::default);
    /// assert_eq!(&*vec, &[1, 2, 3, 0, 0]);
    ///
    /// let mut vec : SmallVec<[_; 4]> = smallvec![];
    /// let mut p = 1;
    /// vec.resize_with(4, || { p *= 2; p });
    /// assert_eq!(&*vec, &[2, 4, 8, 16]);
    /// ```
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> A::Item,
    {
        let old_len = self.len();
        if old_len < new_len {
            let mut f = f;
            let additional = new_len - old_len;
            self.reserve(additional);
            for _ in 0..additional {
                self.push(f());
            }
        } else if old_len > new_len {
            self.truncate(new_len);
        }
    }
    /// Creates a `SmallVec` directly from the raw components of another
    /// `SmallVec`.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * `ptr` needs to have been previously allocated via `SmallVec` for its
    ///   spilled storage (at least, it's highly likely to be incorrect if it
    ///   wasn't).
    /// * `ptr`'s `A::Item` type needs to be the same size and alignment that
    ///   it was allocated with
    /// * `length` needs to be less than or equal to `capacity`.
    /// * `capacity` needs to be the capacity that the pointer was allocated
    ///   with.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures.
    ///
    /// Additionally, `capacity` must be greater than the amount of inline
    /// storage `A` has; that is, the new `SmallVec` must need to spill over
    /// into heap allocated storage. This condition is asserted against.
    ///
    /// The ownership of `ptr` is effectively transferred to the
    /// `SmallVec` which may then deallocate, reallocate or change the
    /// contents of memory pointed to by the pointer at will. Ensure
    /// that nothing else uses the pointer after calling this
    /// function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use smallvec::{smallvec, SmallVec};
    /// use std::mem;
    /// use std::ptr;
    ///
    /// fn main() {
    ///     let mut v: SmallVec<[_; 1]> = smallvec![1, 2, 3];
    ///
    ///     // Pull out the important parts of `v`.
    ///     let p = v.as_mut_ptr();
    ///     let len = v.len();
    ///     let cap = v.capacity();
    ///     let spilled = v.spilled();
    ///
    ///     unsafe {
    ///         // Forget all about `v`. The heap allocation that stored the
    ///         // three values won't be deallocated.
    ///         mem::forget(v);
    ///
    ///         // Overwrite memory with [4, 5, 6].
    ///         //
    ///         // This is only safe if `spilled` is true! Otherwise, we are
    ///         // writing into the old `SmallVec`'s inline storage on the
    ///         // stack.
    ///         assert!(spilled);
    ///         for i in 0..len {
    ///             ptr::write(p.add(i), 4 + i);
    ///         }
    ///
    ///         // Put everything back together into a SmallVec with a different
    ///         // amount of inline storage, but which is still less than `cap`.
    ///         let rebuilt = SmallVec::<[_; 2]>::from_raw_parts(p, len, cap);
    ///         assert_eq!(&*rebuilt, &[4, 5, 6]);
    ///     }
    /// }
    #[inline]
    pub unsafe fn from_raw_parts(
        ptr: *mut A::Item,
        length: usize,
        capacity: usize,
    ) -> SmallVec<A> {
        let ptr = unsafe {
            if true {
                if !!ptr.is_null() {
                    ::core::panicking::panic(
                        "Called `from_raw_parts` with null pointer.",
                    )
                }
            }
            NonNull::new_unchecked(ptr)
        };
        if !(capacity > Self::inline_capacity()) {
            ::core::panicking::panic(
                "assertion failed: capacity > Self::inline_capacity()",
            )
        }
        SmallVec {
            capacity,
            data: SmallVecData::from_heap(ptr, length),
        }
    }
    /// Returns a raw pointer to the vector's buffer.
    pub fn as_ptr(&self) -> *const A::Item {
        self.triple().0.as_ptr()
    }
    /// Returns a raw mutable pointer to the vector's buffer.
    pub fn as_mut_ptr(&mut self) -> *mut A::Item {
        self.triple_mut().0.as_ptr()
    }
}
impl<A: Array> SmallVec<A>
where
    A::Item: Copy,
{
    /// Copy the elements from a slice into a new `SmallVec`.
    ///
    /// For slices of `Copy` types, this is more efficient than `SmallVec::from(slice)`.
    pub fn from_slice(slice: &[A::Item]) -> Self {
        let len = slice.len();
        if len <= Self::inline_capacity() {
            SmallVec {
                capacity: len,
                data: SmallVecData::from_inline(unsafe {
                    let mut data: MaybeUninit<A> = MaybeUninit::uninit();
                    ptr::copy_nonoverlapping(
                        slice.as_ptr(),
                        data.as_mut_ptr() as *mut A::Item,
                        len,
                    );
                    data
                }),
            }
        } else {
            let mut b = slice.to_vec();
            let cap = b.capacity();
            let ptr = NonNull::new(b.as_mut_ptr())
                .expect("Vec always contain non null pointers.");
            mem::forget(b);
            SmallVec {
                capacity: cap,
                data: SmallVecData::from_heap(ptr, len),
            }
        }
    }
    /// Copy elements from a slice into the vector at position `index`, shifting any following
    /// elements toward the back.
    ///
    /// For slices of `Copy` types, this is more efficient than `insert`.
    #[inline]
    pub fn insert_from_slice(&mut self, index: usize, slice: &[A::Item]) {
        self.reserve(slice.len());
        let len = self.len();
        if !(index <= len) {
            ::core::panicking::panic("assertion failed: index <= len")
        }
        unsafe {
            let slice_ptr = slice.as_ptr();
            let ptr = self.as_mut_ptr().add(index);
            ptr::copy(ptr, ptr.add(slice.len()), len - index);
            ptr::copy_nonoverlapping(slice_ptr, ptr, slice.len());
            self.set_len(len + slice.len());
        }
    }
    /// Copy elements from a slice and append them to the vector.
    ///
    /// For slices of `Copy` types, this is more efficient than `extend`.
    #[inline]
    pub fn extend_from_slice(&mut self, slice: &[A::Item]) {
        let len = self.len();
        self.insert_from_slice(len, slice);
    }
}
impl<A: Array> SmallVec<A>
where
    A::Item: Clone,
{
    /// Resizes the vector so that its length is equal to `len`.
    ///
    /// If `len` is less than the current length, the vector simply truncated.
    ///
    /// If `len` is greater than the current length, `value` is appended to the
    /// vector until its length equals `len`.
    pub fn resize(&mut self, len: usize, value: A::Item) {
        let old_len = self.len();
        if len > old_len {
            self.extend(repeat(value).take(len - old_len));
        } else {
            self.truncate(len);
        }
    }
    /// Creates a `SmallVec` with `n` copies of `elem`.
    /// ```
    /// use smallvec::SmallVec;
    ///
    /// let v = SmallVec::<[char; 128]>::from_elem('d', 2);
    /// assert_eq!(v, SmallVec::from_buf(['d', 'd']));
    /// ```
    pub fn from_elem(elem: A::Item, n: usize) -> Self {
        if n > Self::inline_capacity() {
            ::alloc::vec::from_elem(elem, n).into()
        } else {
            let mut v = SmallVec::<A>::new();
            unsafe {
                let (ptr, len_ptr, _) = v.triple_mut();
                let ptr = ptr.as_ptr();
                let mut local_len = SetLenOnDrop::new(len_ptr);
                for i in 0..n {
                    ::core::ptr::write(ptr.add(i), elem.clone());
                    local_len.increment_len(1);
                }
            }
            v
        }
    }
}
impl<A: Array> ops::Deref for SmallVec<A> {
    type Target = [A::Item];
    #[inline]
    fn deref(&self) -> &[A::Item] {
        unsafe {
            let (ptr, len, _) = self.triple();
            slice::from_raw_parts(ptr.as_ptr(), len)
        }
    }
}
impl<A: Array> ops::DerefMut for SmallVec<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [A::Item] {
        unsafe {
            let (ptr, &mut len, _) = self.triple_mut();
            slice::from_raw_parts_mut(ptr.as_ptr(), len)
        }
    }
}
impl<A: Array> AsRef<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_ref(&self) -> &[A::Item] {
        self
    }
}
impl<A: Array> AsMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [A::Item] {
        self
    }
}
impl<A: Array> Borrow<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow(&self) -> &[A::Item] {
        self
    }
}
impl<A: Array> BorrowMut<[A::Item]> for SmallVec<A> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [A::Item] {
        self
    }
}
impl<'a, A: Array> From<&'a [A::Item]> for SmallVec<A>
where
    A::Item: Clone,
{
    #[inline]
    fn from(slice: &'a [A::Item]) -> SmallVec<A> {
        slice.iter().cloned().collect()
    }
}
impl<A: Array> From<Vec<A::Item>> for SmallVec<A> {
    #[inline]
    fn from(vec: Vec<A::Item>) -> SmallVec<A> {
        SmallVec::from_vec(vec)
    }
}
impl<A: Array> From<A> for SmallVec<A> {
    #[inline]
    fn from(array: A) -> SmallVec<A> {
        SmallVec::from_buf(array)
    }
}
impl<A: Array, I: SliceIndex<[A::Item]>> ops::Index<I> for SmallVec<A> {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        &(**self)[index]
    }
}
impl<A: Array, I: SliceIndex<[A::Item]>> ops::IndexMut<I> for SmallVec<A> {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        &mut (&mut **self)[index]
    }
}
#[allow(deprecated)]
impl<A: Array> ExtendFromSlice<A::Item> for SmallVec<A>
where
    A::Item: Copy,
{
    fn extend_from_slice(&mut self, other: &[A::Item]) {
        SmallVec::extend_from_slice(self, other)
    }
}
impl<A: Array> FromIterator<A::Item> for SmallVec<A> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = A::Item>>(iterable: I) -> SmallVec<A> {
        let mut v = SmallVec::new();
        v.extend(iterable);
        v
    }
}
impl<A: Array> Extend<A::Item> for SmallVec<A> {
    fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iterable: I) {
        let mut iter = iterable.into_iter();
        let (lower_size_bound, _) = iter.size_hint();
        self.reserve(lower_size_bound);
        unsafe {
            let (ptr, len_ptr, cap) = self.triple_mut();
            let ptr = ptr.as_ptr();
            let mut len = SetLenOnDrop::new(len_ptr);
            while len.get() < cap {
                if let Some(out) = iter.next() {
                    ptr::write(ptr.add(len.get()), out);
                    len.increment_len(1);
                } else {
                    return;
                }
            }
        }
        for elem in iter {
            self.push(elem);
        }
    }
}
impl<A: Array> fmt::Debug for SmallVec<A>
where
    A::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl<A: Array> Default for SmallVec<A> {
    #[inline]
    fn default() -> SmallVec<A> {
        SmallVec::new()
    }
}
impl<A: Array> Drop for SmallVec<A> {
    fn drop(&mut self) {
        unsafe {
            if self.spilled() {
                let (ptr, &mut len) = self.data.heap_mut();
                drop(Vec::from_raw_parts(ptr.as_ptr(), len, self.capacity));
            } else {
                ptr::drop_in_place(&mut self[..]);
            }
        }
    }
}
impl<A: Array> Clone for SmallVec<A>
where
    A::Item: Clone,
{
    #[inline]
    fn clone(&self) -> SmallVec<A> {
        SmallVec::from(self.as_slice())
    }
    fn clone_from(&mut self, source: &Self) {
        self.truncate(source.len());
        let (init, tail) = source.split_at(self.len());
        self.clone_from_slice(init);
        self.extend(tail.iter().cloned());
    }
}
impl<A: Array, B: Array> PartialEq<SmallVec<B>> for SmallVec<A>
where
    A::Item: PartialEq<B::Item>,
{
    #[inline]
    fn eq(&self, other: &SmallVec<B>) -> bool {
        self[..] == other[..]
    }
}
impl<A: Array> Eq for SmallVec<A>
where
    A::Item: Eq,
{}
impl<A: Array> PartialOrd for SmallVec<A>
where
    A::Item: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &SmallVec<A>) -> Option<cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}
impl<A: Array> Ord for SmallVec<A>
where
    A::Item: Ord,
{
    #[inline]
    fn cmp(&self, other: &SmallVec<A>) -> cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<A: Array> Hash for SmallVec<A>
where
    A::Item: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
unsafe impl<A: Array> Send for SmallVec<A>
where
    A::Item: Send,
{}
/// An iterator that consumes a `SmallVec` and yields its items by value.
///
/// Returned from [`SmallVec::into_iter`][1].
///
/// [1]: struct.SmallVec.html#method.into_iter
pub struct IntoIter<A: Array> {
    data: SmallVec<A>,
    current: usize,
    end: usize,
}
impl<A: Array> fmt::Debug for IntoIter<A>
where
    A::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}
impl<A: Array + Clone> Clone for IntoIter<A>
where
    A::Item: Clone,
{
    fn clone(&self) -> IntoIter<A> {
        SmallVec::from(self.as_slice()).into_iter()
    }
}
impl<A: Array> Drop for IntoIter<A> {
    fn drop(&mut self) {
        for _ in self {}
    }
}
impl<A: Array> Iterator for IntoIter<A> {
    type Item = A::Item;
    #[inline]
    fn next(&mut self) -> Option<A::Item> {
        if self.current == self.end {
            None
        } else {
            unsafe {
                let current = self.current;
                self.current += 1;
                Some(ptr::read(self.data.as_ptr().add(current)))
            }
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.end - self.current;
        (size, Some(size))
    }
}
impl<A: Array> DoubleEndedIterator for IntoIter<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A::Item> {
        if self.current == self.end {
            None
        } else {
            unsafe {
                self.end -= 1;
                Some(ptr::read(self.data.as_ptr().add(self.end)))
            }
        }
    }
}
impl<A: Array> ExactSizeIterator for IntoIter<A> {}
impl<A: Array> FusedIterator for IntoIter<A> {}
impl<A: Array> IntoIter<A> {
    /// Returns the remaining items of this iterator as a slice.
    pub fn as_slice(&self) -> &[A::Item] {
        let len = self.end - self.current;
        unsafe { core::slice::from_raw_parts(self.data.as_ptr().add(self.current), len) }
    }
    /// Returns the remaining items of this iterator as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [A::Item] {
        let len = self.end - self.current;
        unsafe {
            core::slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(self.current),
                len,
            )
        }
    }
}
impl<A: Array> IntoIterator for SmallVec<A> {
    type IntoIter = IntoIter<A>;
    type Item = A::Item;
    fn into_iter(mut self) -> Self::IntoIter {
        unsafe {
            let len = self.len();
            self.set_len(0);
            IntoIter {
                data: self,
                current: 0,
                end: len,
            }
        }
    }
}
impl<'a, A: Array> IntoIterator for &'a SmallVec<A> {
    type IntoIter = slice::Iter<'a, A::Item>;
    type Item = &'a A::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, A: Array> IntoIterator for &'a mut SmallVec<A> {
    type IntoIter = slice::IterMut<'a, A::Item>;
    type Item = &'a mut A::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
/// Types that can be used as the backing store for a [`SmallVec`].
pub unsafe trait Array {
    /// The type of the array's elements.
    type Item;
    /// Returns the number of items the array can hold.
    fn size() -> usize;
}
/// Set the length of the vec when the `SetLenOnDrop` value goes out of scope.
///
/// Copied from <https://github.com/rust-lang/rust/pull/36355>
struct SetLenOnDrop<'a> {
    len: &'a mut usize,
    local_len: usize,
}
impl<'a> SetLenOnDrop<'a> {
    #[inline]
    fn new(len: &'a mut usize) -> Self {
        SetLenOnDrop {
            local_len: *len,
            len,
        }
    }
    #[inline]
    fn get(&self) -> usize {
        self.local_len
    }
    #[inline]
    fn increment_len(&mut self, increment: usize) {
        self.local_len += increment;
    }
}
impl<'a> Drop for SetLenOnDrop<'a> {
    #[inline]
    fn drop(&mut self) {
        *self.len = self.local_len;
    }
}
unsafe impl<T> Array for [T; 0] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0
    }
}
unsafe impl<T> Array for [T; 1] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        1
    }
}
unsafe impl<T> Array for [T; 2] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        2
    }
}
unsafe impl<T> Array for [T; 3] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        3
    }
}
unsafe impl<T> Array for [T; 4] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        4
    }
}
unsafe impl<T> Array for [T; 5] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        5
    }
}
unsafe impl<T> Array for [T; 6] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        6
    }
}
unsafe impl<T> Array for [T; 7] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        7
    }
}
unsafe impl<T> Array for [T; 8] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        8
    }
}
unsafe impl<T> Array for [T; 9] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        9
    }
}
unsafe impl<T> Array for [T; 10] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        10
    }
}
unsafe impl<T> Array for [T; 11] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        11
    }
}
unsafe impl<T> Array for [T; 12] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        12
    }
}
unsafe impl<T> Array for [T; 13] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        13
    }
}
unsafe impl<T> Array for [T; 14] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        14
    }
}
unsafe impl<T> Array for [T; 15] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        15
    }
}
unsafe impl<T> Array for [T; 16] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        16
    }
}
unsafe impl<T> Array for [T; 17] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        17
    }
}
unsafe impl<T> Array for [T; 18] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        18
    }
}
unsafe impl<T> Array for [T; 19] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        19
    }
}
unsafe impl<T> Array for [T; 20] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        20
    }
}
unsafe impl<T> Array for [T; 21] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        21
    }
}
unsafe impl<T> Array for [T; 22] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        22
    }
}
unsafe impl<T> Array for [T; 23] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        23
    }
}
unsafe impl<T> Array for [T; 24] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        24
    }
}
unsafe impl<T> Array for [T; 25] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        25
    }
}
unsafe impl<T> Array for [T; 26] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        26
    }
}
unsafe impl<T> Array for [T; 27] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        27
    }
}
unsafe impl<T> Array for [T; 28] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        28
    }
}
unsafe impl<T> Array for [T; 29] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        29
    }
}
unsafe impl<T> Array for [T; 30] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        30
    }
}
unsafe impl<T> Array for [T; 31] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        31
    }
}
unsafe impl<T> Array for [T; 32] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        32
    }
}
unsafe impl<T> Array for [T; 36] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        36
    }
}
unsafe impl<T> Array for [T; 0x40] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x40
    }
}
unsafe impl<T> Array for [T; 0x60] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x60
    }
}
unsafe impl<T> Array for [T; 0x80] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x80
    }
}
unsafe impl<T> Array for [T; 0x100] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x100
    }
}
unsafe impl<T> Array for [T; 0x200] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x200
    }
}
unsafe impl<T> Array for [T; 0x400] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x400
    }
}
unsafe impl<T> Array for [T; 0x600] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x600
    }
}
unsafe impl<T> Array for [T; 0x800] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x800
    }
}
unsafe impl<T> Array for [T; 0x1000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x1000
    }
}
unsafe impl<T> Array for [T; 0x2000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x2000
    }
}
unsafe impl<T> Array for [T; 0x4000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x4000
    }
}
unsafe impl<T> Array for [T; 0x6000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x6000
    }
}
unsafe impl<T> Array for [T; 0x8000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x8000
    }
}
unsafe impl<T> Array for [T; 0x10000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x10000
    }
}
unsafe impl<T> Array for [T; 0x20000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x20000
    }
}
unsafe impl<T> Array for [T; 0x40000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x40000
    }
}
unsafe impl<T> Array for [T; 0x60000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x60000
    }
}
unsafe impl<T> Array for [T; 0x80000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x80000
    }
}
unsafe impl<T> Array for [T; 0x10_0000] {
    type Item = T;
    #[inline]
    fn size() -> usize {
        0x10_0000
    }
}
/// Convenience trait for constructing a `SmallVec`
pub trait ToSmallVec<A: Array> {
    /// Construct a new `SmallVec` from a slice.
    fn to_smallvec(&self) -> SmallVec<A>;
}
impl<A: Array> ToSmallVec<A> for [A::Item]
where
    A::Item: Copy,
{
    #[inline]
    fn to_smallvec(&self) -> SmallVec<A> {
        SmallVec::from_slice(self)
    }
}
#[repr(transparent)]
struct ConstNonNull<T>(NonNull<T>);
impl<T> ConstNonNull<T> {
    #[inline]
    fn new(ptr: *const T) -> Option<Self> {
        NonNull::new(ptr as *mut T).map(Self)
    }
    #[inline]
    fn as_ptr(self) -> *const T {
        self.0.as_ptr()
    }
}
impl<T> Clone for ConstNonNull<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ConstNonNull<T> {}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &drain,
            &drain_forget,
            &drain_overflow,
            &drain_rev,
            &empty_macro,
            &grow_spilled_same_size,
            &grow_to_shrink,
            &panic_early_at_end,
            &panic_early_at_start,
            &panic_early_in_middle,
            &panic_late_at_end,
            &panic_late_at_start,
            &into_iter,
            &into_iter_drop,
            &into_iter_rev,
            &issue_4,
            &issue_5,
            &max_dont_panic,
            &max_remove,
            &max_swap_remove,
            &resumable_extend,
            &shrink_to_fit_unspill,
            &test_as_mut,
            &test_as_ref,
            &test_borrow,
            &test_borrow_mut,
            &test_capacity,
            &test_clone_from,
            &test_dedup,
            &test_double_spill,
            &test_drop_panic_smallvec,
            &test_eq,
            &test_exact_size_iterator,
            &test_extend_from_slice,
            &test_from,
            &test_from_slice,
            &test_from_vec,
            &test_hash,
            &test_inline,
            &test_insert_from_slice,
            &test_insert_many,
            &test_insert_many_long_hint,
            &test_insert_many_overflow,
            &test_insert_many_short_hint,
            &test_insert_out_of_bounds,
            &test_into_inner,
            &test_into_iter_as_slice,
            &test_into_iter_clone,
            &test_into_iter_clone_empty_smallvec,
            &test_into_iter_clone_partially_consumed_iterator,
            &test_into_vec,
            &test_invalid_grow,
            &test_ord,
            &test_resize,
            &test_retain,
            &test_size,
            &test_spill,
            &test_truncate,
            &test_with_capacity,
            &test_zero,
            &uninhabited,
            &zero_size_items,
        ],
    )
}
