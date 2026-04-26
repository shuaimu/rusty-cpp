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
mod encoder {
    #![allow(dead_code)]
    pub(crate) struct Encoder;
    #[automatically_derived]
    impl ::core::marker::Copy for Encoder {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Encoder {}
    #[automatically_derived]
    impl ::core::clone::Clone for Encoder {
        #[inline]
        fn clone(&self) -> Encoder {
            *self
        }
    }
    impl toml_test_harness::Encoder for Encoder {
        fn name(&self) -> &str {
            "toml"
        }
        fn encode(
            &self,
            data: toml_test_harness::DecodedValue,
        ) -> Result<String, toml_test_harness::Error> {
            let value = from_decoded(&data)?;
            let toml::Value::Table(document) = value else {
                return Err(toml_test_harness::Error::new("no root table"));
            };
            let s = toml::to_string(&document).map_err(toml_test_harness::Error::new)?;
            Ok(s)
        }
    }
    pub(crate) struct EncoderPretty;
    #[automatically_derived]
    impl ::core::marker::Copy for EncoderPretty {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for EncoderPretty {}
    #[automatically_derived]
    impl ::core::clone::Clone for EncoderPretty {
        #[inline]
        fn clone(&self) -> EncoderPretty {
            *self
        }
    }
    impl toml_test_harness::Encoder for EncoderPretty {
        fn name(&self) -> &str {
            "toml"
        }
        fn encode(
            &self,
            data: toml_test_harness::DecodedValue,
        ) -> Result<String, toml_test_harness::Error> {
            let value = from_decoded(&data)?;
            let toml::Value::Table(document) = value else {
                return Err(toml_test_harness::Error::new("no root table"));
            };
            let s = toml::to_string_pretty(&document)
                .map_err(toml_test_harness::Error::new)?;
            Ok(s)
        }
    }
    fn from_decoded(
        decoded: &toml_test_harness::DecodedValue,
    ) -> Result<toml::Value, toml_test_harness::Error> {
        let value = match decoded {
            toml_test_harness::DecodedValue::Scalar(value) => from_decoded_scalar(value)?,
            toml_test_harness::DecodedValue::Table(value) => {
                toml::Value::Table(from_table(value)?)
            }
            toml_test_harness::DecodedValue::Array(value) => {
                toml::Value::Array(from_array(value)?)
            }
        };
        Ok(value)
    }
    fn from_decoded_scalar(
        decoded: &toml_test_harness::DecodedScalar,
    ) -> Result<toml::Value, toml_test_harness::Error> {
        match decoded {
            toml_test_harness::DecodedScalar::String(value) => {
                Ok(toml::Value::String(value.clone()))
            }
            toml_test_harness::DecodedScalar::Integer(value) => {
                value
                    .parse::<i64>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Integer)
            }
            toml_test_harness::DecodedScalar::Float(value) => {
                value
                    .parse::<f64>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Float)
            }
            toml_test_harness::DecodedScalar::Bool(value) => {
                value
                    .parse::<bool>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Boolean)
            }
            toml_test_harness::DecodedScalar::Datetime(value) => {
                value
                    .parse::<toml::value::Datetime>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Datetime)
            }
            toml_test_harness::DecodedScalar::DatetimeLocal(value) => {
                value
                    .parse::<toml::value::Datetime>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Datetime)
            }
            toml_test_harness::DecodedScalar::DateLocal(value) => {
                value
                    .parse::<toml::value::Datetime>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Datetime)
            }
            toml_test_harness::DecodedScalar::TimeLocal(value) => {
                value
                    .parse::<toml::value::Datetime>()
                    .map_err(toml_test_harness::Error::new)
                    .map(toml::Value::Datetime)
            }
        }
    }
    fn from_table(
        decoded: &std::collections::HashMap<String, toml_test_harness::DecodedValue>,
    ) -> Result<toml::value::Table, toml_test_harness::Error> {
        decoded
            .iter()
            .map(|(k, v)| {
                let v = from_decoded(v)?;
                Ok((k.to_owned(), v))
            })
            .collect()
    }
    fn from_array(
        decoded: &[toml_test_harness::DecodedValue],
    ) -> Result<toml::value::Array, toml_test_harness::Error> {
        decoded.iter().map(from_decoded).collect()
    }
}
fn main() {
    let encoder = encoder::Encoder;
    let decoder = decoder::Decoder;
    let mut harness = toml_test_harness::EncoderHarness::new(encoder, decoder);
    harness.version("1.0.0");
    harness.test();
}
