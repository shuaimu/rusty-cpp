#![feature(prelude_import)]
//! A macro for defining `#[cfg]` if-else statements.
//!
//! The macro provided by this crate, `cfg_if`, is similar to the `if/elif` C
//! preprocessor macro by allowing definition of a cascade of `#[cfg]` cases,
//! emitting the implementation which matches first.
//!
//! This allows you to conveniently provide a long list `#[cfg]`'d blocks of code
//! without having to rewrite each clause multiple times.
//!
//! # Example
//!
//! ```
//! cfg_if::cfg_if! {
//!     if #[cfg(unix)] {
//!         fn foo() { /* unix specific functionality */ }
//!     } else if #[cfg(target_pointer_width = "32")] {
//!         fn foo() { /* non-unix, 32-bit functionality */ }
//!     } else {
//!         fn foo() { /* fallback implementation */ }
//!     }
//! }
//!
//! # fn main() {}
//! ```
#![no_std]
#![doc(html_root_url = "https://docs.rs/cfg-if")]
#![deny(missing_docs)]
#![allow(unexpected_cfgs)]
extern crate core;
#[prelude_import]
use core::prelude::rust_2018::*;
mod tests {
    use core::option::Option as Option2;
    fn works1() -> Option2<u32> {
        Some(1)
    }
    fn works2() -> bool {
        true
    }
    fn works3() -> bool {
        true
    }
    use core::option::Option as Option3;
    fn works4() -> Option3<u32> {
        Some(1)
    }
    fn works5() -> bool {
        true
    }
    type _A = i32;
    type _B = i32;
    fn works6() -> bool {
        true
    }
    extern crate test;
    #[rustc_test_marker = "tests::it_works"]
    #[doc(hidden)]
    pub const it_works: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::it_works"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "/home/shuai/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cfg-if-1.0.4/src/lib.rs",
            start_line: 169usize,
            start_col: 8usize,
            end_line: 169usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(it_works()),
        ),
    };
    fn it_works() {
        if !works1().is_some() {
            ::core::panicking::panic("assertion failed: works1().is_some()")
        }
        if !works2() {
            ::core::panicking::panic("assertion failed: works2()")
        }
        if !works3() {
            ::core::panicking::panic("assertion failed: works3()")
        }
        if !works4().is_some() {
            ::core::panicking::panic("assertion failed: works4().is_some()")
        }
        if !works5() {
            ::core::panicking::panic("assertion failed: works5()")
        }
        if !works6() {
            ::core::panicking::panic("assertion failed: works6()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "tests::test_usage_within_a_function"]
    #[doc(hidden)]
    pub const test_usage_within_a_function: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("tests::test_usage_within_a_function"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "/home/shuai/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cfg-if-1.0.4/src/lib.rs",
            start_line: 181usize,
            start_col: 8usize,
            end_line: 181usize,
            end_col: 36usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_usage_within_a_function()),
        ),
    };
    #[allow(clippy::assertions_on_constants)]
    fn test_usage_within_a_function() {
        if !true {
            ::core::panicking::panic("assertion failed: cfg!(debug_assertions)")
        }
        match (&4, &(2 + 2)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[allow(dead_code)]
    trait Trait {
        fn blah(&self);
    }
    #[allow(dead_code)]
    struct Struct;
    impl Trait for Struct {
        fn blah(&self) {
            ::core::panicking::panic("not implemented");
        }
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&it_works, &test_usage_within_a_function])
}
