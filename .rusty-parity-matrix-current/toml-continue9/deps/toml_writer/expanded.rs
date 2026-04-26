#![feature(prelude_import)]
//! A low-level interface for writing out TOML
//!
//! Considerations when serializing arbitrary data:
//! - Verify the implementation with [`toml-test-harness`](https://docs.rs/toml-test-harness)
//! - Be sure to group keys under a table before writing another table
//! - Watch for extra trailing newlines and leading newlines, both when starting with top-level
//!   keys or a table
//! - When serializing an array-of-tables, be sure to verify that all elements of the array
//!   serialize as tables
//! - Standard tables and inline tables may need separate implementations of corner cases,
//!   requiring verifying them both
//!
//! When serializing Rust data structures
//! - `Option`: Skip key-value pairs with a value of `None`, otherwise error when seeing `None`
//!   - When skipping key-value pairs, be careful that a deeply nested `None` doesn't get skipped
//! - Scalars and arrays are unsupported as top-level data types
//! - Tuples and tuple variants seriallize as arrays
//! - Structs, struct variants, and maps serialize as tables
//! - Newtype variants serialize as to the inner type
//! - Unit variants serialize to a string
//! - Unit and unit structs don't have a clear meaning in TOML
//!
//! # Example
//!
//! ```rust
//! use toml_writer::TomlWrite as _;
//!
//! # fn main() -> std::fmt::Result {
//! let mut output = String::new();
//! output.newline()?;
//! output.open_table_header()?;
//! output.key("table")?;
//! output.close_table_header()?;
//! output.newline()?;
//!
//! output.key("key")?;
//! output.space()?;
//! output.keyval_sep()?;
//! output.space()?;
//! output.value("value")?;
//! output.newline()?;
//!
//! assert_eq!(output, r#"
//! [table]
//! key = "value"
//! "#);
//! #   Ok(())
//! # }
//! ```
#![forbid(unsafe_code)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
extern crate alloc;
mod integer {
    use core::fmt::{self, Display};
    /// Describes how a TOML integer should be formatted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "alloc")] {
    /// # use toml_writer::ToTomlValue as _;
    /// let format = toml_writer::TomlIntegerFormat::new().as_hex_lower();
    /// let number = 10;
    /// let number = format.format(number).unwrap_or(toml_writer::TomlInteger::new(number));
    /// let number = number.to_toml_value();
    /// assert_eq!(number, "0xa");
    /// # }
    /// ```
    pub struct TomlIntegerFormat {
        radix: Radix,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TomlIntegerFormat {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for TomlIntegerFormat {}
    #[automatically_derived]
    impl ::core::clone::Clone for TomlIntegerFormat {
        #[inline]
        fn clone(&self) -> TomlIntegerFormat {
            let _: ::core::clone::AssertParamIsClone<Radix>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for TomlIntegerFormat {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "TomlIntegerFormat",
                "radix",
                &&self.radix,
            )
        }
    }
    impl TomlIntegerFormat {
        /// Creates a new integer format (decimal).
        pub fn new() -> Self {
            Self { radix: Radix::Decimal }
        }
        /// Sets the format to decimal.
        pub fn as_decimal(mut self) -> Self {
            self.radix = Radix::Decimal;
            self
        }
        /// Sets the format to hexadecimal with all characters in uppercase.
        pub fn as_hex_upper(mut self) -> Self {
            self.radix = Radix::Hexadecimal {
                case: HexCase::Upper,
            };
            self
        }
        /// Sets the format to hexadecimal with all characters in lowercase.
        pub fn as_hex_lower(mut self) -> Self {
            self.radix = Radix::Hexadecimal {
                case: HexCase::Lower,
            };
            self
        }
        /// Sets the format to octal.
        pub fn as_octal(mut self) -> Self {
            self.radix = Radix::Octal;
            self
        }
        /// Sets the format to binary.
        pub fn as_binary(mut self) -> Self {
            self.radix = Radix::Binary;
            self
        }
        /// Formats `value` as a TOML integer.
        ///
        /// Returns `None` if the value cannot be formatted
        /// (e.g. value is negative and the radix is not decimal).
        pub fn format<N: PartialOrd<i32>>(self, value: N) -> Option<TomlInteger<N>>
        where
            TomlInteger<N>: crate::WriteTomlValue,
        {
            match self.radix {
                Radix::Decimal => {}
                Radix::Hexadecimal { .. } | Radix::Octal | Radix::Binary => {
                    if value < 0 {
                        return None;
                    }
                }
            }
            Some(TomlInteger { value, format: self })
        }
    }
    impl Default for TomlIntegerFormat {
        fn default() -> Self {
            Self::new()
        }
    }
    /// Helper struct for formatting TOML integers.
    ///
    /// This may be constructed by calling [`TomlIntegerFormat::format()`].
    pub struct TomlInteger<N> {
        value: N,
        format: TomlIntegerFormat,
    }
    #[automatically_derived]
    impl<N: ::core::marker::Copy> ::core::marker::Copy for TomlInteger<N> {}
    #[automatically_derived]
    impl<N: ::core::clone::Clone> ::core::clone::Clone for TomlInteger<N> {
        #[inline]
        fn clone(&self) -> TomlInteger<N> {
            TomlInteger {
                value: ::core::clone::Clone::clone(&self.value),
                format: ::core::clone::Clone::clone(&self.format),
            }
        }
    }
    #[automatically_derived]
    impl<N: ::core::fmt::Debug> ::core::fmt::Debug for TomlInteger<N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TomlInteger",
                "value",
                &self.value,
                "format",
                &&self.format,
            )
        }
    }
    impl<N> TomlInteger<N>
    where
        Self: crate::WriteTomlValue,
    {
        /// Apply default formatting
        pub fn new(value: N) -> Self {
            Self {
                value,
                format: TomlIntegerFormat::new(),
            }
        }
    }
    impl crate::WriteTomlValue for TomlInteger<u8> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<i8> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<u16> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<i16> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<u32> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<i32> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<u64> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<i64> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<u128> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<i128> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<usize> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    impl crate::WriteTomlValue for TomlInteger<isize> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> fmt::Result {
            write_toml_value(self.value, &self.format, writer)
        }
    }
    enum Radix {
        Decimal,
        Hexadecimal { case: HexCase },
        Octal,
        Binary,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Radix {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Radix {}
    #[automatically_derived]
    impl ::core::clone::Clone for Radix {
        #[inline]
        fn clone(&self) -> Radix {
            let _: ::core::clone::AssertParamIsClone<HexCase>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Radix {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Radix::Decimal => ::core::fmt::Formatter::write_str(f, "Decimal"),
                Radix::Hexadecimal { case: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Hexadecimal",
                        "case",
                        &__self_0,
                    )
                }
                Radix::Octal => ::core::fmt::Formatter::write_str(f, "Octal"),
                Radix::Binary => ::core::fmt::Formatter::write_str(f, "Binary"),
            }
        }
    }
    enum HexCase {
        Upper,
        Lower,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for HexCase {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for HexCase {}
    #[automatically_derived]
    impl ::core::clone::Clone for HexCase {
        #[inline]
        fn clone(&self) -> HexCase {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for HexCase {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    HexCase::Upper => "Upper",
                    HexCase::Lower => "Lower",
                },
            )
        }
    }
    fn write_toml_value<
        N: Display + fmt::UpperHex + fmt::LowerHex + fmt::Octal + fmt::Binary,
        W: crate::TomlWrite + ?Sized,
    >(value: N, format: &TomlIntegerFormat, writer: &mut W) -> fmt::Result {
        match format.radix {
            Radix::Decimal => writer.write_fmt(format_args!("{0}", value))?,
            Radix::Hexadecimal { case } => {
                match case {
                    HexCase::Upper => writer.write_fmt(format_args!("0x{0:X}", value))?,
                    HexCase::Lower => writer.write_fmt(format_args!("0x{0:x}", value))?,
                }
            }
            Radix::Octal => writer.write_fmt(format_args!("0o{0:o}", value))?,
            Radix::Binary => writer.write_fmt(format_args!("0b{0:b}", value))?,
        }
        Ok(())
    }
}
mod key {
    use alloc::borrow::Cow;
    use alloc::string::String;
    use crate::TomlWrite;
    pub trait ToTomlKey {
        fn to_toml_key(&self) -> String;
    }
    impl<T> ToTomlKey for T
    where
        T: WriteTomlKey + ?Sized,
    {
        fn to_toml_key(&self) -> String {
            let mut result = String::new();
            let _ = self.write_toml_key(&mut result);
            result
        }
    }
    pub trait WriteTomlKey {
        fn write_toml_key<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result;
    }
    impl WriteTomlKey for str {
        fn write_toml_key<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            crate::TomlKeyBuilder::new(self).as_default().write_toml_key(writer)
        }
    }
    impl WriteTomlKey for String {
        fn write_toml_key<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_str().write_toml_key(writer)
        }
    }
    impl WriteTomlKey for Cow<'_, str> {
        fn write_toml_key<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_ref().write_toml_key(writer)
        }
    }
    impl<V: WriteTomlKey + ?Sized> WriteTomlKey for &V {
        fn write_toml_key<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            (*self).write_toml_key(writer)
        }
    }
}
mod string {
    /// Describes how a TOML string (key or value) should be formatted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "alloc")] {
    /// # use toml_writer::ToTomlValue as _;
    /// let string = "Hello
    /// world!
    /// ";
    /// let string = toml_writer::TomlStringBuilder::new(string).as_default();
    /// let string = string.to_toml_value();
    /// assert_eq!(string, r#""""
    /// Hello
    /// world!
    /// """"#);
    /// # }
    /// ```
    pub struct TomlStringBuilder<'s> {
        decoded: &'s str,
        metrics: ValueMetrics,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for TomlStringBuilder<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for TomlStringBuilder<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for TomlStringBuilder<'s> {
        #[inline]
        fn clone(&self) -> TomlStringBuilder<'s> {
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            let _: ::core::clone::AssertParamIsClone<ValueMetrics>;
            *self
        }
    }
    #[automatically_derived]
    impl<'s> ::core::fmt::Debug for TomlStringBuilder<'s> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TomlStringBuilder",
                "decoded",
                &self.decoded,
                "metrics",
                &&self.metrics,
            )
        }
    }
    impl<'s> TomlStringBuilder<'s> {
        pub fn new(decoded: &'s str) -> Self {
            Self {
                decoded,
                metrics: ValueMetrics::calculate(decoded),
            }
        }
        pub fn as_default(&self) -> TomlString<'s> {
            self.as_basic_pretty()
                .or_else(|| self.as_literal())
                .or_else(|| self.as_ml_basic_pretty())
                .or_else(|| self.as_ml_literal())
                .unwrap_or_else(|| {
                    if self.metrics.newline {
                        self.as_ml_basic()
                    } else {
                        self.as_basic()
                    }
                })
        }
        pub fn as_literal(&self) -> Option<TomlString<'s>> {
            if self.metrics.escape_codes || 0 < self.metrics.max_seq_single_quotes
                || self.metrics.newline
            {
                None
            } else {
                Some(TomlString {
                    decoded: self.decoded,
                    encoding: Encoding::LiteralString,
                    newline: self.metrics.newline,
                })
            }
        }
        pub fn as_ml_literal(&self) -> Option<TomlString<'s>> {
            if self.metrics.escape_codes || 2 < self.metrics.max_seq_single_quotes {
                None
            } else {
                Some(TomlString {
                    decoded: self.decoded,
                    encoding: Encoding::MlLiteralString,
                    newline: self.metrics.newline,
                })
            }
        }
        pub fn as_basic_pretty(&self) -> Option<TomlString<'s>> {
            if self.metrics.escape_codes || self.metrics.escape
                || 0 < self.metrics.max_seq_double_quotes || self.metrics.newline
            {
                None
            } else {
                Some(self.as_basic())
            }
        }
        pub fn as_ml_basic_pretty(&self) -> Option<TomlString<'s>> {
            if self.metrics.escape_codes || self.metrics.escape
                || 2 < self.metrics.max_seq_double_quotes
            {
                None
            } else {
                Some(self.as_ml_basic())
            }
        }
        pub fn as_basic(&self) -> TomlString<'s> {
            TomlString {
                decoded: self.decoded,
                encoding: Encoding::BasicString,
                newline: self.metrics.newline,
            }
        }
        pub fn as_ml_basic(&self) -> TomlString<'s> {
            TomlString {
                decoded: self.decoded,
                encoding: Encoding::MlBasicString,
                newline: self.metrics.newline,
            }
        }
    }
    pub struct TomlString<'s> {
        decoded: &'s str,
        encoding: Encoding,
        newline: bool,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for TomlString<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for TomlString<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for TomlString<'s> {
        #[inline]
        fn clone(&self) -> TomlString<'s> {
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            let _: ::core::clone::AssertParamIsClone<Encoding>;
            let _: ::core::clone::AssertParamIsClone<bool>;
            *self
        }
    }
    #[automatically_derived]
    impl<'s> ::core::marker::StructuralPartialEq for TomlString<'s> {}
    #[automatically_derived]
    impl<'s> ::core::cmp::PartialEq for TomlString<'s> {
        #[inline]
        fn eq(&self, other: &TomlString<'s>) -> bool {
            self.newline == other.newline && self.decoded == other.decoded
                && self.encoding == other.encoding
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::Eq for TomlString<'s> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'s str>;
            let _: ::core::cmp::AssertParamIsEq<Encoding>;
            let _: ::core::cmp::AssertParamIsEq<bool>;
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::PartialOrd for TomlString<'s> {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TomlString<'s>,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.decoded, &other.decoded) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    match ::core::cmp::PartialOrd::partial_cmp(
                        &self.encoding,
                        &other.encoding,
                    ) {
                        ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                            ::core::cmp::PartialOrd::partial_cmp(
                                &self.newline,
                                &other.newline,
                            )
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::Ord for TomlString<'s> {
        #[inline]
        fn cmp(&self, other: &TomlString<'s>) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.decoded, &other.decoded) {
                ::core::cmp::Ordering::Equal => {
                    match ::core::cmp::Ord::cmp(&self.encoding, &other.encoding) {
                        ::core::cmp::Ordering::Equal => {
                            ::core::cmp::Ord::cmp(&self.newline, &other.newline)
                        }
                        cmp => cmp,
                    }
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl<'s> ::core::hash::Hash for TomlString<'s> {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            ::core::hash::Hash::hash(&self.decoded, state);
            ::core::hash::Hash::hash(&self.encoding, state);
            ::core::hash::Hash::hash(&self.newline, state)
        }
    }
    #[automatically_derived]
    impl<'s> ::core::fmt::Debug for TomlString<'s> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "TomlString",
                "decoded",
                &self.decoded,
                "encoding",
                &self.encoding,
                "newline",
                &&self.newline,
            )
        }
    }
    impl crate::WriteTomlValue for TomlString<'_> {
        fn write_toml_value<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            write_toml_value(self.decoded, Some(self.encoding), self.newline, writer)
        }
    }
    pub struct TomlKeyBuilder<'s> {
        decoded: &'s str,
        metrics: KeyMetrics,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for TomlKeyBuilder<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for TomlKeyBuilder<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for TomlKeyBuilder<'s> {
        #[inline]
        fn clone(&self) -> TomlKeyBuilder<'s> {
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            let _: ::core::clone::AssertParamIsClone<KeyMetrics>;
            *self
        }
    }
    #[automatically_derived]
    impl<'s> ::core::fmt::Debug for TomlKeyBuilder<'s> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TomlKeyBuilder",
                "decoded",
                &self.decoded,
                "metrics",
                &&self.metrics,
            )
        }
    }
    impl<'s> TomlKeyBuilder<'s> {
        pub fn new(decoded: &'s str) -> Self {
            Self {
                decoded,
                metrics: KeyMetrics::calculate(decoded),
            }
        }
        pub fn as_default(&self) -> TomlKey<'s> {
            self.as_unquoted()
                .or_else(|| self.as_basic_pretty())
                .or_else(|| self.as_literal())
                .unwrap_or_else(|| self.as_basic())
        }
        pub fn as_unquoted(&self) -> Option<TomlKey<'s>> {
            if self.metrics.unquoted {
                Some(TomlKey {
                    decoded: self.decoded,
                    encoding: None,
                })
            } else {
                None
            }
        }
        pub fn as_literal(&self) -> Option<TomlKey<'s>> {
            if self.metrics.escape_codes || self.metrics.single_quotes {
                None
            } else {
                Some(TomlKey {
                    decoded: self.decoded,
                    encoding: Some(Encoding::LiteralString),
                })
            }
        }
        pub fn as_basic_pretty(&self) -> Option<TomlKey<'s>> {
            if self.metrics.escape_codes || self.metrics.escape
                || self.metrics.double_quotes
            {
                None
            } else {
                Some(self.as_basic())
            }
        }
        pub fn as_basic(&self) -> TomlKey<'s> {
            TomlKey {
                decoded: self.decoded,
                encoding: Some(Encoding::BasicString),
            }
        }
    }
    pub struct TomlKey<'s> {
        decoded: &'s str,
        encoding: Option<Encoding>,
    }
    #[automatically_derived]
    impl<'s> ::core::marker::Copy for TomlKey<'s> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'s> ::core::clone::TrivialClone for TomlKey<'s> {}
    #[automatically_derived]
    impl<'s> ::core::clone::Clone for TomlKey<'s> {
        #[inline]
        fn clone(&self) -> TomlKey<'s> {
            let _: ::core::clone::AssertParamIsClone<&'s str>;
            let _: ::core::clone::AssertParamIsClone<Option<Encoding>>;
            *self
        }
    }
    #[automatically_derived]
    impl<'s> ::core::marker::StructuralPartialEq for TomlKey<'s> {}
    #[automatically_derived]
    impl<'s> ::core::cmp::PartialEq for TomlKey<'s> {
        #[inline]
        fn eq(&self, other: &TomlKey<'s>) -> bool {
            self.decoded == other.decoded && self.encoding == other.encoding
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::Eq for TomlKey<'s> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'s str>;
            let _: ::core::cmp::AssertParamIsEq<Option<Encoding>>;
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::PartialOrd for TomlKey<'s> {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TomlKey<'s>,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.decoded, &other.decoded) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    ::core::cmp::PartialOrd::partial_cmp(&self.encoding, &other.encoding)
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl<'s> ::core::cmp::Ord for TomlKey<'s> {
        #[inline]
        fn cmp(&self, other: &TomlKey<'s>) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.decoded, &other.decoded) {
                ::core::cmp::Ordering::Equal => {
                    ::core::cmp::Ord::cmp(&self.encoding, &other.encoding)
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl<'s> ::core::hash::Hash for TomlKey<'s> {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            ::core::hash::Hash::hash(&self.decoded, state);
            ::core::hash::Hash::hash(&self.encoding, state)
        }
    }
    #[automatically_derived]
    impl<'s> ::core::fmt::Debug for TomlKey<'s> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TomlKey",
                "decoded",
                &self.decoded,
                "encoding",
                &&self.encoding,
            )
        }
    }
    impl crate::WriteTomlKey for TomlKey<'_> {
        fn write_toml_key<W: crate::TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            let newline = false;
            write_toml_value(self.decoded, self.encoding, newline, writer)
        }
    }
    #[repr(u8)]
    #[allow(clippy::enum_variant_names)]
    enum Encoding {
        LiteralString,
        BasicString,
        MlLiteralString,
        MlBasicString,
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::marker::Copy for Encoding {}
    #[automatically_derived]
    #[doc(hidden)]
    #[allow(clippy::enum_variant_names)]
    unsafe impl ::core::clone::TrivialClone for Encoding {}
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::clone::Clone for Encoding {
        #[inline]
        fn clone(&self) -> Encoding {
            *self
        }
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::marker::StructuralPartialEq for Encoding {}
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::cmp::PartialEq for Encoding {
        #[inline]
        fn eq(&self, other: &Encoding) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::cmp::Eq for Encoding {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::cmp::PartialOrd for Encoding {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Encoding,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr)
        }
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::cmp::Ord for Encoding {
        #[inline]
        fn cmp(&self, other: &Encoding) -> ::core::cmp::Ordering {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr)
        }
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::hash::Hash for Encoding {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            ::core::hash::Hash::hash(&__self_discr, state)
        }
    }
    #[automatically_derived]
    #[allow(clippy::enum_variant_names)]
    impl ::core::fmt::Debug for Encoding {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Encoding::LiteralString => "LiteralString",
                    Encoding::BasicString => "BasicString",
                    Encoding::MlLiteralString => "MlLiteralString",
                    Encoding::MlBasicString => "MlBasicString",
                },
            )
        }
    }
    impl Encoding {}
    fn write_toml_value<W: crate::TomlWrite + ?Sized>(
        decoded: &str,
        encoding: Option<Encoding>,
        newline: bool,
        writer: &mut W,
    ) -> core::fmt::Result {
        let delimiter = match encoding {
            Some(Encoding::LiteralString) => "'",
            Some(Encoding::BasicString) => "\"",
            Some(Encoding::MlLiteralString) => "'''",
            Some(Encoding::MlBasicString) => "\"\"\"",
            None => "",
        };
        let escaped = match encoding {
            Some(Encoding::LiteralString) | Some(Encoding::MlLiteralString) => false,
            Some(Encoding::BasicString) | Some(Encoding::MlBasicString) => true,
            None => false,
        };
        let is_ml = match encoding {
            Some(Encoding::LiteralString) | Some(Encoding::BasicString) => false,
            Some(Encoding::MlLiteralString) | Some(Encoding::MlBasicString) => true,
            None => false,
        };
        let newline_prefix = newline && is_ml;
        writer.write_fmt(format_args!("{0}", delimiter))?;
        if newline_prefix {
            writer.newline()?;
        }
        if escaped {
            let max_seq_double_quotes = if is_ml { 2 } else { 0 };
            let mut stream = decoded;
            while !stream.is_empty() {
                let mut unescaped_end = 0;
                let mut escaped = None;
                let mut seq_double_quotes = 0;
                for (i, b) in stream.as_bytes().iter().enumerate() {
                    if *b == b'"' {
                        seq_double_quotes += 1;
                        if max_seq_double_quotes < seq_double_quotes {
                            escaped = Some(r#"\""#);
                            break;
                        }
                    } else {
                        seq_double_quotes = 0;
                    }
                    match *b {
                        0x8 => {
                            escaped = Some(r#"\b"#);
                            break;
                        }
                        0x9 => {
                            escaped = Some(r#"\t"#);
                            break;
                        }
                        0xa => {
                            if !is_ml {
                                escaped = Some(r#"\n"#);
                                break;
                            }
                        }
                        0xc => {
                            escaped = Some(r#"\f"#);
                            break;
                        }
                        0xd => {
                            escaped = Some(r#"\r"#);
                            break;
                        }
                        0x22 => {}
                        0x5c => {
                            escaped = Some(r#"\\"#);
                            break;
                        }
                        c if c <= 0x1f || c == 0x7f => {
                            break;
                        }
                        _ => {}
                    }
                    unescaped_end = i + 1;
                }
                let unescaped = &stream[0..unescaped_end];
                let escaped_str = escaped.unwrap_or("");
                let end = unescaped_end + if escaped.is_some() { 1 } else { 0 };
                stream = &stream[end..];
                writer.write_fmt(format_args!("{0}{1}", unescaped, escaped_str))?;
                if escaped.is_none() && !stream.is_empty() {
                    let b = stream.as_bytes().first().unwrap();
                    writer.write_fmt(format_args!("\\u{0:04X}", *b as u32))?;
                    stream = &stream[1..];
                }
            }
        } else {
            writer.write_fmt(format_args!("{0}", decoded))?;
        }
        writer.write_fmt(format_args!("{0}", delimiter))?;
        Ok(())
    }
    struct ValueMetrics {
        max_seq_single_quotes: u8,
        max_seq_double_quotes: u8,
        escape_codes: bool,
        escape: bool,
        newline: bool,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for ValueMetrics {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for ValueMetrics {}
    #[automatically_derived]
    impl ::core::clone::Clone for ValueMetrics {
        #[inline]
        fn clone(&self) -> ValueMetrics {
            let _: ::core::clone::AssertParamIsClone<u8>;
            let _: ::core::clone::AssertParamIsClone<bool>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ValueMetrics {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "ValueMetrics",
                "max_seq_single_quotes",
                &self.max_seq_single_quotes,
                "max_seq_double_quotes",
                &self.max_seq_double_quotes,
                "escape_codes",
                &self.escape_codes,
                "escape",
                &self.escape,
                "newline",
                &&self.newline,
            )
        }
    }
    impl ValueMetrics {
        fn new() -> Self {
            Self {
                max_seq_single_quotes: 0,
                max_seq_double_quotes: 0,
                escape_codes: false,
                escape: false,
                newline: false,
            }
        }
        fn calculate(s: &str) -> Self {
            let mut metrics = Self::new();
            let mut prev_single_quotes = 0;
            let mut prev_double_quotes = 0;
            for byte in s.as_bytes() {
                if *byte == b'\'' {
                    prev_single_quotes += 1;
                    metrics.max_seq_single_quotes = metrics
                        .max_seq_single_quotes
                        .max(prev_single_quotes);
                } else {
                    prev_single_quotes = 0;
                }
                if *byte == b'"' {
                    prev_double_quotes += 1;
                    metrics.max_seq_double_quotes = metrics
                        .max_seq_double_quotes
                        .max(prev_double_quotes);
                } else {
                    prev_double_quotes = 0;
                }
                match *byte {
                    b'\\' => metrics.escape = true,
                    b'\t' => {}
                    b'\n' => metrics.newline = true,
                    c if c <= 0x1f || c == 0x7f => metrics.escape_codes = true,
                    _ => {}
                }
            }
            metrics
        }
    }
    struct KeyMetrics {
        unquoted: bool,
        single_quotes: bool,
        double_quotes: bool,
        escape_codes: bool,
        escape: bool,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for KeyMetrics {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for KeyMetrics {}
    #[automatically_derived]
    impl ::core::clone::Clone for KeyMetrics {
        #[inline]
        fn clone(&self) -> KeyMetrics {
            let _: ::core::clone::AssertParamIsClone<bool>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for KeyMetrics {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "KeyMetrics",
                "unquoted",
                &self.unquoted,
                "single_quotes",
                &self.single_quotes,
                "double_quotes",
                &self.double_quotes,
                "escape_codes",
                &self.escape_codes,
                "escape",
                &&self.escape,
            )
        }
    }
    impl KeyMetrics {
        fn new() -> Self {
            Self {
                unquoted: true,
                single_quotes: false,
                double_quotes: false,
                escape_codes: false,
                escape: false,
            }
        }
        fn calculate(s: &str) -> Self {
            let mut metrics = Self::new();
            metrics.unquoted = !s.is_empty();
            for byte in s.as_bytes() {
                if !#[allow(non_exhaustive_omitted_patterns)]
                match *byte {
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' => true,
                    _ => false,
                } {
                    metrics.unquoted = false;
                }
                match *byte {
                    b'\'' => metrics.single_quotes = true,
                    b'"' => metrics.double_quotes = true,
                    b'\\' => metrics.escape = true,
                    b'\t' => {}
                    c if c <= 0x1f || c == 0x7f => metrics.escape_codes = true,
                    _ => {}
                }
            }
            metrics
        }
    }
}
mod value {
    use alloc::borrow::Cow;
    use alloc::string::String;
    use alloc::vec::Vec;
    use crate::TomlWrite;
    use crate::WriteTomlKey;
    pub trait ToTomlValue {
        fn to_toml_value(&self) -> String;
    }
    impl<T> ToTomlValue for T
    where
        T: WriteTomlValue + ?Sized,
    {
        fn to_toml_value(&self) -> String {
            let mut result = String::new();
            let _ = self.write_toml_value(&mut result);
            result
        }
    }
    pub trait WriteTomlValue {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result;
    }
    impl WriteTomlValue for bool {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for u8 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for i8 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for u16 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for i16 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for u32 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for i32 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for u64 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for i64 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for u128 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for i128 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.write_fmt(format_args!("{0}", self))
        }
    }
    impl WriteTomlValue for f32 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            match (self.is_sign_negative(), self.is_nan(), *self == 0.0) {
                (true, true, _) => writer.write_fmt(format_args!("-nan")),
                (false, true, _) => writer.write_fmt(format_args!("nan")),
                (true, false, true) => writer.write_fmt(format_args!("-0.0")),
                (false, false, true) => writer.write_fmt(format_args!("0.0")),
                (_, false, false) => {
                    if self % 1.0 == 0.0 {
                        writer.write_fmt(format_args!("{0}.0", self))
                    } else {
                        writer.write_fmt(format_args!("{0}", self))
                    }
                }
            }
        }
    }
    impl WriteTomlValue for f64 {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            match (self.is_sign_negative(), self.is_nan(), *self == 0.0) {
                (true, true, _) => writer.write_fmt(format_args!("-nan")),
                (false, true, _) => writer.write_fmt(format_args!("nan")),
                (true, false, true) => writer.write_fmt(format_args!("-0.0")),
                (false, false, true) => writer.write_fmt(format_args!("0.0")),
                (_, false, false) => {
                    if self % 1.0 == 0.0 {
                        writer.write_fmt(format_args!("{0}.0", self))
                    } else {
                        writer.write_fmt(format_args!("{0}", self))
                    }
                }
            }
        }
    }
    impl WriteTomlValue for char {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            let mut buf = [0; 4];
            let v = self.encode_utf8(&mut buf);
            v.write_toml_value(writer)
        }
    }
    impl WriteTomlValue for str {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            crate::TomlStringBuilder::new(self).as_default().write_toml_value(writer)
        }
    }
    impl WriteTomlValue for String {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_str().write_toml_value(writer)
        }
    }
    impl WriteTomlValue for Cow<'_, str> {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_ref().write_toml_value(writer)
        }
    }
    impl<V: WriteTomlValue> WriteTomlValue for [V] {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            writer.open_array()?;
            let mut iter = self.iter();
            if let Some(v) = iter.next() {
                writer.value(v)?;
            }
            for v in iter {
                writer.val_sep()?;
                writer.space()?;
                writer.value(v)?;
            }
            writer.close_array()?;
            Ok(())
        }
    }
    impl<V: WriteTomlValue, const N: usize> WriteTomlValue for [V; N] {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_slice().write_toml_value(writer)
        }
    }
    impl<V: WriteTomlValue> WriteTomlValue for Vec<V> {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            self.as_slice().write_toml_value(writer)
        }
    }
    impl<K: WriteTomlKey, V: WriteTomlValue> WriteTomlValue
    for alloc::collections::BTreeMap<K, V> {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            write_toml_inline_table(self.iter(), writer)
        }
    }
    impl<K: WriteTomlKey, V: WriteTomlValue> WriteTomlValue
    for std::collections::HashMap<K, V> {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            write_toml_inline_table(self.iter(), writer)
        }
    }
    fn write_toml_inline_table<
        'i,
        I: Iterator<Item = (&'i K, &'i V)>,
        K: WriteTomlKey + 'i,
        V: WriteTomlValue + 'i,
        W: TomlWrite + ?Sized,
    >(mut iter: I, writer: &mut W) -> core::fmt::Result {
        writer.open_inline_table()?;
        let mut trailing_space = false;
        if let Some((key, value)) = iter.next() {
            writer.space()?;
            writer.key(key)?;
            writer.space()?;
            writer.keyval_sep()?;
            writer.space()?;
            writer.value(value)?;
            trailing_space = true;
        }
        for (key, value) in iter {
            writer.val_sep()?;
            writer.space()?;
            writer.key(key)?;
            writer.space()?;
            writer.keyval_sep()?;
            writer.space()?;
            writer.value(value)?;
        }
        if trailing_space {
            writer.space()?;
        }
        writer.close_inline_table()?;
        Ok(())
    }
    impl<V: WriteTomlValue + ?Sized> WriteTomlValue for &V {
        fn write_toml_value<W: TomlWrite + ?Sized>(
            &self,
            writer: &mut W,
        ) -> core::fmt::Result {
            (*self).write_toml_value(writer)
        }
    }
}
mod write {
    pub trait TomlWrite: core::fmt::Write {
        fn open_table_header(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("["))
        }
        fn close_table_header(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("]"))
        }
        fn open_array_of_tables_header(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("[["))
        }
        fn close_array_of_tables_header(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("]]"))
        }
        fn open_inline_table(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("{{"))
        }
        fn close_inline_table(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("}}"))
        }
        fn open_array(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("["))
        }
        fn close_array(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("]"))
        }
        fn key_sep(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("."))
        }
        fn keyval_sep(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("="))
        }
        /// Write an encoded TOML key
        ///
        /// To customize the encoding, see [`TomlStringBuilder`][crate::TomlStringBuilder].
        fn key(&mut self, value: impl crate::WriteTomlKey) -> core::fmt::Result {
            value.write_toml_key(self)
        }
        /// Write an encoded TOML scalar value
        ///
        /// To customize the encoding, see
        /// - [`TomlStringBuilder`][crate::TomlStringBuilder]
        /// - [`TomlIntegerFormat`][crate::TomlIntegerFormat]
        ///
        /// <div class="warning">
        ///
        /// For floats, this preserves the sign bit for [`f32::NAN`] / [`f64::NAN`] for the sake of
        /// format-preserving editing.
        /// However, in most cases the sign bit is indeterminate and outputting signed NANs can be a
        /// cause of non-repeatable behavior.
        ///
        /// For general serialization, you should discard the sign bit.  For example:
        /// ```
        /// # let mut v = f64::NAN;
        /// if v.is_nan() {
        ///     v = v.copysign(1.0);
        /// }
        /// ```
        ///
        /// </div>
        fn value(&mut self, value: impl crate::WriteTomlValue) -> core::fmt::Result {
            value.write_toml_value(self)
        }
        fn val_sep(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!(","))
        }
        fn space(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!(" "))
        }
        fn open_comment(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("#"))
        }
        fn newline(&mut self) -> core::fmt::Result {
            self.write_fmt(format_args!("\n"))
        }
    }
    impl<W> TomlWrite for W
    where
        W: core::fmt::Write,
    {}
}
pub use integer::TomlInteger;
pub use integer::TomlIntegerFormat;
pub use key::ToTomlKey;
pub use key::WriteTomlKey;
pub use string::TomlKey;
pub use string::TomlKeyBuilder;
pub use string::TomlString;
pub use string::TomlStringBuilder;
pub use value::ToTomlValue;
pub use value::WriteTomlValue;
pub use write::TomlWrite;
