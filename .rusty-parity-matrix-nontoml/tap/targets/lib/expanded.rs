#![feature(prelude_import)]
#![no_std]
extern crate std;
#[prelude_import]
use ::std::prelude::rust_2015::*;
extern crate tap;
use tap::*;
extern crate test;
#[rustc_test_marker = "filter_map"]
#[doc(hidden)]
pub const filter_map: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("filter_map"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/lib.rs",
        start_line: 6usize,
        start_col: 4usize,
        end_line: 6usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(filter_map()),
    ),
};
fn filter_map() {
    let values: &[Result<i32, &str>] = &[Ok(3), Err("foo"), Err("bar"), Ok(8)];
    let _ = values
        .iter()
        .filter_map(|result| {
            result
                .tap_err(|error| {
                    ::std::io::_print(format_args!("Invalid entry: {0}\n", error));
                })
                .ok()
        });
}
extern crate test;
#[rustc_test_marker = "basic"]
#[doc(hidden)]
pub const basic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("basic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/lib.rs",
        start_line: 18usize,
        start_col: 4usize,
        end_line: 18usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(basic())),
};
fn basic() {
    let mut foo = 5;
    if 10.tap(|v| foo += *v) > 0 {
        match (&foo, &15) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    let _: Result<i32, i32> = Err(5).tap_err(|e| foo = *e);
    match (&foo, &5) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let _: Option<i32> = None.tap_none(|| foo = 10);
    match (&foo, &10) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
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
    test::test_main_static(&[&basic, &filter_map])
}
