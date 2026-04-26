#![feature(prelude_import)]
#![recursion_limit = "256"]
#![allow(clippy::dbg_macro)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
mod invalid {
    use snapbox::assert_data_eq;
    use snapbox::prelude::*;
    use snapbox::str;
    #[track_caller]
    fn t(toml: &str, expected: impl IntoData) {
        match toml {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "toml",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            7u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        match toml.parse::<crate::RustDocument>() {
            Ok(s) => {
                ::core::panicking::panic_fmt(format_args!("parsed to: {0:#?}", s));
            }
            Err(e) => {
                let actual = ::snapbox::IntoData::into_data(e.to_string());
                let expected = ::snapbox::IntoData::into_data(expected.raw());
                ::snapbox::Assert::new()
                    .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                    .eq(actual, expected);
            }
        }
    }
    extern crate test;
    #[rustc_test_marker = "invalid::basic_string_escape"]
    #[doc(hidden)]
    pub const basic_string_escape: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::basic_string_escape"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 15usize,
            start_col: 4usize,
            end_line: 15usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(basic_string_escape()),
        ),
    };
    fn basic_string_escape() {
        t(
            "a = \"\u{7f}\"",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 18u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 6
  |
1 | a = ""
  |      ^
invalid basic string, expected non-double-quote visible characters, `\`

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::literal_escape"]
    #[doc(hidden)]
    pub const literal_escape: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::literal_escape"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 30usize,
            start_col: 4usize,
            end_line: 30usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(literal_escape()),
        ),
    };
    fn literal_escape() {
        t(
            "a = '\u{7f}'",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 33u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 6
  |
1 | a = ''
  |      ^
invalid literal string, expected non-single-quote visible characters

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::stray_cr"]
    #[doc(hidden)]
    pub const stray_cr: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::stray_cr"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 45usize,
            start_col: 4usize,
            end_line: 45usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(stray_cr()),
        ),
    };
    fn stray_cr() {
        t(
            "\r",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 48u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 2
  |
1 | 
  |  ^
carriage return must be followed by newline, expected newline

"#,
                };
                inline
            },
        );
        t(
            "a = [ \r ]",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 59u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 8
  |
1 | a = [ 
 ]
  |        ^
carriage return must be followed by newline, expected newline

"#,
                };
                inline
            },
        );
        t(
            "a = \"\"\"\r\"\"\"",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 71u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 9
  |
1 | a = """
"""
  |         ^
carriage return must be followed by newline, expected newline

"#,
                };
                inline
            },
        );
        t(
            "a = \"\"\"\\  \r  \"\"\"",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 83u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 12
  |
1 | a = """\  
  """
  |            ^
carriage return must be followed by newline, expected newline

"#,
                };
                inline
            },
        );
        t(
            "a = '''\r'''",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 95u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 9
  |
1 | a = '''
'''
  |         ^
carriage return must be followed by newline, expected newline

"#,
                };
                inline
            },
        );
        t(
            "a = '\r'",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 107u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 6
  |
1 | a = '
'
  |      ^
invalid literal string, expected non-single-quote visible characters

"#,
                };
                inline
            },
        );
        t(
            "a = \"\r\"",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 119u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 6
  |
1 | a = "
"
  |      ^
invalid basic string, expected non-double-quote visible characters, `\`

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::duplicate_key_with_crlf"]
    #[doc(hidden)]
    pub const duplicate_key_with_crlf: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::duplicate_key_with_crlf"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 132usize,
            start_col: 4usize,
            end_line: 132usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(duplicate_key_with_crlf()),
        ),
    };
    fn duplicate_key_with_crlf() {
        t(
            "\r\n\
         [t1]\r\n\
         [t2]\r\n\
         a = 1\r\n\
         a = 2\r\n\
         ",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 140u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 5, column 1
  |
5 | a = 2
  | ^
duplicate key

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::inline_table_missing_key"]
    #[doc(hidden)]
    pub const inline_table_missing_key: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::inline_table_missing_key"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 152usize,
            start_col: 4usize,
            end_line: 152usize,
            end_col: 28usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(inline_table_missing_key()),
        ),
    };
    fn inline_table_missing_key() {
        t(
            "={[]\r].",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 155u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 3
  |
1 | ={[]
].
  |   ^
missing key for inline table element, expected key

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::inline_table_missing_key_in_array"]
    #[doc(hidden)]
    pub const inline_table_missing_key_in_array: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::inline_table_missing_key_in_array"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 168usize,
            start_col: 4usize,
            end_line: 168usize,
            end_col: 37usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(inline_table_missing_key_in_array()),
        ),
    };
    fn inline_table_missing_key_in_array() {
        t(
            "a=[{[]-]{\na.",
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/invalid.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 171u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 5
  |
1 | a=[{[]-]{
  |     ^
missing key for inline table element, expected key

"#,
                };
                inline
            },
        );
    }
    extern crate test;
    #[rustc_test_marker = "invalid::emoji_error_span"]
    #[doc(hidden)]
    pub const emoji_error_span: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::emoji_error_span"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 183usize,
            start_col: 4usize,
            end_line: 183usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(emoji_error_span()),
        ),
    };
    fn emoji_error_span() {
        let input = "key = 😀";
        match input {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "input",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            185u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let err = input.parse::<crate::RustDocument>().unwrap_err();
        match &err {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "&err",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            187u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let actual = &input[err.span().unwrap()];
        match (&actual, &"😀") {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "invalid::text_error_span"]
    #[doc(hidden)]
    pub const text_error_span: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::text_error_span"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 193usize,
            start_col: 4usize,
            end_line: 193usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(text_error_span()),
        ),
    };
    fn text_error_span() {
        let input = "key = asdf";
        match input {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "input",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            195u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let err = input.parse::<crate::RustDocument>().unwrap_err();
        match &err {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "&err",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            197u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let actual = &input[err.span().unwrap()];
        match (&actual, &"asdf") {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "invalid::fuzzed_68144_error_span"]
    #[doc(hidden)]
    pub const fuzzed_68144_error_span: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("invalid::fuzzed_68144_error_span"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/invalid.rs",
            start_line: 203usize,
            start_col: 4usize,
            end_line: 203usize,
            end_col: 27usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(fuzzed_68144_error_span()),
        ),
    };
    fn fuzzed_68144_error_span() {
        let input = "key = \"\\ᾂr\"";
        match input {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "input",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            205u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let err = input.parse::<crate::RustDocument>().unwrap_err();
        match &err {
            tmp => {
                {
                    ::std::io::_eprint(
                        format_args!(
                            "[{2}:{3}:{4}] {0} = {1:#?}\n",
                            "&err",
                            &&tmp as &dyn ::std::fmt::Debug,
                            "crates/toml/tests/compliance/invalid.rs",
                            207u32,
                            5u32,
                        ),
                    );
                };
                (tmp)
            }
        };
        let actual = &input[err.span().unwrap()];
        match (&actual, &"") {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
mod parse {
    use snapbox::assert_data_eq;
    use snapbox::prelude::*;
    use snapbox::str;
    extern crate test;
    #[rustc_test_marker = "parse::test_value_from_str"]
    #[doc(hidden)]
    pub const test_value_from_str: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::test_value_from_str"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 6usize,
            start_col: 4usize,
            end_line: 6usize,
            end_col: 23usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_value_from_str()),
        ),
    };
    fn test_value_from_str() {
        if !{
            let v = "1979-05-27T00:32:00.999999-07:00".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1979-05-27T00:32:00.999999-07:00",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1979-05-27T00:32:00.999999-07:00\").is_datetime()",
            )
        }
        if !{
            let v = "1979-05-27T00:32:00.999999Z".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1979-05-27T00:32:00.999999Z",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1979-05-27T00:32:00.999999Z\").is_datetime()",
            )
        }
        if !{
            let v = "1979-05-27T00:32:00.999999".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1979-05-27T00:32:00.999999",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1979-05-27T00:32:00.999999\").is_datetime()",
            )
        }
        if !{
            let v = "1979-05-27T00:32:00".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1979-05-27T00:32:00",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1979-05-27T00:32:00\").is_datetime()",
            )
        }
        if !{
            let v = "1979-05-27".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1979-05-27",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1979-05-27\").is_datetime()",
            )
        }
        if !{
            let v = "00:32:00".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "00:32:00",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_datetime()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"00:32:00\").is_datetime()",
            )
        }
        if !{
            let v = "-239".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "-239",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_integer()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"-239\").is_integer()",
            )
        }
        if !{
            let v = "1e200".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "1e200",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_float()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"1e200\").is_float()",
            )
        }
        if !{
            let v = "9_224_617.445_991_228_313".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            "9_224_617.445_991_228_313",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_float()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(\"9_224_617.445_991_228_313\").is_float()",
            )
        }
        if !{
            let v = r#""basic string\nJos\u00E9\n""#.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r#""basic string\nJos\u00E9\n""#,
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_str()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r#\"\"basic string\\nJos\\u00E9\\n\"\"#).is_str()",
            )
        }
        if !{
            let v = r#""""
multiline basic string
""""#.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r#""""
multiline basic string
""""#,
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_str()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r#\"\"\"\"\nmultiline basic string\n\"\"\"\"#).is_str()",
            )
        }
        if !{
            let v = r"'literal string\ \'".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r"'literal string\ \'",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_str()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r\"\'literal string\\ \\\'\").is_str()",
            )
        }
        if !{
            let v = r"'''multiline
literal \ \
string'''".parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r"'''multiline
literal \ \
string'''",
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_str()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r\"\'\'\'multiline\nliteral \\ \\\nstring\'\'\'\").is_str()",
            )
        }
        if !{
            let v = r#"{ hello = "world", a = 1}"#.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r#"{ hello = "world", a = 1}"#,
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_table()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r#\"{ hello = \"world\", a = 1}\"#).is_table()",
            )
        }
        if !{
            let v = r#"[ { x = 1, a = "2" }, {a = "a",b = "b",     c =    "c"} ]"#
                .parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r#"[ { x = 1, a = "2" }, {a = "a",b = "b",     c =    "c"} ]"#,
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_array()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r#\"[ { x = 1, a = \"2\" }, {a = \"a\",b = \"b\",     c =    \"c\"} ]\"#).is_array()",
            )
        }
        let wp = "C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9";
        let lwp = "'C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9'";
        match (
            &crate::RustValue::from(wp).as_str(),
            &{
                let v = lwp.parse::<toml::Value>();
                if !v.is_ok() {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                                v.unwrap_err(),
                                lwp,
                            ),
                        );
                    }
                }
                v.unwrap()
            }
                .as_str(),
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
        if !{
            let v = r#""\\\"\b\f\n\r\t\u00E9\U000A0000""#.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            r#""\\\"\b\f\n\r\t\u00E9\U000A0000""#,
                        ),
                    );
                }
            }
            v.unwrap()
        }
            .is_str()
        {
            ::core::panicking::panic(
                "assertion failed: parse_value!(r#\"\"\\\\\\\"\\b\\f\\n\\r\\t\\u00E9\\U000A0000\"\"#).is_str()",
            )
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::crlf"]
    #[doc(hidden)]
    pub const crlf: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::crlf"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 70usize,
            start_col: 4usize,
            end_line: 70usize,
            end_col: 8usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(crlf())),
    };
    fn crlf() {
        "\
     [project]\r\n\
     \r\n\
     name = \"splay\"\r\n\
     version = \"0.1.0\"\r\n\
     authors = [\"alex@crichton.co\"]\r\n\
     \r\n\
     [[lib]]\r\n\
     \r\n\
     path = \"lib.rs\"\r\n\
     name = \"splay\"\r\n\
     description = \"\"\"\
     A Rust implementation of a TAR file reader and writer. This library does not\r\n\
     currently handle compression, but it is abstract over all I/O readers and\r\n\
     writers. Additionally, great lengths are taken to ensure that the entire\r\n\
     contents are never required to be entirely resident in memory all at once.\r\n\
     \"\"\"\
     "
            .parse::<crate::RustDocument>()
            .unwrap();
    }
    extern crate test;
    #[rustc_test_marker = "parse::fun_with_strings"]
    #[doc(hidden)]
    pub const fun_with_strings: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::fun_with_strings"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 94usize,
            start_col: 4usize,
            end_line: 94usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(fun_with_strings()),
        ),
    };
    fn fun_with_strings() {
        let table = r#"
bar = "\U00000000"
key1 = "One\nTwo"
key2 = """One\nTwo"""
key3 = """
One
Two"""

key4 = "The quick brown fox jumps over the lazy dog."
key5 = """
The quick brown \


fox jumps over \
the lazy dog."""
key6 = """\
   The quick brown \
   fox jumps over \
   the lazy dog.\
   """
# What you see is what you get.
winpath  = 'C:\Users\nodejs\templates'
winpath2 = '\\ServerX\admin$\system32\'
quoted   = 'Tom "Dubs" Preston-Werner'
regex    = '<\i\c*\s*>'

regex2 = '''I [dw]on't need \d{2} apples'''
lines  = '''
The first newline is
trimmed in raw strings.
All other whitespace
is preserved.
'''
"#
            .parse::<crate::RustDocument>()
            .unwrap();
        match (&table["bar"].as_str(), &Some("\0")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["key1"].as_str(), &Some("One\nTwo")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["key2"].as_str(), &Some("One\nTwo")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["key3"].as_str(), &Some("One\nTwo")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        let msg = "The quick brown fox jumps over the lazy dog.";
        match (&table["key4"].as_str(), &Some(msg)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["key5"].as_str(), &Some(msg)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["key6"].as_str(), &Some(msg)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["winpath"].as_str(), &Some(r"C:\Users\nodejs\templates")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["winpath2"].as_str(), &Some(r"\\ServerX\admin$\system32\")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["quoted"].as_str(), &Some(r#"Tom "Dubs" Preston-Werner"#)) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["regex"].as_str(), &Some(r"<\i\c*\s*>")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["regex2"].as_str(), &Some(r"I [dw]on't need \d{2} apples")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
            &table["lines"].as_str(),
            &Some(
                "The first newline is\n\
             trimmed in raw strings.\n\
             All other whitespace\n\
             is preserved.\n",
            ),
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
    #[rustc_test_marker = "parse::tables_in_arrays"]
    #[doc(hidden)]
    pub const tables_in_arrays: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::tables_in_arrays"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 170usize,
            start_col: 4usize,
            end_line: 170usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(tables_in_arrays()),
        ),
    };
    fn tables_in_arrays() {
        let table = r#"
[[foo]]
#…
[foo.bar]
#…

[[foo]] # ...
#…
[foo.bar]
#...
"#
            .parse::<crate::RustDocument>()
            .unwrap();
        table["foo"][0]["bar"].as_table().unwrap();
        table["foo"][1]["bar"].as_table().unwrap();
    }
    extern crate test;
    #[rustc_test_marker = "parse::empty_table"]
    #[doc(hidden)]
    pub const empty_table: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::empty_table"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 189usize,
            start_col: 4usize,
            end_line: 189usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(empty_table()),
        ),
    };
    fn empty_table() {
        let table = r#"
[foo]"#.parse::<crate::RustDocument>().unwrap();
        table["foo"].as_table().unwrap();
    }
    extern crate test;
    #[rustc_test_marker = "parse::mixed_table_issue_527"]
    #[doc(hidden)]
    pub const mixed_table_issue_527: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::mixed_table_issue_527"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 198usize,
            start_col: 4usize,
            end_line: 198usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(mixed_table_issue_527()),
        ),
    };
    fn mixed_table_issue_527() {
        let input = r#"
[package]
metadata.msrv = "1.65.0"

[package.metadata.release.pre-release-replacements]
"#;
        let expected = {
            let position = ::snapbox::data::Position {
                file: {
                    let root = {
                        if let Some(rustc_root) = ::core::option::Option::None::<
                            &'static str,
                        > {
                            ::std::path::Path::new(rustc_root)
                        } else {
                            let manifest_dir = ::std::path::Path::new(
                                "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                            );
                            manifest_dir
                                .ancestors()
                                .filter(|it| it.join("Cargo.toml").exists())
                                .last()
                                .unwrap()
                        }
                    };
                    let file = "crates/toml/tests/compliance/parse.rs";
                    let rel_path = ::std::path::Path::new(file);
                    root.join(rel_path)
                },
                line: 205u32,
                column: 20u32,
            };
            let inline = ::snapbox::data::Inline {
                position,
                data: r#"
[package.metadata]
msrv = "1.65.0"

[package.metadata.release.pre-release-replacements]

"#,
            };
            inline
        };
        let document = input.parse::<crate::RustDocument>().unwrap();
        let actual = document.to_string();
        {
            let actual = ::snapbox::IntoData::into_data(actual);
            let expected = ::snapbox::IntoData::into_data(expected.raw());
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::fruit"]
    #[doc(hidden)]
    pub const fruit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::fruit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 218usize,
            start_col: 4usize,
            end_line: 218usize,
            end_col: 9usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(fruit())),
    };
    fn fruit() {
        let table = r#"
[[fruit]]
name = "apple"

[fruit.physical]
color = "red"
shape = "round"

[[fruit.variety]]
name = "red delicious"

[[fruit.variety]]
name = "granny smith"

[[fruit]]
name = "banana"

[[fruit.variety]]
name = "plantain"
"#
            .parse::<crate::RustDocument>()
            .unwrap();
        match (&table["fruit"][0]["name"].as_str(), &Some("apple")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["fruit"][0]["physical"]["color"].as_str(), &Some("red")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["fruit"][0]["physical"]["shape"].as_str(), &Some("round")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
            &table["fruit"][0]["variety"][0]["name"].as_str(),
            &Some("red delicious"),
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
            &table["fruit"][0]["variety"][1]["name"].as_str(),
            &Some("granny smith"),
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
        match (&table["fruit"][1]["name"].as_str(), &Some("banana")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["fruit"][1]["variety"][0]["name"].as_str(), &Some("plantain")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "parse::blank_literal_string"]
    #[doc(hidden)]
    pub const blank_literal_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::blank_literal_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 263usize,
            start_col: 4usize,
            end_line: 263usize,
            end_col: 24usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(blank_literal_string()),
        ),
    };
    fn blank_literal_string() {
        let table = "foo = ''".parse::<crate::RustDocument>().unwrap();
        match (&table["foo"].as_str(), &Some("")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "parse::many_blank"]
    #[doc(hidden)]
    pub const many_blank: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::many_blank"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 269usize,
            start_col: 4usize,
            end_line: 269usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(many_blank()),
        ),
    };
    fn many_blank() {
        let table = "foo = \"\"\"\n\n\n\"\"\"".parse::<crate::RustDocument>().unwrap();
        match (&table["foo"].as_str(), &Some("\n\n")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "parse::literal_eats_crlf"]
    #[doc(hidden)]
    pub const literal_eats_crlf: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::literal_eats_crlf"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 277usize,
            start_col: 4usize,
            end_line: 277usize,
            end_col: 21usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(literal_eats_crlf()),
        ),
    };
    fn literal_eats_crlf() {
        let table = "
        foo = \"\"\"\\\r\n\"\"\"
        bar = \"\"\"\\\r\n   \r\n   \r\n   a\"\"\"
    "
            .parse::<crate::RustDocument>()
            .unwrap();
        match (&table["foo"].as_str(), &Some("")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::None,
                    );
                }
            }
        };
        match (&table["bar"].as_str(), &Some("a")) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "parse::floats"]
    #[doc(hidden)]
    pub const floats: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::floats"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 289usize,
            start_col: 4usize,
            end_line: 289usize,
            end_col: 10usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(floats()),
        ),
    };
    fn floats() {
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1.0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1.0e0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1.0e+0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1.0e-0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1E-0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1.001e-0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &1.001) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2e10"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &2e10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2e+10"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &2e10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2e-10"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &2e-10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2_0.0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &20.0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2_0.0_0e1_0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &20.0e10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2_0.1_0e1_0"))
            });
            {
                ::std::io::_print(format_args!("{0}\n", f));
            };
            let a = f.parse::<crate::RustDocument>().unwrap();
            match (&a["foo"].as_float().unwrap(), &20.1e10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::bare_key_names"]
    #[doc(hidden)]
    pub const bare_key_names: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::bare_key_names"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 314usize,
            start_col: 4usize,
            end_line: 314usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(bare_key_names()),
        ),
    };
    fn bare_key_names() {
        let a = "
        foo = 3
        foo_3 = 3
        foo_-2--3--r23f--4-f2-4 = 3
        _ = 3
        - = 3
        8 = 8
        \"a\" = 3
        \"!\" = 3
        \"a^b\" = 3
        \"\\\"\" = 3
        \"character encoding\" = \"value\"
        'ʎǝʞ' = \"value\"
    "
            .parse::<crate::RustDocument>()
            .unwrap();
        let _ = &a["foo"];
        let _ = &a["-"];
        let _ = &a["_"];
        let _ = &a["8"];
        let _ = &a["foo_3"];
        let _ = &a["foo_-2--3--r23f--4-f2-4"];
        let _ = &a["a"];
        let _ = &a["!"];
        let _ = &a["\""];
        let _ = &a["character encoding"];
        let _ = &a["ʎǝʞ"];
    }
    extern crate test;
    #[rustc_test_marker = "parse::table_names"]
    #[doc(hidden)]
    pub const table_names: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::table_names"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 345usize,
            start_col: 4usize,
            end_line: 345usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(table_names()),
        ),
    };
    fn table_names() {
        let a = "
        [a.\"b\"]
        [\"f f\"]
        [\"f.f\"]
        [\"\\\"\"]
        ['a.a']
        ['\"\"']
    "
            .parse::<crate::RustDocument>()
            .unwrap();
        {
            ::std::io::_print(format_args!("{0:?}\n", a));
        };
        let _ = &a["a"]["b"];
        let _ = &a["f f"];
        let _ = &a["f.f"];
        let _ = &a["\""];
        let _ = &a["\"\""];
    }
    extern crate test;
    #[rustc_test_marker = "parse::inline_tables"]
    #[doc(hidden)]
    pub const inline_tables: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::inline_tables"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 365usize,
            start_col: 4usize,
            end_line: 365usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(inline_tables()),
        ),
    };
    fn inline_tables() {
        "a = {}".parse::<crate::RustDocument>().unwrap();
        "a = {b=1}".parse::<crate::RustDocument>().unwrap();
        "a = {   b   =   1    }".parse::<crate::RustDocument>().unwrap();
        "a = {a=1,b=2}".parse::<crate::RustDocument>().unwrap();
        "a = {a=1,b=2,c={}}".parse::<crate::RustDocument>().unwrap();
        "a = {a=[\n]}".parse::<crate::RustDocument>().unwrap();
        "a = {\"a\"=[\n]}".parse::<crate::RustDocument>().unwrap();
        "a = [\n{},\n{},\n]".parse::<crate::RustDocument>().unwrap();
    }
    extern crate test;
    #[rustc_test_marker = "parse::number_underscores"]
    #[doc(hidden)]
    pub const number_underscores: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::number_underscores"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 379usize,
            start_col: 4usize,
            end_line: 379usize,
            end_col: 22usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(number_underscores()),
        ),
    };
    fn number_underscores() {
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1_0"))
            });
            let table = f.parse::<crate::RustDocument>().unwrap();
            match (&table["foo"].as_integer().unwrap(), &10) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1_0_0"))
            });
            let table = f.parse::<crate::RustDocument>().unwrap();
            match (&table["foo"].as_integer().unwrap(), &100) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "1_000"))
            });
            let table = f.parse::<crate::RustDocument>().unwrap();
            match (&table["foo"].as_integer().unwrap(), &1000) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "+1_000"))
            });
            let table = f.parse::<crate::RustDocument>().unwrap();
            match (&table["foo"].as_integer().unwrap(), &1000) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "-1_000"))
            });
            let table = f.parse::<crate::RustDocument>().unwrap();
            match (&table["foo"].as_integer().unwrap(), &-1000) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::None,
                        );
                    }
                }
            };
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::empty_string"]
    #[doc(hidden)]
    pub const empty_string: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::empty_string"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 396usize,
            start_col: 4usize,
            end_line: 396usize,
            end_col: 16usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(empty_string()),
        ),
    };
    fn empty_string() {
        match (
            &"foo = \"\""
                .parse::<crate::RustDocument>()
                .unwrap()["foo"]
                .as_str()
                .unwrap(),
            &"",
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
    #[rustc_test_marker = "parse::datetimes"]
    #[doc(hidden)]
    pub const datetimes: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::datetimes"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 406usize,
            start_col: 4usize,
            end_line: 406usize,
            end_col: 13usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(datetimes()),
        ),
    };
    fn datetimes() {
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2016-09-09T09:09:09Z"))
            });
            let toml = f
                .parse::<crate::RustDocument>()
                .expect(
                    &::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("failed: {0}", f))
                    }),
                );
            match (
                &toml["foo"].as_datetime().unwrap().to_string(),
                &"2016-09-09T09:09:09Z",
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
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("foo = {0}", "2016-09-09T09:09:09.1Z"))
            });
            let toml = f
                .parse::<crate::RustDocument>()
                .expect(
                    &::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("failed: {0}", f))
                    }),
                );
            match (
                &toml["foo"].as_datetime().unwrap().to_string(),
                &"2016-09-09T09:09:09.1Z",
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
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!("foo = {0}", "2016-09-09T09:09:09.2+10:00"),
                )
            });
            let toml = f
                .parse::<crate::RustDocument>()
                .expect(
                    &::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("failed: {0}", f))
                    }),
                );
            match (
                &toml["foo"].as_datetime().unwrap().to_string(),
                &"2016-09-09T09:09:09.2+10:00",
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
        };
        {
            let f = ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!("foo = {0}", "2016-09-09T09:09:09.123456789-02:00"),
                )
            });
            let toml = f
                .parse::<crate::RustDocument>()
                .expect(
                    &::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("failed: {0}", f))
                    }),
                );
            match (
                &toml["foo"].as_datetime().unwrap().to_string(),
                &"2016-09-09T09:09:09.123456789-02:00",
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
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::dont_use_dotted_key_prefix_on_table_fuzz_57049"]
    #[doc(hidden)]
    pub const dont_use_dotted_key_prefix_on_table_fuzz_57049: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName(
                "parse::dont_use_dotted_key_prefix_on_table_fuzz_57049",
            ),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 424usize,
            start_col: 4usize,
            end_line: 424usize,
            end_col: 50usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(dont_use_dotted_key_prefix_on_table_fuzz_57049()),
        ),
    };
    fn dont_use_dotted_key_prefix_on_table_fuzz_57049() {
        let input = r#"
p.a=4
[p.o]
"#;
        let expected = {
            let position = ::snapbox::data::Position {
                file: {
                    let root = {
                        if let Some(rustc_root) = ::core::option::Option::None::<
                            &'static str,
                        > {
                            ::std::path::Path::new(rustc_root)
                        } else {
                            let manifest_dir = ::std::path::Path::new(
                                "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                            );
                            manifest_dir
                                .ancestors()
                                .filter(|it| it.join("Cargo.toml").exists())
                                .last()
                                .unwrap()
                        }
                    };
                    let file = "crates/toml/tests/compliance/parse.rs";
                    let rel_path = ::std::path::Path::new(file);
                    root.join(rel_path)
                },
                line: 434u32,
                column: 20u32,
            };
            let inline = ::snapbox::data::Inline {
                position,
                data: r#"
[p]
a = 4

[p.o]

"#,
            };
            inline
        };
        let document = input.parse::<crate::RustDocument>().unwrap();
        let actual = document.to_string();
        {
            let actual = ::snapbox::IntoData::into_data(actual);
            let expected = ::snapbox::IntoData::into_data(expected.raw());
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::string_repr_roundtrip"]
    #[doc(hidden)]
    pub const string_repr_roundtrip: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::string_repr_roundtrip"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 478usize,
            start_col: 4usize,
            end_line: 478usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(string_repr_roundtrip()),
        ),
    };
    fn string_repr_roundtrip() {
        assert_string_repr_roundtrip(
            r#""""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 479u32,
                    column: 43u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""a""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 480u32,
                    column: 44u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""a""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""tab \t tab""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 482u32,
                    column: 53u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""tab \t tab""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""lf \n lf""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 485u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
"""
lf 
 lf"""
"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""crlf \r\n crlf""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 493u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
"""
crlf \r
 crlf"""
"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""bell \b bell""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 499u32,
                    column: 55u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""bell \b bell""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""feed \f feed""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 500u32,
                    column: 55u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""feed \f feed""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""backslash \\ backslash""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 503u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'backslash \ backslash'"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""squote ' squote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 506u32,
                    column: 58u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""squote ' squote""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""triple squote ''' triple squote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 509u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""triple squote ''' triple squote""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""end squote '""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 511u32,
                    column: 55u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""end squote '""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""quote \" quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 513u32,
                    column: 57u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'quote " quote'"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""triple quote \"\"\" triple quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 516u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'triple quote """ triple quote'"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""end quote \"""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 518u32,
                    column: 55u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'end quote "'"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""quoted \"content\" quoted""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 521u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'quoted "content" quoted'"#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""squoted 'content' squoted""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 525u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""squoted 'content' squoted""#,
                };
                inline
            },
        );
        assert_string_repr_roundtrip(
            r#""mixed quoted \"start\" 'end'' mixed quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 529u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""""mixed quoted "start" 'end'' mixed quote""""#,
                };
                inline
            },
        );
    }
    #[track_caller]
    fn assert_string_repr_roundtrip(input: &str, expected: impl IntoData) {
        let value = {
            let v = input.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            input,
                        ),
                    );
                }
            }
            v.unwrap()
        };
        let actual = value.to_string();
        let _ = {
            let v = (&actual).parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            &actual,
                        ),
                    );
                }
            }
            v.unwrap()
        };
        {
            let actual = ::snapbox::IntoData::into_data(actual);
            let expected = ::snapbox::IntoData::into_data(expected.raw());
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::string_value_roundtrip"]
    #[doc(hidden)]
    pub const string_value_roundtrip: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::string_value_roundtrip"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 542usize,
            start_col: 4usize,
            end_line: 542usize,
            end_col: 26usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(string_value_roundtrip()),
        ),
    };
    fn string_value_roundtrip() {
        assert_string_value_roundtrip(
            r#""""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 543u32,
                    column: 44u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""a""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 544u32,
                    column: 45u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""a""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""tab \t tab""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 546u32,
                    column: 54u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""tab \t tab""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""lf \n lf""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 549u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
"""
lf 
 lf"""
"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""crlf \r\n crlf""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 557u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
"""
crlf \r
 crlf"""
"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""bell \b bell""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 563u32,
                    column: 56u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""bell \b bell""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""feed \f feed""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 564u32,
                    column: 56u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""feed \f feed""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""backslash \\ backslash""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 567u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'backslash \ backslash'"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""squote ' squote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 570u32,
                    column: 59u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""squote ' squote""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""triple squote ''' triple squote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 573u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""triple squote ''' triple squote""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""end squote '""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 575u32,
                    column: 56u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""end squote '""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""quote \" quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 577u32,
                    column: 58u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'quote " quote'"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""triple quote \"\"\" triple quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 580u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'triple quote """ triple quote'"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""end quote \"""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 582u32,
                    column: 56u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'end quote "'"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""quoted \"content\" quoted""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 585u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"'quoted "content" quoted'"#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""squoted 'content' squoted""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 589u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""squoted 'content' squoted""#,
                };
                inline
            },
        );
        assert_string_value_roundtrip(
            r#""mixed quoted \"start\" 'end'' mixed quote""#,
            {
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 593u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#""""mixed quoted "start" 'end'' mixed quote""""#,
                };
                inline
            },
        );
    }
    #[track_caller]
    fn assert_string_value_roundtrip(input: &str, expected: impl IntoData) {
        let value = {
            let v = input.parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            input,
                        ),
                    );
                }
            }
            v.unwrap()
        };
        let value = crate::RustValue::from(value.as_str().unwrap());
        let actual = value.to_string();
        let _ = {
            let v = (&actual).parse::<toml::Value>();
            if !v.is_ok() {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "Failed with `{0}` when parsing:\n```\n{1}\n```\n",
                            v.unwrap_err(),
                            &actual,
                        ),
                    );
                }
            }
            v.unwrap()
        };
        {
            let actual = ::snapbox::IntoData::into_data(actual);
            let expected = ::snapbox::IntoData::into_data(expected.raw());
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::array_recursion_limit"]
    #[doc(hidden)]
    pub const array_recursion_limit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::array_recursion_limit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 608usize,
            start_col: 4usize,
            end_line: 608usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(array_recursion_limit()),
        ),
    };
    fn array_recursion_limit() {
        let depths = [(1, true), (20, true), (300, false)];
        for (depth, is_ok) in depths {
            let input = ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!("x={0}{1}", &"[".repeat(depth), &"]".repeat(depth)),
                )
            });
            let document = input.parse::<crate::RustDocument>();
            match (&document.is_ok(), &is_ok) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("depth: {0}", depth),
                            ),
                        );
                    }
                }
            };
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::inline_table_recursion_limit"]
    #[doc(hidden)]
    pub const inline_table_recursion_limit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::inline_table_recursion_limit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 619usize,
            start_col: 4usize,
            end_line: 619usize,
            end_col: 32usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(inline_table_recursion_limit()),
        ),
    };
    fn inline_table_recursion_limit() {
        let depths = [(1, true), (20, true), (300, false)];
        for (depth, is_ok) in depths {
            let input = ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "x={0}true{1}",
                        &"{ x = ".repeat(depth),
                        &"}".repeat(depth),
                    ),
                )
            });
            let document = input.parse::<crate::RustDocument>();
            match (&document.is_ok(), &is_ok) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("depth: {0}", depth),
                            ),
                        );
                    }
                }
            };
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::table_key_recursion_limit"]
    #[doc(hidden)]
    pub const table_key_recursion_limit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::table_key_recursion_limit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 630usize,
            start_col: 4usize,
            end_line: 630usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(table_key_recursion_limit()),
        ),
    };
    fn table_key_recursion_limit() {
        let depths = [(1, true), (20, true), (300, false)];
        for (depth, is_ok) in depths {
            let input = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("[x{0}]", &".x".repeat(depth)))
            });
            let document = input.parse::<crate::RustDocument>();
            match (&document.is_ok(), &is_ok) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("depth: {0}", depth),
                            ),
                        );
                    }
                }
            };
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::dotted_key_recursion_limit"]
    #[doc(hidden)]
    pub const dotted_key_recursion_limit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::dotted_key_recursion_limit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 641usize,
            start_col: 4usize,
            end_line: 641usize,
            end_col: 30usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(dotted_key_recursion_limit()),
        ),
    };
    fn dotted_key_recursion_limit() {
        let depths = [(1, true), (20, true), (300, false)];
        for (depth, is_ok) in depths {
            let input = ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("x{0} = true", &".x".repeat(depth)))
            });
            let document = input.parse::<crate::RustDocument>();
            match (&document.is_ok(), &is_ok) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("depth: {0}", depth),
                            ),
                        );
                    }
                }
            };
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::inline_dotted_key_recursion_limit"]
    #[doc(hidden)]
    pub const inline_dotted_key_recursion_limit: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::inline_dotted_key_recursion_limit"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 652usize,
            start_col: 4usize,
            end_line: 652usize,
            end_col: 37usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(inline_dotted_key_recursion_limit()),
        ),
    };
    fn inline_dotted_key_recursion_limit() {
        let depths = [(1, true), (20, true), (300, false)];
        for (depth, is_ok) in depths {
            let input = ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!("x = {{ x{0} = true }}", &".x".repeat(depth)),
                )
            });
            let document = input.parse::<crate::RustDocument>();
            match (&document.is_ok(), &is_ok) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("depth: {0}", depth),
                            ),
                        );
                    }
                }
            };
        }
    }
    extern crate test;
    #[rustc_test_marker = "parse::garbage1"]
    #[doc(hidden)]
    pub const garbage1: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::garbage1"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 662usize,
            start_col: 4usize,
            end_line: 662usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(garbage1()),
        ),
    };
    fn garbage1() {
        let err = "={=<=u==".parse::<crate::RustDocument>().unwrap_err();
        {
            let actual = ::snapbox::IntoData::into_data(err.to_string());
            let expected = ::snapbox::IntoData::into_data({
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 666u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 5
  |
1 | ={=<=u==
  |     ^
extra assignment between key-value pairs, expected `,`

"#,
                };
                inline
            });
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::garbage2"]
    #[doc(hidden)]
    pub const garbage2: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::garbage2"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 678usize,
            start_col: 4usize,
            end_line: 678usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(garbage2()),
        ),
    };
    fn garbage2() {
        let err = "={=<=u==}".parse::<crate::RustDocument>().unwrap_err();
        {
            let actual = ::snapbox::IntoData::into_data(err.to_string());
            let expected = ::snapbox::IntoData::into_data({
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 682u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 5
  |
1 | ={=<=u==}
  |     ^
extra assignment between key-value pairs, expected `,`

"#,
                };
                inline
            });
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "parse::garbage3"]
    #[doc(hidden)]
    pub const garbage3: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("parse::garbage3"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/compliance/parse.rs",
            start_line: 694usize,
            start_col: 4usize,
            end_line: 694usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(garbage3()),
        ),
    };
    fn garbage3() {
        let err = "==\n[._[._".parse::<crate::RustDocument>().unwrap_err();
        {
            let actual = ::snapbox::IntoData::into_data(err.to_string());
            let expected = ::snapbox::IntoData::into_data({
                let position = ::snapbox::data::Position {
                    file: {
                        let root = {
                            if let Some(rustc_root) = ::core::option::Option::None::<
                                &'static str,
                            > {
                                ::std::path::Path::new(rustc_root)
                            } else {
                                let manifest_dir = ::std::path::Path::new(
                                    "/home/shuai/git/rusty-cpp/tests/transpile_tests/toml/crates/toml",
                                );
                                manifest_dir
                                    .ancestors()
                                    .filter(|it| it.join("Cargo.toml").exists())
                                    .last()
                                    .unwrap()
                            }
                        };
                        let file = "crates/toml/tests/compliance/parse.rs";
                        let rel_path = ::std::path::Path::new(file);
                        root.join(rel_path)
                    },
                    line: 698u32,
                    column: 9u32,
                };
                let inline = ::snapbox::data::Inline {
                    position,
                    data: r#"
TOML parse error at line 1, column 2
  |
1 | ==
  |  ^
extra `=`, expected nothing

"#,
                };
                inline
            });
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
}
use toml::Table as RustDocument;
use toml::Value as RustValue;
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &basic_string_escape,
            &duplicate_key_with_crlf,
            &emoji_error_span,
            &fuzzed_68144_error_span,
            &inline_table_missing_key,
            &inline_table_missing_key_in_array,
            &literal_escape,
            &stray_cr,
            &text_error_span,
            &array_recursion_limit,
            &bare_key_names,
            &blank_literal_string,
            &crlf,
            &datetimes,
            &dont_use_dotted_key_prefix_on_table_fuzz_57049,
            &dotted_key_recursion_limit,
            &empty_string,
            &empty_table,
            &floats,
            &fruit,
            &fun_with_strings,
            &garbage1,
            &garbage2,
            &garbage3,
            &inline_dotted_key_recursion_limit,
            &inline_table_recursion_limit,
            &inline_tables,
            &literal_eats_crlf,
            &many_blank,
            &mixed_table_issue_527,
            &number_underscores,
            &string_repr_roundtrip,
            &string_value_roundtrip,
            &table_key_recursion_limit,
            &table_names,
            &tables_in_arrays,
            &test_value_from_str,
        ],
    )
}
