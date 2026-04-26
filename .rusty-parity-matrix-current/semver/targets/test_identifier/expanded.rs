#![feature(prelude_import)]
#![allow(
    clippy::eq_op,
    clippy::needless_pass_by_value,
    clippy::toplevel_ref_arg,
    clippy::wildcard_imports
)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
mod util {
    #![allow(dead_code)]
    use semver::{BuildMetadata, Comparator, Error, Prerelease, Version, VersionReq};
    use std::fmt::Display;
    #[track_caller]
    pub(super) fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    #[track_caller]
    pub(super) fn version_err(text: &str) -> Error {
        Version::parse(text).unwrap_err()
    }
    #[track_caller]
    pub(super) fn req(text: &str) -> VersionReq {
        VersionReq::parse(text).unwrap()
    }
    #[track_caller]
    pub(super) fn req_err(text: &str) -> Error {
        VersionReq::parse(text).unwrap_err()
    }
    #[track_caller]
    pub(super) fn comparator(text: &str) -> Comparator {
        Comparator::parse(text).unwrap()
    }
    #[track_caller]
    pub(super) fn comparator_err(text: &str) -> Error {
        Comparator::parse(text).unwrap_err()
    }
    #[track_caller]
    pub(super) fn prerelease(text: &str) -> Prerelease {
        Prerelease::new(text).unwrap()
    }
    #[track_caller]
    pub(super) fn prerelease_err(text: &str) -> Error {
        Prerelease::new(text).unwrap_err()
    }
    #[track_caller]
    pub(super) fn build_metadata(text: &str) -> BuildMetadata {
        BuildMetadata::new(text).unwrap()
    }
    #[track_caller]
    pub(super) fn assert_to_string(value: impl Display, expected: &str) {
        match (&value.to_string(), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
use crate::util::*;
use semver::Prerelease;
extern crate test;
#[rustc_test_marker = "test_new"]
#[doc(hidden)]
pub const test_new: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_new"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_identifier.rs",
        start_line: 14usize,
        start_col: 4usize,
        end_line: 14usize,
        end_col: 12usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_new())),
};
fn test_new() {
    fn test(identifier: Prerelease, expected: &str) {
        match (&identifier.is_empty(), &expected.is_empty()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&identifier.len(), &expected.len()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&identifier.as_str(), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&identifier, &identifier) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&identifier, &identifier.clone()) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    let ref mut string = String::new();
    let limit = if false { 40 } else { 280 };
    for _ in 0..limit {
        test(prerelease(string), string);
        string.push('1');
    }
    if !false {
        let ref string = string.repeat(20000);
        test(prerelease(string), string);
    }
}
extern crate test;
#[rustc_test_marker = "test_eq"]
#[doc(hidden)]
pub const test_eq: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_eq"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_identifier.rs",
        start_line: 37usize,
        start_col: 4usize,
        end_line: 37usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_eq())),
};
fn test_eq() {
    match (&prerelease("-"), &prerelease("-")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&prerelease("a"), &prerelease("aa")) {
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
    match (&prerelease("aa"), &prerelease("a")) {
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
    match (&prerelease("aaaaaaaaa"), &prerelease("a")) {
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
    match (&prerelease("a"), &prerelease("aaaaaaaaa")) {
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
    match (&prerelease("aaaaaaaaa"), &prerelease("bbbbbbbbb")) {
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
    match (&build_metadata("1"), &build_metadata("001")) {
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
#[rustc_test_marker = "test_prerelease"]
#[doc(hidden)]
pub const test_prerelease: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_prerelease"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_identifier.rs",
        start_line: 48usize,
        start_col: 4usize,
        end_line: 48usize,
        end_col: 19usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_prerelease()),
    ),
};
fn test_prerelease() {
    let err = prerelease_err("1.b\0");
    assert_to_string(err, "unexpected character in pre-release identifier");
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_eq, &test_new, &test_prerelease])
}
