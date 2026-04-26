#![feature(prelude_import)]
#![recursion_limit = "256"]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
mod macros {
    use std::f64;
    use toml::toml;
    extern crate test;
    #[rustc_test_marker = "macros::test_cargo_toml"]
    #[doc(hidden)]
    pub const test_cargo_toml: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_cargo_toml"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 37usize,
            start_col: 4usize,
            end_line: 37usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_cargo_toml()),
        ),
    };
    fn test_cargo_toml() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-package"[1..]],
                ::toml::Value::Table(::toml::value::Table::new()),
            );
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-package"[1..], &"-name"[1..]],
                    {
                        let de = ::toml::macros::IntoDeserializer::<
                            ::toml::de::Error,
                        >::into_deserializer("toml");
                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(de)
                            .unwrap()
                    },
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-package"[1..], &"-version"[1..]],
                        {
                            let de = ::toml::macros::IntoDeserializer::<
                                ::toml::de::Error,
                            >::into_deserializer("0.4.5");
                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                    de,
                                )
                                .unwrap()
                        },
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-package"[1..], &"-authors"[1..]],
                            {
                                let mut array = ::toml::value::Array::new();
                                array
                                    .push({
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer(
                                            "Alex Crichton <alex@alexcrichton.com>",
                                        );
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    });
                                ::toml::Value::Array(array)
                            },
                        );
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-badges"[1..]],
                            ::toml::Value::Table(::toml::value::Table::new()),
                        );
                        {
                            ::toml::macros::insert_toml(
                                &mut root,
                                &[&"-badges"[1..], &"-travis-ci"[1..]],
                                {
                                    let mut table = ::toml::Value::Table(
                                        ::toml::value::Table::new(),
                                    );
                                    ::toml::macros::insert_toml(
                                        &mut table,
                                        &[&"-repository"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer("alexcrichton/toml-rs");
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    table
                                },
                            );
                            ::toml::macros::insert_toml(
                                &mut root,
                                &[&"-dependencies"[1..]],
                                ::toml::Value::Table(::toml::value::Table::new()),
                            );
                            {
                                ::toml::macros::insert_toml(
                                    &mut root,
                                    &[&"-dependencies"[1..], &"-serde"[1..]],
                                    {
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer("1.0");
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    },
                                );
                                ::toml::macros::insert_toml(
                                    &mut root,
                                    &[&"-dev-dependencies"[1..]],
                                    ::toml::Value::Table(::toml::value::Table::new()),
                                );
                                {
                                    ::toml::macros::insert_toml(
                                        &mut root,
                                        &[&"-dev-dependencies"[1..], &"-serde_derive"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer("1.0");
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    {
                                        ::toml::macros::insert_toml(
                                            &mut root,
                                            &[&"-dev-dependencies"[1..], &"-serde_json"[1..]],
                                            {
                                                let de = ::toml::macros::IntoDeserializer::<
                                                    ::toml::de::Error,
                                                >::into_deserializer("1.0");
                                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                        de,
                                                    )
                                                    .unwrap()
                                            },
                                        );
                                    };
                                };
                            };
                        };
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table
                .insert(
                    "package".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("name".to_owned(), "toml".to_owned().into());
                        table.insert("version".to_owned(), "0.4.5".to_owned().into());
                        table
                            .insert(
                                "authors".to_owned(),
                                {
                                    #![allow(clippy::vec_init_then_push)]
                                    #[allow(unused_mut)]
                                    let mut array = toml::value::Array::new();
                                    array
                                        .push(
                                            "Alex Crichton <alex@alexcrichton.com>".to_owned().into(),
                                        );
                                    toml::Value::Array(array)
                                }
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "badges".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table
                            .insert(
                                "travis-ci".to_owned(),
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table
                                        .insert(
                                            "repository".to_owned(),
                                            "alexcrichton/toml-rs".to_owned().into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "dependencies".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("serde".to_owned(), "1.0".to_owned().into());
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "dev-dependencies".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("serde_derive".to_owned(), "1.0".to_owned().into());
                        table.insert("serde_json".to_owned(), "1.0".to_owned().into());
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_array"]
    #[doc(hidden)]
    pub const test_array: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_array"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 89usize,
            start_col: 4usize,
            end_line: 89usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_array()),
        ),
    };
    fn test_array() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            ::toml::macros::push_toml(&mut root, &[&"-fruit"[1..]]);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-fruit"[1..], &"-name"[1..]],
                    {
                        let de = ::toml::macros::IntoDeserializer::<
                            ::toml::de::Error,
                        >::into_deserializer("apple");
                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(de)
                            .unwrap()
                    },
                );
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-fruit"[1..], &"-physical"[1..]],
                    ::toml::Value::Table(::toml::value::Table::new()),
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-fruit"[1..], &"-physical"[1..], &"-color"[1..]],
                        {
                            let de = ::toml::macros::IntoDeserializer::<
                                ::toml::de::Error,
                            >::into_deserializer("red");
                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                    de,
                                )
                                .unwrap()
                        },
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-fruit"[1..], &"-physical"[1..], &"-shape"[1..]],
                            {
                                let de = ::toml::macros::IntoDeserializer::<
                                    ::toml::de::Error,
                                >::into_deserializer("round");
                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                        de,
                                    )
                                    .unwrap()
                            },
                        );
                        ::toml::macros::push_toml(
                            &mut root,
                            &[&"-fruit"[1..], &"-variety"[1..]],
                        );
                        {
                            ::toml::macros::insert_toml(
                                &mut root,
                                &[&"-fruit"[1..], &"-variety"[1..], &"-name"[1..]],
                                {
                                    let de = ::toml::macros::IntoDeserializer::<
                                        ::toml::de::Error,
                                    >::into_deserializer("red delicious");
                                    <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                            de,
                                        )
                                        .unwrap()
                                },
                            );
                            ::toml::macros::push_toml(
                                &mut root,
                                &[&"-fruit"[1..], &"-variety"[1..]],
                            );
                            {
                                ::toml::macros::insert_toml(
                                    &mut root,
                                    &[&"-fruit"[1..], &"-variety"[1..], &"-name"[1..]],
                                    {
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer("granny smith");
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    },
                                );
                                ::toml::macros::push_toml(&mut root, &[&"-fruit"[1..]]);
                                {
                                    ::toml::macros::insert_toml(
                                        &mut root,
                                        &[&"-fruit"[1..], &"-name"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer("banana");
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    ::toml::macros::push_toml(
                                        &mut root,
                                        &[&"-fruit"[1..], &"-variety"[1..]],
                                    );
                                    {
                                        ::toml::macros::insert_toml(
                                            &mut root,
                                            &[&"-fruit"[1..], &"-variety"[1..], &"-name"[1..]],
                                            {
                                                let de = ::toml::macros::IntoDeserializer::<
                                                    ::toml::de::Error,
                                                >::into_deserializer("plantain");
                                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                        de,
                                                    )
                                                    .unwrap()
                                            },
                                        );
                                    };
                                };
                            };
                        };
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table
                .insert(
                    "fruit".to_owned(),
                    {
                        #![allow(clippy::vec_init_then_push)]
                        #[allow(unused_mut)]
                        let mut array = toml::value::Array::new();
                        array
                            .push(
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table.insert("name".to_owned(), "apple".into());
                                    table
                                        .insert(
                                            "physical".to_owned(),
                                            {
                                                #[allow(unused_mut)]
                                                let mut table = toml::value::Table::new();
                                                table.insert("color".to_owned(), "red".into());
                                                table.insert("shape".to_owned(), "round".into());
                                                toml::Value::Table(table)
                                            }
                                                .into(),
                                        );
                                    table
                                        .insert(
                                            "variety".to_owned(),
                                            {
                                                #![allow(clippy::vec_init_then_push)]
                                                #[allow(unused_mut)]
                                                let mut array = toml::value::Array::new();
                                                array
                                                    .push(
                                                        {
                                                            #[allow(unused_mut)]
                                                            let mut table = toml::value::Table::new();
                                                            table.insert("name".to_owned(), "red delicious".into());
                                                            toml::Value::Table(table)
                                                        }
                                                            .into(),
                                                    );
                                                array
                                                    .push(
                                                        {
                                                            #[allow(unused_mut)]
                                                            let mut table = toml::value::Table::new();
                                                            table.insert("name".to_owned(), "granny smith".into());
                                                            toml::Value::Table(table)
                                                        }
                                                            .into(),
                                                    );
                                                toml::Value::Array(array)
                                            }
                                                .into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        array
                            .push(
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table.insert("name".to_owned(), "banana".into());
                                    table
                                        .insert(
                                            "variety".to_owned(),
                                            {
                                                #![allow(clippy::vec_init_then_push)]
                                                #[allow(unused_mut)]
                                                let mut array = toml::value::Array::new();
                                                array
                                                    .push(
                                                        {
                                                            #[allow(unused_mut)]
                                                            let mut table = toml::value::Table::new();
                                                            table.insert("name".to_owned(), "plantain".into());
                                                            toml::Value::Table(table)
                                                        }
                                                            .into(),
                                                    );
                                                toml::Value::Array(array)
                                            }
                                                .into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        toml::Value::Array(array)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_number"]
    #[doc(hidden)]
    pub const test_number: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_number"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 144usize,
            start_col: 4usize,
            end_line: 144usize,
            end_col: 15usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_number()),
        ),
    };
    fn test_number() {
        #![allow(clippy::unusual_byte_groupings)]
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-positive"[1..]],
                    {
                        let de = ::toml::macros::IntoDeserializer::<
                            ::toml::de::Error,
                        >::into_deserializer(1);
                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(de)
                            .unwrap()
                    },
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-negative"[1..]],
                        {
                            let de = ::toml::macros::IntoDeserializer::<
                                ::toml::de::Error,
                            >::into_deserializer((-1));
                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                    de,
                                )
                                .unwrap()
                        },
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-table"[1..]],
                            {
                                let mut table = ::toml::Value::Table(
                                    ::toml::value::Table::new(),
                                );
                                ::toml::macros::insert_toml(
                                    &mut table,
                                    &[&"-positive"[1..]],
                                    {
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer(1);
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    },
                                );
                                ::toml::macros::insert_toml(
                                    &mut table,
                                    &[&"-negative"[1..]],
                                    {
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer((-1));
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    },
                                );
                                table
                            },
                        );
                        {
                            ::toml::macros::insert_toml(
                                &mut root,
                                &[&"-array"[1..]],
                                {
                                    let mut array = ::toml::value::Array::new();
                                    array
                                        .push({
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer(1);
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        });
                                    array
                                        .push({
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer((-1));
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        });
                                    ::toml::Value::Array(array)
                                },
                            );
                            {
                                ::toml::macros::insert_toml(
                                    &mut root,
                                    &[&"-neg_zero"[1..]],
                                    {
                                        let de = ::toml::macros::IntoDeserializer::<
                                            ::toml::de::Error,
                                        >::into_deserializer((-0));
                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                de,
                                            )
                                            .unwrap()
                                    },
                                );
                                {
                                    ::toml::macros::insert_toml(
                                        &mut root,
                                        &[&"-pos_zero"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer((0));
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    {
                                        ::toml::macros::insert_toml(
                                            &mut root,
                                            &[&"-float"[1..]],
                                            {
                                                let de = ::toml::macros::IntoDeserializer::<
                                                    ::toml::de::Error,
                                                >::into_deserializer(1.618);
                                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                        de,
                                                    )
                                                    .unwrap()
                                            },
                                        );
                                        {
                                            ::toml::macros::insert_toml(
                                                &mut root,
                                                &[&"-sf1"[1..]],
                                                ::toml::Value::Float(::core::f64::INFINITY),
                                            );
                                            {
                                                ::toml::macros::insert_toml(
                                                    &mut root,
                                                    &[&"-sf2"[1..]],
                                                    ::toml::Value::Float(::core::f64::INFINITY),
                                                );
                                                {
                                                    ::toml::macros::insert_toml(
                                                        &mut root,
                                                        &[&"-sf3"[1..]],
                                                        ::toml::Value::Float(::core::f64::NEG_INFINITY),
                                                    );
                                                    {
                                                        ::toml::macros::insert_toml(
                                                            &mut root,
                                                            &[&"-sf7"[1..]],
                                                            {
                                                                let de = ::toml::macros::IntoDeserializer::<
                                                                    ::toml::de::Error,
                                                                >::into_deserializer((0.0));
                                                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                                        de,
                                                                    )
                                                                    .unwrap()
                                                            },
                                                        );
                                                        {
                                                            ::toml::macros::insert_toml(
                                                                &mut root,
                                                                &[&"-sf8"[1..]],
                                                                {
                                                                    let de = ::toml::macros::IntoDeserializer::<
                                                                        ::toml::de::Error,
                                                                    >::into_deserializer((-0.0));
                                                                    <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                                            de,
                                                                        )
                                                                        .unwrap()
                                                                },
                                                            );
                                                            {
                                                                ::toml::macros::insert_toml(
                                                                    &mut root,
                                                                    &[&"-hex"[1..]],
                                                                    {
                                                                        let de = ::toml::macros::IntoDeserializer::<
                                                                            ::toml::de::Error,
                                                                        >::into_deserializer(0xa_b_c);
                                                                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                                                de,
                                                                            )
                                                                            .unwrap()
                                                                    },
                                                                );
                                                                {
                                                                    ::toml::macros::insert_toml(
                                                                        &mut root,
                                                                        &[&"-oct"[1..]],
                                                                        {
                                                                            let de = ::toml::macros::IntoDeserializer::<
                                                                                ::toml::de::Error,
                                                                            >::into_deserializer(0o755);
                                                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                                                    de,
                                                                                )
                                                                                .unwrap()
                                                                        },
                                                                    );
                                                                    {
                                                                        ::toml::macros::insert_toml(
                                                                            &mut root,
                                                                            &[&"-bin"[1..]],
                                                                            {
                                                                                let de = ::toml::macros::IntoDeserializer::<
                                                                                    ::toml::de::Error,
                                                                                >::into_deserializer(0b11010110);
                                                                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                                                        de,
                                                                                    )
                                                                                    .unwrap()
                                                                            },
                                                                        );
                                                                    };
                                                                };
                                                            };
                                                        };
                                                    };
                                                };
                                            };
                                        };
                                    };
                                };
                            };
                        };
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table.insert("positive".to_owned(), 1.into());
            table.insert("negative".to_owned(), (-1).into());
            table
                .insert(
                    "table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("positive".to_owned(), 1.into());
                        table.insert("negative".to_owned(), (-1).into());
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "array".to_owned(),
                    {
                        #![allow(clippy::vec_init_then_push)]
                        #[allow(unused_mut)]
                        let mut array = toml::value::Array::new();
                        array.push(1.into());
                        array.push((-1).into());
                        toml::Value::Array(array)
                    }
                        .into(),
                );
            table.insert("neg_zero".to_owned(), (-0).into());
            table.insert("pos_zero".to_owned(), 0.into());
            table.insert("float".to_owned(), 1.618.into());
            table.insert("sf1".to_owned(), f64::INFINITY.into());
            table.insert("sf2".to_owned(), f64::INFINITY.into());
            table.insert("sf3".to_owned(), f64::NEG_INFINITY.into());
            table.insert("sf7".to_owned(), 0.0.into());
            table.insert("sf8".to_owned(), (-0.0).into());
            table.insert("hex".to_owned(), 2748.into());
            table.insert("oct".to_owned(), 493.into());
            table.insert("bin".to_owned(), 214.into());
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_nan"]
    #[doc(hidden)]
    pub const test_nan: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_nan"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 195usize,
            start_col: 4usize,
            end_line: 195usize,
            end_col: 12usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_nan()),
        ),
    };
    fn test_nan() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-sf4"[1..]],
                    ::toml::Value::Float(::core::f64::NAN.copysign(1.0)),
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-sf5"[1..]],
                        ::toml::Value::Float(::core::f64::NAN.copysign(1.0)),
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-sf6"[1..]],
                            ::toml::Value::Float(::core::f64::NAN.copysign(-1.0)),
                        );
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let sf4 = actual["sf4"].as_float().unwrap();
        if !sf4.is_nan() {
            ::core::panicking::panic("assertion failed: sf4.is_nan()")
        }
        if !sf4.is_sign_positive() {
            ::core::panicking::panic("assertion failed: sf4.is_sign_positive()")
        }
        let sf5 = actual["sf5"].as_float().unwrap();
        if !sf5.is_nan() {
            ::core::panicking::panic("assertion failed: sf5.is_nan()")
        }
        if !sf5.is_sign_positive() {
            ::core::panicking::panic("assertion failed: sf5.is_sign_positive()")
        }
        let sf6 = actual["sf6"].as_float().unwrap();
        if !sf6.is_nan() {
            ::core::panicking::panic("assertion failed: sf6.is_nan()")
        }
        if !sf6.is_sign_negative() {
            ::core::panicking::panic("assertion failed: sf6.is_sign_negative()")
        }
    }
    extern crate test;
    #[rustc_test_marker = "macros::test_datetime"]
    #[doc(hidden)]
    pub const test_datetime: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_datetime"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 216usize,
            start_col: 4usize,
            end_line: 216usize,
            end_col: 17usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_datetime()),
        ),
    };
    fn test_datetime() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-odt1"[1..]],
                ::toml::Value::Datetime("1979-05-27T07:32:00Z".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-odt2"[1..]],
                ::toml::Value::Datetime("1979-05-27T00:32:00-07:00".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-odt3"[1..]],
                ::toml::Value::Datetime(
                    "1979-05-27T00:32:00.999999-07:00".parse().unwrap(),
                ),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-odt4"[1..]],
                ::toml::Value::Datetime("1979-05-27T07:32:00Z".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-ldt1"[1..]],
                ::toml::Value::Datetime("1979-05-27T07:32:00".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-ldt2"[1..]],
                ::toml::Value::Datetime("1979-05-27T00:32:00.999999".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-ld1"[1..]],
                ::toml::Value::Datetime("1979-05-27".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-lt1"[1..]],
                ::toml::Value::Datetime("07:32:00".parse().unwrap()),
            );
            ::toml::macros::insert_toml(
                &mut root,
                &[&"-lt2"[1..]],
                ::toml::Value::Datetime("00:32:00.999999".parse().unwrap()),
            );
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-table"[1..]],
                    {
                        let mut table = ::toml::Value::Table(
                            ::toml::value::Table::new(),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-odt1"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T07:32:00Z".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-odt2"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T00:32:00-07:00".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-odt3"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T00:32:00.999999-07:00".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-odt4"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T07:32:00Z".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-ldt1"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T07:32:00".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-ldt2"[1..]],
                            ::toml::Value::Datetime(
                                "1979-05-27T00:32:00.999999".parse().unwrap(),
                            ),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-ld1"[1..]],
                            ::toml::Value::Datetime("1979-05-27".parse().unwrap()),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-lt1"[1..]],
                            ::toml::Value::Datetime("07:32:00".parse().unwrap()),
                        );
                        ::toml::macros::insert_toml(
                            &mut table,
                            &[&"-lt2"[1..]],
                            ::toml::Value::Datetime("00:32:00.999999".parse().unwrap()),
                        );
                        table
                    },
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-array"[1..]],
                        {
                            let mut array = ::toml::value::Array::new();
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T07:32:00Z".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T00:32:00-07:00".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T00:32:00.999999-07:00".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T07:32:00Z".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T07:32:00".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime(
                                        "1979-05-27T00:32:00.999999".parse().unwrap(),
                                    ),
                                );
                            array
                                .push(
                                    ::toml::Value::Datetime("1979-05-27".parse().unwrap()),
                                );
                            array
                                .push(::toml::Value::Datetime("07:32:00".parse().unwrap()));
                            array
                                .push(
                                    ::toml::Value::Datetime("00:32:00.999999".parse().unwrap()),
                                );
                            ::toml::Value::Array(array)
                        },
                    );
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table
                .insert(
                    "odt1".to_owned(),
                    "1979-05-27T07:32:00Z"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "odt2".to_owned(),
                    "1979-05-27T00:32:00-07:00"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "odt3".to_owned(),
                    "1979-05-27T00:32:00.999999-07:00"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "odt4".to_owned(),
                    "1979-05-27 07:32:00Z"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "ldt1".to_owned(),
                    "1979-05-27T07:32:00"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "ldt2".to_owned(),
                    "1979-05-27T00:32:00.999999"
                        .parse::<toml::value::Datetime>()
                        .unwrap()
                        .into(),
                );
            table
                .insert(
                    "ld1".to_owned(),
                    "1979-05-27".parse::<toml::value::Datetime>().unwrap().into(),
                );
            table
                .insert(
                    "lt1".to_owned(),
                    "07:32:00".parse::<toml::value::Datetime>().unwrap().into(),
                );
            table
                .insert(
                    "lt2".to_owned(),
                    "00:32:00.999999".parse::<toml::value::Datetime>().unwrap().into(),
                );
            table
                .insert(
                    "table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table
                            .insert(
                                "odt1".to_owned(),
                                "1979-05-27T07:32:00Z"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "odt2".to_owned(),
                                "1979-05-27T00:32:00-07:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "odt3".to_owned(),
                                "1979-05-27T00:32:00.999999-07:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "odt4".to_owned(),
                                "1979-05-27 07:32:00Z"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "ldt1".to_owned(),
                                "1979-05-27T07:32:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "ldt2".to_owned(),
                                "1979-05-27T00:32:00.999999"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "ld1".to_owned(),
                                "1979-05-27"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        table
                            .insert(
                                "lt1".to_owned(),
                                "07:32:00".parse::<toml::value::Datetime>().unwrap().into(),
                            );
                        table
                            .insert(
                                "lt2".to_owned(),
                                "00:32:00.999999"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "array".to_owned(),
                    {
                        #![allow(clippy::vec_init_then_push)]
                        #[allow(unused_mut)]
                        let mut array = toml::value::Array::new();
                        array
                            .push(
                                "1979-05-27T07:32:00Z"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27T00:32:00-07:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27T00:32:00.999999-07:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27 07:32:00Z"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27T07:32:00"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27T00:32:00.999999"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "1979-05-27"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        array
                            .push(
                                "07:32:00".parse::<toml::value::Datetime>().unwrap().into(),
                            );
                        array
                            .push(
                                "00:32:00.999999"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        toml::Value::Array(array)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_quoted_key"]
    #[doc(hidden)]
    pub const test_quoted_key: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_quoted_key"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 295usize,
            start_col: 4usize,
            end_line: 295usize,
            end_col: 19usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_quoted_key()),
        ),
    };
    fn test_quoted_key() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-quoted"[1..]],
                    {
                        let de = ::toml::macros::IntoDeserializer::<
                            ::toml::de::Error,
                        >::into_deserializer(true);
                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(de)
                            .unwrap()
                    },
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-table"[1..]],
                        {
                            let mut table = ::toml::Value::Table(
                                ::toml::value::Table::new(),
                            );
                            ::toml::macros::insert_toml(
                                &mut table,
                                &[&"-quoted"[1..]],
                                {
                                    let de = ::toml::macros::IntoDeserializer::<
                                        ::toml::de::Error,
                                    >::into_deserializer(true);
                                    <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                            de,
                                        )
                                        .unwrap()
                                },
                            );
                            table
                        },
                    );
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-target"[1..], &"-cfg(windows)"[1..], &"-dependencies"[1..]],
                        ::toml::Value::Table(::toml::value::Table::new()),
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[
                                &"-target"[1..],
                                &"-cfg(windows)"[1..],
                                &"-dependencies"[1..],
                                &"-winapi"[1..],
                            ],
                            {
                                let de = ::toml::macros::IntoDeserializer::<
                                    ::toml::de::Error,
                                >::into_deserializer("0.2.8");
                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                        de,
                                    )
                                    .unwrap()
                            },
                        );
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table.insert("quoted".to_owned(), true.into());
            table
                .insert(
                    "table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("quoted".to_owned(), true.into());
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "target".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table
                            .insert(
                                "cfg(windows)".to_owned(),
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table
                                        .insert(
                                            "dependencies".to_owned(),
                                            {
                                                #[allow(unused_mut)]
                                                let mut table = toml::value::Table::new();
                                                table.insert("winapi".to_owned(), "0.2.8".into());
                                                toml::Value::Table(table)
                                            }
                                                .into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_empty"]
    #[doc(hidden)]
    pub const test_empty: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_empty"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 322usize,
            start_col: 4usize,
            end_line: 322usize,
            end_col: 14usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_empty()),
        ),
    };
    fn test_empty() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-empty_inline_table"[1..]],
                    {
                        let mut table = ::toml::Value::Table(
                            ::toml::value::Table::new(),
                        );
                        table
                    },
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-empty_inline_array"[1..]],
                        {
                            let mut array = ::toml::value::Array::new();
                            ::toml::Value::Array(array)
                        },
                    );
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-empty_table"[1..]],
                        ::toml::Value::Table(::toml::value::Table::new()),
                    );
                    ::toml::macros::push_toml(&mut root, &[&"-empty_array"[1..]]);
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table
                .insert(
                    "empty_inline_table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "empty_inline_array".to_owned(),
                    {
                        #![allow(clippy::vec_init_then_push)]
                        #[allow(unused_mut)]
                        let mut array = toml::value::Array::new();
                        toml::Value::Array(array)
                    }
                        .into(),
                );
            table
                .insert(
                    "empty_table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "empty_array".to_owned(),
                    {
                        #![allow(clippy::vec_init_then_push)]
                        #[allow(unused_mut)]
                        let mut array = toml::value::Array::new();
                        array
                            .push(
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        toml::Value::Array(array)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
    #[rustc_test_marker = "macros::test_dotted_keys"]
    #[doc(hidden)]
    pub const test_dotted_keys: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("macros::test_dotted_keys"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/macros.rs",
            start_line: 345usize,
            start_col: 4usize,
            end_line: 345usize,
            end_col: 20usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_dotted_keys()),
        ),
    };
    fn test_dotted_keys() {
        let actual = {
            let table = ::toml::value::Table::new();
            let mut root = ::toml::Value::Table(table);
            {
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-a"[1..], &"-b"[1..]],
                    {
                        let de = ::toml::macros::IntoDeserializer::<
                            ::toml::de::Error,
                        >::into_deserializer(123);
                        <::toml::Value as ::toml::macros::Deserialize>::deserialize(de)
                            .unwrap()
                    },
                );
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-a"[1..], &"-c"[1..]],
                    ::toml::Value::Datetime("1979-05-27T07:32:00Z".parse().unwrap()),
                );
                ::toml::macros::insert_toml(
                    &mut root,
                    &[&"-table"[1..]],
                    ::toml::Value::Table(::toml::value::Table::new()),
                );
                {
                    ::toml::macros::insert_toml(
                        &mut root,
                        &[&"-table"[1..], &"-a"[1..], &"-b"[1..], &"-c"[1..]],
                        {
                            let de = ::toml::macros::IntoDeserializer::<
                                ::toml::de::Error,
                            >::into_deserializer(1);
                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                    de,
                                )
                                .unwrap()
                        },
                    );
                    {
                        ::toml::macros::insert_toml(
                            &mut root,
                            &[&"-table"[1..], &"-a"[1..], &"-b"[1..], &"-d"[1..]],
                            {
                                let de = ::toml::macros::IntoDeserializer::<
                                    ::toml::de::Error,
                                >::into_deserializer(2);
                                <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                        de,
                                    )
                                    .unwrap()
                            },
                        );
                        {
                            ::toml::macros::insert_toml(
                                &mut root,
                                &[&"-table"[1..], &"-in"[1..]],
                                {
                                    let mut table = ::toml::Value::Table(
                                        ::toml::value::Table::new(),
                                    );
                                    ::toml::macros::insert_toml(
                                        &mut table,
                                        &[&"-type"[1..], &"-name"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer("cat");
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    ::toml::macros::insert_toml(
                                        &mut table,
                                        &[&"-type"[1..], &"-color"[1..]],
                                        {
                                            let de = ::toml::macros::IntoDeserializer::<
                                                ::toml::de::Error,
                                            >::into_deserializer("blue");
                                            <::toml::Value as ::toml::macros::Deserialize>::deserialize(
                                                    de,
                                                )
                                                .unwrap()
                                        },
                                    );
                                    table
                                },
                            );
                        };
                    };
                };
            };
            match root {
                ::toml::Value::Table(table) => table,
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        };
        let expected = {
            #[allow(unused_mut)]
            let mut table = toml::value::Table::new();
            table
                .insert(
                    "a".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table.insert("b".to_owned(), 123.into());
                        table
                            .insert(
                                "c".to_owned(),
                                "1979-05-27T07:32:00Z"
                                    .parse::<toml::value::Datetime>()
                                    .unwrap()
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            table
                .insert(
                    "table".to_owned(),
                    {
                        #[allow(unused_mut)]
                        let mut table = toml::value::Table::new();
                        table
                            .insert(
                                "a".to_owned(),
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table
                                        .insert(
                                            "b".to_owned(),
                                            {
                                                #[allow(unused_mut)]
                                                let mut table = toml::value::Table::new();
                                                table.insert("c".to_owned(), 1.into());
                                                table.insert("d".to_owned(), 2.into());
                                                toml::Value::Table(table)
                                            }
                                                .into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        table
                            .insert(
                                "in".to_owned(),
                                {
                                    #[allow(unused_mut)]
                                    let mut table = toml::value::Table::new();
                                    table
                                        .insert(
                                            "type".to_owned(),
                                            {
                                                #[allow(unused_mut)]
                                                let mut table = toml::value::Table::new();
                                                table.insert("name".to_owned(), "cat".into());
                                                table.insert("color".to_owned(), "blue".into());
                                                toml::Value::Table(table)
                                            }
                                                .into(),
                                        );
                                    toml::Value::Table(table)
                                }
                                    .into(),
                            );
                        toml::Value::Table(table)
                    }
                        .into(),
                );
            toml::Value::Table(table)
        };
        match (&toml::Value::Table(actual), &expected) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
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
mod table {
    use snapbox::assert_data_eq;
    use snapbox::prelude::*;
    use snapbox::str;
    use toml::Value::{Array, Integer, String, Table};
    use toml::map::Map;
    extern crate test;
    #[rustc_test_marker = "table::display"]
    #[doc(hidden)]
    pub const display: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("table::display"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/table.rs",
            start_line: 9usize,
            start_col: 4usize,
            end_line: 9usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(display()),
        ),
    };
    fn display() {
        {
            let actual = ::snapbox::IntoData::into_data(
                {
                    let mut _m = Map::new();
                    _m
                }
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data("");
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                {
                    let mut _m = Map::new();
                    _m.insert("test".to_owned(), Integer(2));
                    _m.insert("test2".to_owned(), Integer(3));
                    _m
                }
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/table.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 16u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: r#"
test = 2
test2 = 3

"#,
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                {
                    let mut _m = Map::new();
                    _m.insert("test".to_owned(), Integer(2));
                    _m.insert(
                        "test2".to_owned(),
                        Table({
                            let mut _m = Map::new();
                            _m.insert("test".to_owned(), String("wut".to_owned()));
                            _m
                        }),
                    );
                    _m
                }
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/table.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 31u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: r#"
test = 2

[test2]
test = "wut"

"#,
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                {
                    let mut _m = Map::new();
                    _m.insert("test".to_owned(), Integer(2));
                    _m.insert(
                        "test2".to_owned(),
                        Array(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    Table({
                                        let mut _m = Map::new();
                                        _m.insert("test".to_owned(), String("wut".to_owned()));
                                        _m
                                    }),
                                ]),
                            ),
                        ),
                    );
                    _m
                }
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/table.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 48u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: r#"
test = 2

[[test2]]
test = "wut"

"#,
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
    extern crate test;
    #[rustc_test_marker = "table::datetime_offset_issue_496"]
    #[doc(hidden)]
    pub const datetime_offset_issue_496: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("table::datetime_offset_issue_496"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/table.rs",
            start_line: 60usize,
            start_col: 4usize,
            end_line: 60usize,
            end_col: 29usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(datetime_offset_issue_496()),
        ),
    };
    fn datetime_offset_issue_496() {
        let original = "value = 1911-01-01T10:11:12-00:36\n";
        let toml = original.parse::<toml::Table>().unwrap();
        let output = toml.to_string();
        {
            let actual = ::snapbox::IntoData::into_data(output);
            let expected = ::snapbox::IntoData::into_data(original.raw());
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
}
mod value {
    use snapbox::assert_data_eq;
    use snapbox::prelude::*;
    use snapbox::str;
    use toml::Value::{Array, Boolean, Float, Integer, String, Table};
    use toml::map::Map;
    extern crate test;
    #[rustc_test_marker = "value::display"]
    #[doc(hidden)]
    pub const display: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("value::display"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "crates/toml/tests/testsuite/value.rs",
            start_line: 9usize,
            start_col: 4usize,
            end_line: 9usize,
            end_col: 11usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::Unknown,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(display()),
        ),
    };
    fn display() {
        {
            let actual = ::snapbox::IntoData::into_data(
                String("foo".to_owned()).to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 12u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: r#""foo""#,
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(Integer(10).to_string());
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 14u32,
                        column: 46u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "10",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(Float(10.0).to_string());
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 15u32,
                        column: 46u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "10.0",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(Float(2.4).to_string());
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 16u32,
                        column: 45u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "2.4",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(Boolean(true).to_string());
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 17u32,
                        column: 48u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "true",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                Array(::alloc::vec::Vec::new()).to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 18u32,
                        column: 48u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "[]",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                Array(<[_]>::into_vec(::alloc::boxed::box_new([Integer(1), Integer(2)])))
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 21u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "[1, 2]",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
        {
            let actual = ::snapbox::IntoData::into_data(
                Table({
                        let mut _m = Map::new();
                        _m.insert("test".to_owned(), Integer(2));
                        _m.insert("test2".to_owned(), Integer(3));
                        _m
                    })
                    .to_string(),
            );
            let expected = ::snapbox::IntoData::into_data(
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
                            let file = "crates/toml/tests/testsuite/value.rs";
                            let rel_path = ::std::path::Path::new(file);
                            root.join(rel_path)
                        },
                        line: 25u32,
                        column: 9u32,
                    };
                    let inline = ::snapbox::data::Inline {
                        position,
                        data: "{ test = 2, test2 = 3 }",
                    };
                    inline
                }
                    .raw(),
            );
            ::snapbox::Assert::new()
                .action_env(::snapbox::assert::DEFAULT_ACTION_ENV)
                .eq(actual, expected);
        };
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_array,
            &test_cargo_toml,
            &test_datetime,
            &test_dotted_keys,
            &test_empty,
            &test_nan,
            &test_number,
            &test_quoted_key,
            &datetime_offset_issue_496,
            &display,
            &display,
        ],
    )
}
