#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
mod decoder {
    #![allow(dead_code)]
    pub(crate) struct Decoder;
    #[automatically_derived]
    impl ::core::marker::Copy for Decoder {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Decoder {}
    #[automatically_derived]
    impl ::core::clone::Clone for Decoder {
        #[inline]
        fn clone(&self) -> Decoder {
            *self
        }
    }
    impl toml_test_harness::Decoder for Decoder {
        fn name(&self) -> &str {
            "toml"
        }
        fn decode(
            &self,
            data: &[u8],
        ) -> Result<toml_test_harness::DecodedValue, toml_test_harness::Error> {
            use itertools::Itertools as _;
            use serde::de::Deserialize as _;
            let data = std::str::from_utf8(data).map_err(toml_test_harness::Error::new)?;
            let (table, errors) = toml::de::DeTable::parse_recoverable(data);
            if !errors.is_empty() {
                let errors = errors.into_iter().join("\n---\n");
                let error = toml_test_harness::Error::new(errors);
                return Err(error);
            }
            let document = toml::Table::deserialize(toml::de::Deserializer::from(table))
                .map_err(|mut err| {
                    err.set_input(Some(data));
                    toml_test_harness::Error::new(err)
                })?;
            let value = toml::Value::Table(document);
            value_to_decoded(&value)
        }
    }
    fn value_to_decoded(
        value: &toml::Value,
    ) -> Result<toml_test_harness::DecodedValue, toml_test_harness::Error> {
        match value {
            toml::Value::Integer(v) => {
                Ok(
                    toml_test_harness::DecodedValue::Scalar(
                        toml_test_harness::DecodedScalar::from(*v),
                    ),
                )
            }
            toml::Value::String(v) => {
                Ok(
                    toml_test_harness::DecodedValue::Scalar(
                        toml_test_harness::DecodedScalar::from(v),
                    ),
                )
            }
            toml::Value::Float(v) => {
                Ok(
                    toml_test_harness::DecodedValue::Scalar(
                        toml_test_harness::DecodedScalar::from(*v),
                    ),
                )
            }
            &toml::Value::Datetime(mut v) => {
                if let Some(time) = &mut v.time {
                    if time.second.is_none() {
                        time.second = Some(0);
                    }
                    if time.nanosecond.is_none() {
                        time.nanosecond = Some(0);
                    }
                }
                let value = v.to_string();
                let value = match (
                    v.date.is_some(),
                    v.time.is_some(),
                    v.offset.is_some(),
                ) {
                    (true, true, true) => {
                        toml_test_harness::DecodedScalar::Datetime(value)
                    }
                    (true, true, false) => {
                        toml_test_harness::DecodedScalar::DatetimeLocal(value)
                    }
                    (true, false, false) => {
                        toml_test_harness::DecodedScalar::DateLocal(value)
                    }
                    (false, true, false) => {
                        toml_test_harness::DecodedScalar::TimeLocal(value)
                    }
                    _ => {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "internal error: entered unreachable code: {0}",
                                format_args!("Unsupported case"),
                            ),
                        );
                    }
                };
                Ok(toml_test_harness::DecodedValue::Scalar(value))
            }
            toml::Value::Boolean(v) => {
                Ok(
                    toml_test_harness::DecodedValue::Scalar(
                        toml_test_harness::DecodedScalar::from(*v),
                    ),
                )
            }
            toml::Value::Array(v) => {
                let v: Result<_, toml_test_harness::Error> = v
                    .iter()
                    .map(value_to_decoded)
                    .collect();
                Ok(toml_test_harness::DecodedValue::Array(v?))
            }
            toml::Value::Table(v) => table_to_decoded(v),
        }
    }
    fn table_to_decoded(
        value: &toml::value::Table,
    ) -> Result<toml_test_harness::DecodedValue, toml_test_harness::Error> {
        let table: Result<_, toml_test_harness::Error> = value
            .iter()
            .map(|(k, v)| {
                let k = k.to_owned();
                let v = value_to_decoded(v)?;
                Ok((k, v))
            })
            .collect();
        Ok(toml_test_harness::DecodedValue::Table(table?))
    }
}
fn main() {
    let valid_ext = walkdir::WalkDir::new("tests/fixtures/valid")
        .sort_by_file_name()
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("toml")))
        .map(|e| {
            let name = e
                .path()
                .strip_prefix("tests/fixtures")
                .unwrap()
                .to_owned()
                .into();
            let fixture = std::fs::read(e.path()).unwrap().into();
            let expected_path = e.path().with_extension("json");
            let expected = std::fs::read(expected_path).unwrap().into();
            toml_test_data::Valid {
                name,
                fixture,
                expected,
            }
        })
        .collect::<Vec<_>>();
    let invalid_ext = walkdir::WalkDir::new("tests/fixtures/invalid")
        .sort_by_file_name()
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("toml")))
        .map(|e| {
            let name = e
                .path()
                .strip_prefix("tests/fixtures")
                .unwrap()
                .to_owned()
                .into();
            let fixture = std::fs::read(e.path()).unwrap().into();
            toml_test_data::Invalid {
                name,
                fixture,
            }
        })
        .collect::<Vec<_>>();
    let decoder = decoder::Decoder;
    let mut harness = toml_test_harness::DecoderHarness::new(decoder);
    harness.version("1.1.0");
    harness.ignore([]).unwrap();
    harness.snapshot_root("tests/snapshots");
    harness.extend_valid(valid_ext);
    harness.extend_invalid(invalid_ext);
    harness.test();
}
