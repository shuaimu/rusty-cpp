#![feature(prelude_import)]
#![allow(
    clippy::missing_panics_doc,
    clippy::shadow_unrelated,
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
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use semver::VersionReq;
#[track_caller]
fn assert_match_all(req: &VersionReq, versions: &[&str]) {
    for string in versions {
        let parsed = version(string);
        if !req.matches(&parsed) {
            {
                ::std::rt::panic_fmt(format_args!("did not match {0}", string));
            }
        }
    }
}
#[track_caller]
fn assert_match_none(req: &VersionReq, versions: &[&str]) {
    for string in versions {
        let parsed = version(string);
        if !!req.matches(&parsed) {
            {
                ::std::rt::panic_fmt(format_args!("matched {0}", string));
            }
        }
    }
}
extern crate test;
#[rustc_test_marker = "test_basic"]
#[doc(hidden)]
pub const test_basic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_basic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 37usize,
        start_col: 4usize,
        end_line: 37usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_basic()),
    ),
};
fn test_basic() {
    let ref r = req("1.0.0");
    assert_to_string(r, "^1.0.0");
    assert_match_all(r, &["1.0.0", "1.1.0", "1.0.1"]);
    assert_match_none(r, &["0.9.9", "0.10.0", "0.1.0", "1.0.0-pre", "1.0.1-pre"]);
}
extern crate test;
#[rustc_test_marker = "test_default"]
#[doc(hidden)]
pub const test_default: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_default"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 46usize,
        start_col: 4usize,
        end_line: 46usize,
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
    let ref r = VersionReq::default();
    match (&r, &&VersionReq::STAR) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
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
#[rustc_test_marker = "test_exact"]
#[doc(hidden)]
pub const test_exact: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_exact"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 52usize,
        start_col: 4usize,
        end_line: 52usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_exact()),
    ),
};
fn test_exact() {
    let ref r = req("=1.0.0");
    assert_to_string(r, "=1.0.0");
    assert_match_all(r, &["1.0.0"]);
    assert_match_none(r, &["1.0.1", "0.9.9", "0.10.0", "0.1.0", "1.0.0-pre"]);
    let ref r = req("=0.9.0");
    assert_to_string(r, "=0.9.0");
    assert_match_all(r, &["0.9.0"]);
    assert_match_none(r, &["0.9.1", "1.9.0", "0.0.9", "0.9.0-pre"]);
    let ref r = req("=0.0.2");
    assert_to_string(r, "=0.0.2");
    assert_match_all(r, &["0.0.2"]);
    assert_match_none(r, &["0.0.1", "0.0.3", "0.0.2-pre"]);
    let ref r = req("=0.1.0-beta2.a");
    assert_to_string(r, "=0.1.0-beta2.a");
    assert_match_all(r, &["0.1.0-beta2.a"]);
    assert_match_none(r, &["0.9.1", "0.1.0", "0.1.1-beta2.a", "0.1.0-beta2"]);
    let ref r = req("=0.1.0+meta");
    assert_to_string(r, "=0.1.0");
    assert_match_all(r, &["0.1.0", "0.1.0+meta", "0.1.0+any"]);
}
extern crate test;
#[rustc_test_marker = "test_greater_than"]
#[doc(hidden)]
pub const test_greater_than: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_greater_than"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 79usize,
        start_col: 8usize,
        end_line: 79usize,
        end_col: 25usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_greater_than()),
    ),
};
pub fn test_greater_than() {
    let ref r = req(">= 1.0.0");
    assert_to_string(r, ">=1.0.0");
    assert_match_all(r, &["1.0.0", "2.0.0"]);
    assert_match_none(r, &["0.1.0", "0.0.1", "1.0.0-pre", "2.0.0-pre"]);
    let ref r = req(">= 2.1.0-alpha2");
    assert_to_string(r, ">=2.1.0-alpha2");
    assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha3", "2.1.0", "3.0.0"]);
    assert_match_none(r, &["2.0.0", "2.1.0-alpha1", "2.0.0-alpha2", "3.0.0-alpha2"]);
}
extern crate test;
#[rustc_test_marker = "test_less_than"]
#[doc(hidden)]
pub const test_less_than: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_less_than"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 95usize,
        start_col: 8usize,
        end_line: 95usize,
        end_col: 22usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_less_than()),
    ),
};
pub fn test_less_than() {
    let ref r = req("< 1.0.0");
    assert_to_string(r, "<1.0.0");
    assert_match_all(r, &["0.1.0", "0.0.1"]);
    assert_match_none(r, &["1.0.0", "1.0.0-beta", "1.0.1", "0.9.9-alpha"]);
    let ref r = req("<= 2.1.0-alpha2");
    assert_match_all(r, &["2.1.0-alpha2", "2.1.0-alpha1", "2.0.0", "1.0.0"]);
    assert_match_none(r, &["2.1.0", "2.2.0-alpha1", "2.0.0-alpha2", "1.0.0-alpha2"]);
    let ref r = req(">1.0.0-alpha, <1.0.0");
    assert_match_all(r, &["1.0.0-beta"]);
    let ref r = req(">1.0.0-alpha, <1.0");
    assert_match_none(r, &["1.0.0-beta"]);
    let ref r = req(">1.0.0-alpha, <1");
    assert_match_none(r, &["1.0.0-beta"]);
}
extern crate test;
#[rustc_test_marker = "test_multiple"]
#[doc(hidden)]
pub const test_multiple: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_multiple"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 119usize,
        start_col: 8usize,
        end_line: 119usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_multiple()),
    ),
};
pub fn test_multiple() {
    let ref r = req("> 0.0.9, <= 2.5.3");
    assert_to_string(r, ">0.0.9, <=2.5.3");
    assert_match_all(r, &["0.0.10", "1.0.0", "2.5.3"]);
    assert_match_none(r, &["0.0.8", "2.5.4"]);
    let ref r = req("0.3.0, 0.4.0");
    assert_to_string(r, "^0.3.0, ^0.4.0");
    assert_match_none(r, &["0.0.8", "0.3.0", "0.4.0"]);
    let ref r = req("<= 0.2.0, >= 0.5.0");
    assert_to_string(r, "<=0.2.0, >=0.5.0");
    assert_match_none(r, &["0.0.8", "0.3.0", "0.5.1"]);
    let ref r = req("0.1.0, 0.1.4, 0.1.6");
    assert_to_string(r, "^0.1.0, ^0.1.4, ^0.1.6");
    assert_match_all(r, &["0.1.6", "0.1.9"]);
    assert_match_none(r, &["0.1.0", "0.1.4", "0.2.0"]);
    let err = req_err("> 0.1.0,");
    assert_to_string(err, "unexpected end of input while parsing major version number");
    let err = req_err("> 0.3.0, ,");
    assert_to_string(err, "unexpected character ',' while parsing major version number");
    let ref r = req(">=0.5.1-alpha3, <0.6");
    assert_to_string(r, ">=0.5.1-alpha3, <0.6");
    assert_match_all(
        r,
        &["0.5.1-alpha3", "0.5.1-alpha4", "0.5.1-beta", "0.5.1", "0.5.5"],
    );
    assert_match_none(r, &["0.5.1-alpha1", "0.5.2-alpha3", "0.5.5-pre", "0.5.0-pre"]);
    assert_match_none(r, &["0.6.0", "0.6.0-pre"]);
    let err = req_err("1.2.3 - 2.3.4");
    assert_to_string(err, "expected comma after patch version number, found '-'");
    let err = req_err(
        ">1, >2, >3, >4, >5, >6, >7, >8, >9, >10, >11, >12, >13, >14, >15, >16, >17, >18, >19, >20, >21, >22, >23, >24, >25, >26, >27, >28, >29, >30, >31, >32, >33",
    );
    assert_to_string(err, "excessive number of version comparators");
}
extern crate test;
#[rustc_test_marker = "test_whitespace_delimited_comparator_sets"]
#[doc(hidden)]
pub const test_whitespace_delimited_comparator_sets: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_whitespace_delimited_comparator_sets"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 177usize,
        start_col: 8usize,
        end_line: 177usize,
        end_col: 49usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_whitespace_delimited_comparator_sets()),
    ),
};
pub fn test_whitespace_delimited_comparator_sets() {
    let err = req_err("> 0.0.9 <= 2.5.3");
    assert_to_string(err, "expected comma after patch version number, found '<'");
}
extern crate test;
#[rustc_test_marker = "test_tilde"]
#[doc(hidden)]
pub const test_tilde: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_tilde"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 184usize,
        start_col: 8usize,
        end_line: 184usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_tilde()),
    ),
};
pub fn test_tilde() {
    let ref r = req("~1");
    assert_match_all(r, &["1.0.0", "1.0.1", "1.1.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "0.0.9"]);
    let ref r = req("~1.2");
    assert_match_all(r, &["1.2.0", "1.2.1"]);
    assert_match_none(r, &["1.1.1", "1.3.0", "0.0.9"]);
    let ref r = req("~1.2.2");
    assert_match_all(r, &["1.2.2", "1.2.4"]);
    assert_match_none(r, &["1.2.1", "1.9.0", "1.0.9", "2.0.1", "0.1.3"]);
    let ref r = req("~1.2.3-beta.2");
    assert_match_all(r, &["1.2.3", "1.2.4", "1.2.3-beta.2", "1.2.3-beta.4"]);
    assert_match_none(r, &["1.3.3", "1.1.4", "1.2.3-beta.1", "1.2.4-beta.2"]);
}
extern crate test;
#[rustc_test_marker = "test_caret"]
#[doc(hidden)]
pub const test_caret: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_caret"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 203usize,
        start_col: 8usize,
        end_line: 203usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_caret()),
    ),
};
pub fn test_caret() {
    let ref r = req("^1");
    assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1", "1.0.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "0.1.4"]);
    assert_match_none(r, &["1.0.0-beta1", "0.1.0-alpha", "1.0.1-pre"]);
    let ref r = req("^1.1");
    assert_match_all(r, &["1.1.2", "1.1.0", "1.2.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.0.1", "0.1.4"]);
    let ref r = req("^1.1.2");
    assert_match_all(r, &["1.1.2", "1.1.4", "1.2.1"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
    assert_match_none(r, &["1.1.2-alpha1", "1.1.3-alpha1", "2.9.0-alpha1"]);
    let ref r = req("^0.1.2");
    assert_match_all(r, &["0.1.2", "0.1.4"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1"]);
    assert_match_none(r, &["0.1.2-beta", "0.1.3-alpha", "0.2.0-pre"]);
    let ref r = req("^0.5.1-alpha3");
    assert_match_all(
        r,
        &["0.5.1-alpha3", "0.5.1-alpha4", "0.5.1-beta", "0.5.1", "0.5.5"],
    );
    assert_match_none(
        r,
        &["0.5.1-alpha1", "0.5.2-alpha3", "0.5.5-pre", "0.5.0-pre", "0.6.0"],
    );
    let ref r = req("^0.0.2");
    assert_match_all(r, &["0.0.2"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.0.1", "0.1.4"]);
    let ref r = req("^0.0");
    assert_match_all(r, &["0.0.2", "0.0.0"]);
    assert_match_none(r, &["0.9.1", "2.9.0", "1.1.1", "0.1.4"]);
    let ref r = req("^0");
    assert_match_all(r, &["0.9.1", "0.0.2", "0.0.0"]);
    assert_match_none(r, &["2.9.0", "1.1.1"]);
    let ref r = req("^1.4.2-beta.5");
    assert_match_all(r, &["1.4.2", "1.4.3", "1.4.2-beta.5", "1.4.2-beta.6", "1.4.2-c"]);
    assert_match_none(
        r,
        &["0.9.9", "2.0.0", "1.4.2-alpha", "1.4.2-beta.4", "1.4.3-beta.5"],
    );
}
extern crate test;
#[rustc_test_marker = "test_wildcard"]
#[doc(hidden)]
pub const test_wildcard: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_wildcard"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 275usize,
        start_col: 8usize,
        end_line: 275usize,
        end_col: 21usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_wildcard()),
    ),
};
pub fn test_wildcard() {
    let err = req_err("");
    assert_to_string(err, "unexpected end of input while parsing major version number");
    let ref r = req("*");
    assert_match_all(r, &["0.9.1", "2.9.0", "0.0.9", "1.0.1", "1.1.1"]);
    assert_match_none(r, &["1.0.0-pre"]);
    for s in &["x", "X"] {
        match (&*r, &req(s)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    let ref r = req("1.*");
    assert_match_all(r, &["1.2.0", "1.2.1", "1.1.1", "1.3.0"]);
    assert_match_none(r, &["0.0.9", "1.2.0-pre"]);
    for s in &["1.x", "1.X", "1.*.*"] {
        match (&*r, &req(s)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    let ref r = req("1.2.*");
    assert_match_all(r, &["1.2.0", "1.2.2", "1.2.4"]);
    assert_match_none(r, &["1.9.0", "1.0.9", "2.0.1", "0.1.3", "1.2.2-pre"]);
    for s in &["1.2.x", "1.2.X"] {
        match (&*r, &req(s)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
#[rustc_test_marker = "test_logical_or"]
#[doc(hidden)]
pub const test_logical_or: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_logical_or"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 308usize,
        start_col: 8usize,
        end_line: 308usize,
        end_col: 23usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_logical_or()),
    ),
};
pub fn test_logical_or() {
    let err = req_err("=1.2.3 || =2.3.4");
    assert_to_string(err, "expected comma after patch version number, found '|'");
    let err = req_err("1.1 || =1.2.3");
    assert_to_string(err, "expected comma after minor version number, found '|'");
    let err = req_err("6.* || 8.* || >= 10.*");
    assert_to_string(err, "expected comma after minor version number, found '|'");
}
extern crate test;
#[rustc_test_marker = "test_any"]
#[doc(hidden)]
pub const test_any: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_any"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 321usize,
        start_col: 8usize,
        end_line: 321usize,
        end_col: 16usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_any())),
};
pub fn test_any() {
    let ref r = VersionReq::STAR;
    assert_match_all(r, &["0.0.1", "0.1.0", "1.0.0"]);
}
extern crate test;
#[rustc_test_marker = "test_pre"]
#[doc(hidden)]
pub const test_pre: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_pre"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 332usize,
        start_col: 8usize,
        end_line: 332usize,
        end_col: 16usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_pre())),
};
pub fn test_pre() {
    let ref r = req("=2.1.1-really.0");
    assert_match_all(r, &["2.1.1-really.0"]);
}
extern crate test;
#[rustc_test_marker = "test_parse"]
#[doc(hidden)]
pub const test_parse: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_parse"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 338usize,
        start_col: 8usize,
        end_line: 338usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_parse()),
    ),
};
pub fn test_parse() {
    let err = req_err("\0");
    assert_to_string(
        err,
        "unexpected character '\\0' while parsing major version number",
    );
    let err = req_err(">= >= 0.0.2");
    assert_to_string(err, "unexpected character '>' while parsing major version number");
    let err = req_err(">== 0.0.2");
    assert_to_string(err, "unexpected character '=' while parsing major version number");
    let err = req_err("a.0.0");
    assert_to_string(err, "unexpected character 'a' while parsing major version number");
    let err = req_err("1.0.0-");
    assert_to_string(err, "empty identifier segment in pre-release identifier");
    let err = req_err(">=");
    assert_to_string(err, "unexpected end of input while parsing major version number");
}
extern crate test;
#[rustc_test_marker = "test_comparator_parse"]
#[doc(hidden)]
pub const test_comparator_parse: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_comparator_parse"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 374usize,
        start_col: 4usize,
        end_line: 374usize,
        end_col: 25usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_comparator_parse()),
    ),
};
fn test_comparator_parse() {
    let parsed = comparator("1.2.3-alpha");
    assert_to_string(parsed, "^1.2.3-alpha");
    let parsed = comparator("2.X");
    assert_to_string(parsed, "2.*");
    let parsed = comparator("2");
    assert_to_string(parsed, "^2");
    let parsed = comparator("2.x.x");
    assert_to_string(parsed, "2.*");
    let err = comparator_err("1.2.3-01");
    assert_to_string(err, "invalid leading zero in pre-release identifier");
    let err = comparator_err("1.2.3+4.");
    assert_to_string(err, "empty identifier segment in build metadata");
    let err = comparator_err(">");
    assert_to_string(err, "unexpected end of input while parsing major version number");
    let err = comparator_err("1.");
    assert_to_string(err, "unexpected end of input while parsing minor version number");
    let err = comparator_err("1.*.");
    assert_to_string(err, "unexpected character after wildcard in version req");
    let err = comparator_err("1.2.3+4ÿ");
    assert_to_string(err, "unexpected character 'ÿ' after build metadata");
}
extern crate test;
#[rustc_test_marker = "test_cargo3202"]
#[doc(hidden)]
pub const test_cargo3202: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_cargo3202"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 413usize,
        start_col: 4usize,
        end_line: 413usize,
        end_col: 18usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_cargo3202()),
    ),
};
fn test_cargo3202() {
    let ref r = req("0.*.*");
    assert_to_string(r, "0.*");
    assert_match_all(r, &["0.5.0"]);
    let ref r = req("0.0.*");
    assert_to_string(r, "0.0.*");
}
extern crate test;
#[rustc_test_marker = "test_digit_after_wildcard"]
#[doc(hidden)]
pub const test_digit_after_wildcard: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_digit_after_wildcard"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 423usize,
        start_col: 4usize,
        end_line: 423usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_digit_after_wildcard()),
    ),
};
fn test_digit_after_wildcard() {
    let err = req_err("*.1");
    assert_to_string(err, "unexpected character after wildcard in version req");
    let err = req_err("1.*.1");
    assert_to_string(err, "unexpected character after wildcard in version req");
    let err = req_err(">=1.*.1");
    assert_to_string(err, "unexpected character after wildcard in version req");
}
extern crate test;
#[rustc_test_marker = "test_eq_hash"]
#[doc(hidden)]
pub const test_eq_hash: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_eq_hash"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 435usize,
        start_col: 4usize,
        end_line: 435usize,
        end_col: 16usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_eq_hash()),
    ),
};
fn test_eq_hash() {
    fn calculate_hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    if !(req("^1") == req("^1")) {
        ::core::panicking::panic("assertion failed: req(\"^1\") == req(\"^1\")")
    }
    if !(calculate_hash(req("^1")) == calculate_hash(req("^1"))) {
        ::core::panicking::panic(
            "assertion failed: calculate_hash(req(\"^1\")) == calculate_hash(req(\"^1\"))",
        )
    }
    if !(req("^1") != req("^2")) {
        ::core::panicking::panic("assertion failed: req(\"^1\") != req(\"^2\")")
    }
}
extern crate test;
#[rustc_test_marker = "test_leading_digit_in_pre_and_build"]
#[doc(hidden)]
pub const test_leading_digit_in_pre_and_build: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_leading_digit_in_pre_and_build"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 448usize,
        start_col: 4usize,
        end_line: 448usize,
        end_col: 39usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_leading_digit_in_pre_and_build()),
    ),
};
fn test_leading_digit_in_pre_and_build() {
    for op in &["=", ">", ">=", "<", "<=", "~", "^"] {
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-1a", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3+1a", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-01a", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3+01", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-1+1", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-1-1+1-1-1", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-1a+1a", op))
            }),
        );
        req(
            &::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0} 1.2.3-1a-1a+1a-1a-1a", op))
            }),
        );
    }
}
extern crate test;
#[rustc_test_marker = "test_wildcard_and_another"]
#[doc(hidden)]
pub const test_wildcard_and_another: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_wildcard_and_another"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version_req.rs",
        start_line: 467usize,
        start_col: 4usize,
        end_line: 467usize,
        end_col: 29usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_wildcard_and_another()),
    ),
};
fn test_wildcard_and_another() {
    let err = req_err("*, 0.20.0-any");
    assert_to_string(
        err,
        "wildcard req (*) must be the only comparator in the version req",
    );
    let err = req_err("0.20.0-any, *");
    assert_to_string(
        err,
        "wildcard req (*) must be the only comparator in the version req",
    );
    let err = req_err("0.20.0-any, *, 1.0");
    assert_to_string(
        err,
        "wildcard req (*) must be the only comparator in the version req",
    );
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_any,
            &test_basic,
            &test_caret,
            &test_cargo3202,
            &test_comparator_parse,
            &test_default,
            &test_digit_after_wildcard,
            &test_eq_hash,
            &test_exact,
            &test_greater_than,
            &test_leading_digit_in_pre_and_build,
            &test_less_than,
            &test_logical_or,
            &test_multiple,
            &test_parse,
            &test_pre,
            &test_tilde,
            &test_whitespace_delimited_comparator_sets,
            &test_wildcard,
            &test_wildcard_and_another,
        ],
    )
}
