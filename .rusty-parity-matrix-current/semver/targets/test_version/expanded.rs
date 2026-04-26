#![feature(prelude_import)]
#![allow(clippy::nonminimal_bool, clippy::too_many_lines, clippy::wildcard_imports)]
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
use semver::{BuildMetadata, Prerelease, Version};
extern crate test;
#[rustc_test_marker = "test_parse"]
#[doc(hidden)]
pub const test_parse: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_parse"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 13usize,
        start_col: 4usize,
        end_line: 13usize,
        end_col: 14usize,
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
fn test_parse() {
    let err = version_err("");
    assert_to_string(err, "empty string, expected a semver version");
    let err = version_err("  ");
    assert_to_string(err, "unexpected character ' ' while parsing major version number");
    let err = version_err("1");
    assert_to_string(err, "unexpected end of input while parsing major version number");
    let err = version_err("1.2");
    assert_to_string(err, "unexpected end of input while parsing minor version number");
    let err = version_err("1.2.3-");
    assert_to_string(err, "empty identifier segment in pre-release identifier");
    let err = version_err("a.b.c");
    assert_to_string(err, "unexpected character 'a' while parsing major version number");
    let err = version_err("1.2.3 abc");
    assert_to_string(err, "unexpected character ' ' after patch version number");
    let err = version_err("1.2.3-01");
    assert_to_string(err, "invalid leading zero in pre-release identifier");
    let err = version_err("1.2.3++");
    assert_to_string(err, "empty identifier segment in build metadata");
    let err = version_err("07");
    assert_to_string(err, "invalid leading zero in major version number");
    let err = version_err("111111111111111111111.0.0");
    assert_to_string(err, "value of major version number exceeds u64::MAX");
    let err = version_err("8\0");
    assert_to_string(err, "unexpected character '\\0' after major version number");
    let parsed = version("1.2.3");
    let expected = Version::new(1, 2, 3);
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: Prerelease::EMPTY,
        build: BuildMetadata::EMPTY,
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3-alpha1");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: prerelease("alpha1"),
        build: BuildMetadata::EMPTY,
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3+build5");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: Prerelease::EMPTY,
        build: build_metadata("build5"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3+5build");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: Prerelease::EMPTY,
        build: build_metadata("5build"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3-alpha1+build5");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: prerelease("alpha1"),
        build: build_metadata("build5"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3-1.alpha1.9+build5.7.3aedf");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: prerelease("1.alpha1.9"),
        build: build_metadata("build5.7.3aedf"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.2.3-0a.alpha1.9+05build.7.3aedf");
    let expected = Version {
        major: 1,
        minor: 2,
        patch: 3,
        pre: prerelease("0a.alpha1.9"),
        build: build_metadata("05build.7.3aedf"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("0.4.0-beta.1+0851523");
    let expected = Version {
        major: 0,
        minor: 4,
        patch: 0,
        pre: prerelease("beta.1"),
        build: build_metadata("0851523"),
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    let parsed = version("1.1.0-beta-10");
    let expected = Version {
        major: 1,
        minor: 1,
        patch: 0,
        pre: prerelease("beta-10"),
        build: BuildMetadata::EMPTY,
    };
    match (&parsed, &expected) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
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
#[rustc_test_marker = "test_eq"]
#[doc(hidden)]
pub const test_eq: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_eq"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 157usize,
        start_col: 4usize,
        end_line: 157usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_eq())),
};
fn test_eq() {
    match (&version("1.2.3"), &version("1.2.3")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&version("1.2.3-alpha1"), &version("1.2.3-alpha1")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&version("1.2.3+build.42"), &version("1.2.3+build.42")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
    match (&version("1.2.3-alpha1+42"), &version("1.2.3-alpha1+42")) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
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
#[rustc_test_marker = "test_ne"]
#[doc(hidden)]
pub const test_ne: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_ne"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 165usize,
        start_col: 4usize,
        end_line: 165usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_ne())),
};
fn test_ne() {
    match (&version("0.0.0"), &version("0.0.1")) {
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
    match (&version("0.0.0"), &version("0.1.0")) {
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
    match (&version("0.0.0"), &version("1.0.0")) {
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
    match (&version("1.2.3-alpha"), &version("1.2.3-beta")) {
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
    match (&version("1.2.3+23"), &version("1.2.3+42")) {
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
#[rustc_test_marker = "test_display"]
#[doc(hidden)]
pub const test_display: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_display"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 174usize,
        start_col: 4usize,
        end_line: 174usize,
        end_col: 16usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_display()),
    ),
};
fn test_display() {
    assert_to_string(version("1.2.3"), "1.2.3");
    assert_to_string(version("1.2.3-alpha1"), "1.2.3-alpha1");
    assert_to_string(version("1.2.3+build.42"), "1.2.3+build.42");
    assert_to_string(version("1.2.3-alpha1+42"), "1.2.3-alpha1+42");
}
extern crate test;
#[rustc_test_marker = "test_lt"]
#[doc(hidden)]
pub const test_lt: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_lt"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 182usize,
        start_col: 4usize,
        end_line: 182usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_lt())),
};
fn test_lt() {
    if !(version("0.0.0") < version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"0.0.0\") < version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.0.0") < version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.0.0\") < version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.0") < version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.0\") < version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.3-alpha1") < version("1.2.3")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha1\") < version(\"1.2.3\")",
        )
    }
    if !(version("1.2.3-alpha1") < version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha1\") < version(\"1.2.3-alpha2\")",
        )
    }
    if !!(version("1.2.3-alpha2") < version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: !(version(\"1.2.3-alpha2\") < version(\"1.2.3-alpha2\"))",
        )
    }
    if !(version("1.2.3+23") < version("1.2.3+42")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3+23\") < version(\"1.2.3+42\")",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_le"]
#[doc(hidden)]
pub const test_le: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_le"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 193usize,
        start_col: 4usize,
        end_line: 193usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_le())),
};
fn test_le() {
    if !(version("0.0.0") <= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"0.0.0\") <= version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.0.0") <= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.0.0\") <= version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.0") <= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.0\") <= version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.3-alpha1") <= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha1\") <= version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.3-alpha2") <= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") <= version(\"1.2.3-alpha2\")",
        )
    }
    if !(version("1.2.3+23") <= version("1.2.3+42")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3+23\") <= version(\"1.2.3+42\")",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_gt"]
#[doc(hidden)]
pub const test_gt: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_gt"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 203usize,
        start_col: 4usize,
        end_line: 203usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_gt())),
};
fn test_gt() {
    if !(version("1.2.3-alpha2") > version("0.0.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") > version(\"0.0.0\")",
        )
    }
    if !(version("1.2.3-alpha2") > version("1.0.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") > version(\"1.0.0\")",
        )
    }
    if !(version("1.2.3-alpha2") > version("1.2.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") > version(\"1.2.0\")",
        )
    }
    if !(version("1.2.3-alpha2") > version("1.2.3-alpha1")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") > version(\"1.2.3-alpha1\")",
        )
    }
    if !(version("1.2.3") > version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3\") > version(\"1.2.3-alpha2\")",
        )
    }
    if !!(version("1.2.3-alpha2") > version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: !(version(\"1.2.3-alpha2\") > version(\"1.2.3-alpha2\"))",
        )
    }
    if !!(version("1.2.3+23") > version("1.2.3+42")) {
        ::core::panicking::panic(
            "assertion failed: !(version(\"1.2.3+23\") > version(\"1.2.3+42\"))",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_ge"]
#[doc(hidden)]
pub const test_ge: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_ge"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 214usize,
        start_col: 4usize,
        end_line: 214usize,
        end_col: 11usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(test_ge())),
};
fn test_ge() {
    if !(version("1.2.3-alpha2") >= version("0.0.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") >= version(\"0.0.0\")",
        )
    }
    if !(version("1.2.3-alpha2") >= version("1.0.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.0.0\")",
        )
    }
    if !(version("1.2.3-alpha2") >= version("1.2.0")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.0\")",
        )
    }
    if !(version("1.2.3-alpha2") >= version("1.2.3-alpha1")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.3-alpha1\")",
        )
    }
    if !(version("1.2.3-alpha2") >= version("1.2.3-alpha2")) {
        ::core::panicking::panic(
            "assertion failed: version(\"1.2.3-alpha2\") >= version(\"1.2.3-alpha2\")",
        )
    }
    if !!(version("1.2.3+23") >= version("1.2.3+42")) {
        ::core::panicking::panic(
            "assertion failed: !(version(\"1.2.3+23\") >= version(\"1.2.3+42\"))",
        )
    }
}
extern crate test;
#[rustc_test_marker = "test_spec_order"]
#[doc(hidden)]
pub const test_spec_order: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_spec_order"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 224usize,
        start_col: 4usize,
        end_line: 224usize,
        end_col: 19usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_spec_order()),
    ),
};
fn test_spec_order() {
    let vs = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ];
    let mut i = 1;
    while i < vs.len() {
        let a = version(vs[i - 1]);
        let b = version(vs[i]);
        if !(a < b) {
            {
                ::std::rt::panic_fmt(format_args!("nope {0:?} < {1:?}", a, b));
            }
        }
        i += 1;
    }
}
extern crate test;
#[rustc_test_marker = "test_align"]
#[doc(hidden)]
pub const test_align: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_align"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests/test_version.rs",
        start_line: 245usize,
        start_col: 4usize,
        end_line: 245usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_align()),
    ),
};
fn test_align() {
    let version = version("1.2.3-rc1");
    match (
        &"1.2.3-rc1           ",
        &::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("{0:20}", version))
        }),
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
    match (
        &"*****1.2.3-rc1******",
        &::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("{0:*^20}", version))
        }),
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
    match (
        &"           1.2.3-rc1",
        &::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("{0:>20}", version))
        }),
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
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_align,
            &test_display,
            &test_eq,
            &test_ge,
            &test_gt,
            &test_le,
            &test_lt,
            &test_ne,
            &test_parse,
            &test_spec_order,
        ],
    )
}
