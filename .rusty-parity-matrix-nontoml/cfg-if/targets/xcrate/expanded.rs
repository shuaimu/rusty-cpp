#![feature(prelude_import)]
#![allow(unexpected_cfgs)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
fn works() -> bool {
    true
}
extern crate test;
#[rustc_test_marker = "smoke"]
#[doc(hidden)]
pub const smoke: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("smoke"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "/home/shuai/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cfg-if-1.0.4/tests/xcrate.rs",
        start_line: 14usize,
        start_col: 4usize,
        end_line: 14usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(smoke())),
};
fn smoke() {
    if !works() {
        ::core::panicking::panic("assertion failed: works()")
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&smoke])
}
