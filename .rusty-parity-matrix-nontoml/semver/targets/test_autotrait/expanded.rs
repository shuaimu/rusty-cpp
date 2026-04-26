#![feature(prelude_import)]
#![allow(clippy::extra_unused_type_parameters)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
fn assert_send_sync<T: Send + Sync>() {}
extern crate test;
#[rustc_test_marker = "test"]
#[doc(hidden)]
pub const test: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_autotrait.rs",
        start_line: 6usize,
        start_col: 4usize,
        end_line: 6usize,
        end_col: 8usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test())),
};
fn test() {
    assert_send_sync::<semver::BuildMetadata>();
    assert_send_sync::<semver::Comparator>();
    assert_send_sync::<semver::Error>();
    assert_send_sync::<semver::Prerelease>();
    assert_send_sync::<semver::Version>();
    assert_send_sync::<semver::VersionReq>();
    assert_send_sync::<semver::Op>();
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test])
}
