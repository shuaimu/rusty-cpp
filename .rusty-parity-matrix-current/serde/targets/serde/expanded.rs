#![feature(prelude_import)]
//! # Serde
//!
//! Serde is a framework for ***ser***ializing and ***de***serializing Rust data
//! structures efficiently and generically.
//!
//! The Serde ecosystem consists of data structures that know how to serialize
//! and deserialize themselves along with data formats that know how to
//! serialize and deserialize other things. Serde provides the layer by which
//! these two groups interact with each other, allowing any supported data
//! structure to be serialized and deserialized using any supported data format.
//!
//! See the Serde website <https://serde.rs> for additional documentation and
//! usage examples.
//!
//! ## Design
//!
//! Where many other languages rely on runtime reflection for serializing data,
//! Serde is instead built on Rust's powerful trait system. A data structure
//! that knows how to serialize and deserialize itself is one that implements
//! Serde's `Serialize` and `Deserialize` traits (or uses Serde's derive
//! attribute to automatically generate implementations at compile time). This
//! avoids any overhead of reflection or runtime type information. In fact in
//! many situations the interaction between data structure and data format can
//! be completely optimized away by the Rust compiler, leaving Serde
//! serialization to perform the same speed as a handwritten serializer for the
//! specific selection of data structure and data format.
//!
//! ## Data formats
//!
//! The following is a partial list of data formats that have been implemented
//! for Serde by the community.
//!
//! - [JSON], the ubiquitous JavaScript Object Notation used by many HTTP APIs.
//! - [Postcard], a no\_std and embedded-systems friendly compact binary format.
//! - [CBOR], a Concise Binary Object Representation designed for small message
//!   size without the need for version negotiation.
//! - [YAML], a self-proclaimed human-friendly configuration language that ain't
//!   markup language.
//! - [MessagePack], an efficient binary format that resembles a compact JSON.
//! - [TOML], a minimal configuration format used by [Cargo].
//! - [Pickle], a format common in the Python world.
//! - [RON], a Rusty Object Notation.
//! - [BSON], the data storage and network transfer format used by MongoDB.
//! - [Avro], a binary format used within Apache Hadoop, with support for schema
//!   definition.
//! - [JSON5], a superset of JSON including some productions from ES5.
//! - [URL] query strings, in the x-www-form-urlencoded format.
//! - [Starlark], the format used for describing build targets by the Bazel and
//!   Buck build systems. *(serialization only)*
//! - [Envy], a way to deserialize environment variables into Rust structs.
//!   *(deserialization only)*
//! - [Envy Store], a way to deserialize [AWS Parameter Store] parameters into
//!   Rust structs. *(deserialization only)*
//! - [S-expressions], the textual representation of code and data used by the
//!   Lisp language family.
//! - [D-Bus]'s binary wire format.
//! - [FlexBuffers], the schemaless cousin of Google's FlatBuffers zero-copy
//!   serialization format.
//! - [Bencode], a simple binary format used in the BitTorrent protocol.
//! - [Token streams], for processing Rust procedural macro input.
//!   *(deserialization only)*
//! - [DynamoDB Items], the format used by [rusoto_dynamodb] to transfer data to
//!   and from DynamoDB.
//! - [Hjson], a syntax extension to JSON designed around human reading and
//!   editing. *(deserialization only)*
//! - [CSV], Comma-separated values is a tabular text file format.
//!
//! [JSON]: https://github.com/serde-rs/json
//! [Postcard]: https://github.com/jamesmunns/postcard
//! [CBOR]: https://github.com/enarx/ciborium
//! [YAML]: https://github.com/dtolnay/serde-yaml
//! [MessagePack]: https://github.com/3Hren/msgpack-rust
//! [TOML]: https://docs.rs/toml
//! [Pickle]: https://github.com/birkenfeld/serde-pickle
//! [RON]: https://github.com/ron-rs/ron
//! [BSON]: https://github.com/mongodb/bson-rust
//! [Avro]: https://docs.rs/apache-avro
//! [JSON5]: https://github.com/callum-oakley/json5-rs
//! [URL]: https://docs.rs/serde_qs
//! [Starlark]: https://github.com/dtolnay/serde-starlark
//! [Envy]: https://github.com/softprops/envy
//! [Envy Store]: https://github.com/softprops/envy-store
//! [Cargo]: https://doc.rust-lang.org/cargo/reference/manifest.html
//! [AWS Parameter Store]: https://docs.aws.amazon.com/systems-manager/latest/userguide/systems-manager-parameter-store.html
//! [S-expressions]: https://github.com/rotty/lexpr-rs
//! [D-Bus]: https://docs.rs/zvariant
//! [FlexBuffers]: https://github.com/google/flatbuffers/tree/master/rust/flexbuffers
//! [Bencode]: https://github.com/P3KI/bendy
//! [Token streams]: https://github.com/oxidecomputer/serde_tokenstream
//! [DynamoDB Items]: https://docs.rs/serde_dynamo
//! [rusoto_dynamodb]: https://docs.rs/rusoto_dynamodb
//! [Hjson]: https://github.com/Canop/deser-hjson
//! [CSV]: https://docs.rs/csv
#![doc(html_root_url = "https://docs.rs/serde/1.0.228")]
#![allow(unknown_lints, bare_trait_objects, deprecated, mismatched_lifetime_syntaxes)]
#![allow(
    clippy::unnested_or_patterns,
    clippy::semicolon_if_nothing_returned,
    clippy::empty_enum,
    clippy::type_repetition_in_bounds,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::module_name_repetitions,
    clippy::single_match_else,
    clippy::type_complexity,
    clippy::use_self,
    clippy::zero_prefixed_literal,
    clippy::derive_partial_eq_without_eq,
    clippy::enum_glob_use,
    clippy::explicit_auto_deref,
    clippy::incompatible_msrv,
    clippy::let_underscore_untyped,
    clippy::map_err_ignore,
    clippy::new_without_default,
    clippy::result_unit_err,
    clippy::wildcard_imports,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::elidable_lifetime_names,
    clippy::needless_lifetimes,
    clippy::unseparated_literal_suffix,
    clippy::needless_doctest_main,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
)]
#![deny(clippy::question_mark_used)]
#![deny(missing_docs, unused_imports)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
/// A facade around all the types we need from the `std`, `core`, and `alloc`
/// crates. This avoids elaborate import wrangling having to happen in every
/// module.
mod lib {
    mod core {
        pub use std::*;
    }
    pub use self::core::{f32, f64};
    pub use self::core::{ptr, str};
    pub use self::core::slice;
    pub use self::core::clone;
    pub use self::core::convert;
    pub use self::core::default;
    pub use self::core::fmt::{self, Debug, Display, Write as FmtWrite};
    pub use self::core::marker::{self, PhantomData};
    pub use self::core::option;
    pub use self::core::result;
    pub use std::borrow::{Cow, ToOwned};
    pub use std::string::{String, ToString};
    pub use std::vec::Vec;
    pub use std::boxed::Box;
}
pub use serde_core::{
    de, forward_to_deserialize_any, ser, Deserialize, Deserializer, Serialize, Serializer,
};
#[doc(hidden)]
mod private {
    pub mod de {
        use crate::lib::*;
        use crate::de::value::{BorrowedBytesDeserializer, BytesDeserializer};
        use crate::de::{
            Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error,
            IntoDeserializer, VariantAccess, Visitor,
        };
        use crate::de::{MapAccess, Unexpected};
        pub use self::content::{
            content_as_str, Content, ContentDeserializer, ContentRefDeserializer,
            ContentVisitor, EnumDeserializer, InternallyTaggedUnitVisitor,
            TagContentOtherField, TagContentOtherFieldVisitor, TagOrContentField,
            TagOrContentFieldVisitor, TaggedContentVisitor, UntaggedUnitVisitor,
        };
        pub use crate::serde_core_private::InPlaceSeed;
        /// If the missing field is of type `Option<T>` then treat is as `None`,
        /// otherwise it is an error.
        pub fn missing_field<'de, V, E>(field: &'static str) -> Result<V, E>
        where
            V: Deserialize<'de>,
            E: Error,
        {
            struct MissingFieldDeserializer<E>(&'static str, PhantomData<E>);
            #[diagnostic::do_not_recommend]
            impl<'de, E> Deserializer<'de> for MissingFieldDeserializer<E>
            where
                E: Error,
            {
                type Error = E;
                fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    Err(Error::missing_field(self.0))
                }
                fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    visitor.visit_none()
                }
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_seq<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
            }
            let deserializer = MissingFieldDeserializer(field, PhantomData);
            Deserialize::deserialize(deserializer)
        }
        pub fn borrow_cow_str<'de: 'a, 'a, D, R>(deserializer: D) -> Result<R, D::Error>
        where
            D: Deserializer<'de>,
            R: From<Cow<'a, str>>,
        {
            struct CowStrVisitor;
            #[diagnostic::do_not_recommend]
            impl<'a> Visitor<'a> for CowStrVisitor {
                type Value = Cow<'a, str>;
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a string")
                }
                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v.to_owned()))
                }
                fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Borrowed(v))
                }
                fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v))
                }
                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    match str::from_utf8(v) {
                        Ok(s) => Ok(Cow::Owned(s.to_owned())),
                        Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                    }
                }
                fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    match str::from_utf8(v) {
                        Ok(s) => Ok(Cow::Borrowed(s)),
                        Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                    }
                }
                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    match String::from_utf8(v) {
                        Ok(s) => Ok(Cow::Owned(s)),
                        Err(e) => {
                            Err(
                                Error::invalid_value(
                                    Unexpected::Bytes(&e.into_bytes()),
                                    &self,
                                ),
                            )
                        }
                    }
                }
            }
            deserializer.deserialize_str(CowStrVisitor).map(From::from)
        }
        pub fn borrow_cow_bytes<'de: 'a, 'a, D, R>(
            deserializer: D,
        ) -> Result<R, D::Error>
        where
            D: Deserializer<'de>,
            R: From<Cow<'a, [u8]>>,
        {
            struct CowBytesVisitor;
            #[diagnostic::do_not_recommend]
            impl<'a> Visitor<'a> for CowBytesVisitor {
                type Value = Cow<'a, [u8]>;
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a byte array")
                }
                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v.as_bytes().to_vec()))
                }
                fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Borrowed(v.as_bytes()))
                }
                fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v.into_bytes()))
                }
                fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v.to_vec()))
                }
                fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Borrowed(v))
                }
                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    Ok(Cow::Owned(v))
                }
            }
            deserializer.deserialize_bytes(CowBytesVisitor).map(From::from)
        }
        mod content {
            use crate::lib::*;
            use crate::de::{
                self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Expected,
                IgnoredAny, MapAccess, SeqAccess, Unexpected, Visitor,
            };
            use crate::serde_core_private::size_hint;
            pub use crate::serde_core_private::Content;
            pub fn content_as_str<'a, 'de>(
                content: &'a Content<'de>,
            ) -> Option<&'a str> {
                match *content {
                    Content::Str(x) => Some(x),
                    Content::String(ref x) => Some(x),
                    Content::Bytes(x) => str::from_utf8(x).ok(),
                    Content::ByteBuf(ref x) => str::from_utf8(x).ok(),
                    _ => None,
                }
            }
            fn content_clone<'de>(content: &Content<'de>) -> Content<'de> {
                match content {
                    Content::Bool(b) => Content::Bool(*b),
                    Content::U8(n) => Content::U8(*n),
                    Content::U16(n) => Content::U16(*n),
                    Content::U32(n) => Content::U32(*n),
                    Content::U64(n) => Content::U64(*n),
                    Content::I8(n) => Content::I8(*n),
                    Content::I16(n) => Content::I16(*n),
                    Content::I32(n) => Content::I32(*n),
                    Content::I64(n) => Content::I64(*n),
                    Content::F32(f) => Content::F32(*f),
                    Content::F64(f) => Content::F64(*f),
                    Content::Char(c) => Content::Char(*c),
                    Content::String(s) => Content::String(s.clone()),
                    Content::Str(s) => Content::Str(*s),
                    Content::ByteBuf(b) => Content::ByteBuf(b.clone()),
                    Content::Bytes(b) => Content::Bytes(b),
                    Content::None => Content::None,
                    Content::Some(content) => {
                        Content::Some(Box::new(content_clone(content)))
                    }
                    Content::Unit => Content::Unit,
                    Content::Newtype(content) => {
                        Content::Newtype(Box::new(content_clone(content)))
                    }
                    Content::Seq(seq) => {
                        Content::Seq(seq.iter().map(content_clone).collect())
                    }
                    Content::Map(map) => {
                        Content::Map(
                            map
                                .iter()
                                .map(|(k, v)| (content_clone(k), content_clone(v)))
                                .collect(),
                        )
                    }
                }
            }
            #[cold]
            fn content_unexpected<'a, 'de>(content: &'a Content<'de>) -> Unexpected<'a> {
                match *content {
                    Content::Bool(b) => Unexpected::Bool(b),
                    Content::U8(n) => Unexpected::Unsigned(n as u64),
                    Content::U16(n) => Unexpected::Unsigned(n as u64),
                    Content::U32(n) => Unexpected::Unsigned(n as u64),
                    Content::U64(n) => Unexpected::Unsigned(n),
                    Content::I8(n) => Unexpected::Signed(n as i64),
                    Content::I16(n) => Unexpected::Signed(n as i64),
                    Content::I32(n) => Unexpected::Signed(n as i64),
                    Content::I64(n) => Unexpected::Signed(n),
                    Content::F32(f) => Unexpected::Float(f as f64),
                    Content::F64(f) => Unexpected::Float(f),
                    Content::Char(c) => Unexpected::Char(c),
                    Content::String(ref s) => Unexpected::Str(s),
                    Content::Str(s) => Unexpected::Str(s),
                    Content::ByteBuf(ref b) => Unexpected::Bytes(b),
                    Content::Bytes(b) => Unexpected::Bytes(b),
                    Content::None | Content::Some(_) => Unexpected::Option,
                    Content::Unit => Unexpected::Unit,
                    Content::Newtype(_) => Unexpected::NewtypeStruct,
                    Content::Seq(_) => Unexpected::Seq,
                    Content::Map(_) => Unexpected::Map,
                }
            }
            pub struct ContentVisitor<'de> {
                value: PhantomData<Content<'de>>,
            }
            impl<'de> ContentVisitor<'de> {
                pub fn new() -> Self {
                    ContentVisitor {
                        value: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> DeserializeSeed<'de> for ContentVisitor<'de> {
                type Value = Content<'de>;
                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.__deserialize_content_v1(self)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> Visitor<'de> for ContentVisitor<'de> {
                type Value = Content<'de>;
                fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                    fmt.write_str("any value")
                }
                fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::Bool(value))
                }
                fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::I8(value))
                }
                fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::I16(value))
                }
                fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::I32(value))
                }
                fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::I64(value))
                }
                fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::U8(value))
                }
                fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::U16(value))
                }
                fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::U32(value))
                }
                fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::U64(value))
                }
                fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::F32(value))
                }
                fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::F64(value))
                }
                fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::Char(value))
                }
                fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::String(value.into()))
                }
                fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::Str(value))
                }
                fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::String(value))
                }
                fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::ByteBuf(value.into()))
                }
                fn visit_borrowed_bytes<F>(
                    self,
                    value: &'de [u8],
                ) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::Bytes(value))
                }
                fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::ByteBuf(value))
                }
                fn visit_unit<F>(self) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::Unit)
                }
                fn visit_none<F>(self) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    Ok(Content::None)
                }
                fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let v = match ContentVisitor::new().deserialize(deserializer) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(Content::Some(Box::new(v)))
                }
                fn visit_newtype_struct<D>(
                    self,
                    deserializer: D,
                ) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let v = match ContentVisitor::new().deserialize(deserializer) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(Content::Newtype(Box::new(v)))
                }
                fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    let mut vec = Vec::<
                        Content,
                    >::with_capacity(
                        size_hint::cautious::<Content>(visitor.size_hint()),
                    );
                    while let Some(e) = match visitor
                        .next_element_seed(ContentVisitor::new())
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    } {
                        vec.push(e);
                    }
                    Ok(Content::Seq(vec))
                }
                fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut vec = Vec::<
                        (Content, Content),
                    >::with_capacity(
                        size_hint::cautious::<(Content, Content)>(visitor.size_hint()),
                    );
                    while let Some(kv) = match visitor
                        .next_entry_seed(ContentVisitor::new(), ContentVisitor::new())
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    } {
                        vec.push(kv);
                    }
                    Ok(Content::Map(vec))
                }
                fn visit_enum<V>(self, _visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: EnumAccess<'de>,
                {
                    Err(
                        de::Error::custom(
                            "untagged and internally tagged enums do not support enum input",
                        ),
                    )
                }
            }
            /// This is the type of the map keys in an internally tagged enum.
            ///
            /// Not public API.
            pub enum TagOrContent<'de> {
                Tag,
                Content(Content<'de>),
            }
            /// Serves as a seed for deserializing a key of internally tagged enum.
            /// Cannot capture externally tagged enums, `i128` and `u128`.
            struct TagOrContentVisitor<'de> {
                name: &'static str,
                value: PhantomData<TagOrContent<'de>>,
            }
            impl<'de> TagOrContentVisitor<'de> {
                fn new(name: &'static str) -> Self {
                    TagOrContentVisitor {
                        name,
                        value: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> DeserializeSeed<'de> for TagOrContentVisitor<'de> {
                type Value = TagOrContent<'de>;
                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_any(self)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> Visitor<'de> for TagOrContentVisitor<'de> {
                type Value = TagOrContent<'de>;
                fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                    fmt.write_fmt(
                        format_args!("a type tag `{0}` or any other value", self.name),
                    )
                }
                fn visit_bool<F>(self, value: bool) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_bool(value).map(TagOrContent::Content)
                }
                fn visit_i8<F>(self, value: i8) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_i8(value).map(TagOrContent::Content)
                }
                fn visit_i16<F>(self, value: i16) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_i16(value).map(TagOrContent::Content)
                }
                fn visit_i32<F>(self, value: i32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_i32(value).map(TagOrContent::Content)
                }
                fn visit_i64<F>(self, value: i64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_i64(value).map(TagOrContent::Content)
                }
                fn visit_u8<F>(self, value: u8) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_u8(value).map(TagOrContent::Content)
                }
                fn visit_u16<F>(self, value: u16) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_u16(value).map(TagOrContent::Content)
                }
                fn visit_u32<F>(self, value: u32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_u32(value).map(TagOrContent::Content)
                }
                fn visit_u64<F>(self, value: u64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_u64(value).map(TagOrContent::Content)
                }
                fn visit_f32<F>(self, value: f32) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_f32(value).map(TagOrContent::Content)
                }
                fn visit_f64<F>(self, value: f64) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_f64(value).map(TagOrContent::Content)
                }
                fn visit_char<F>(self, value: char) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_char(value).map(TagOrContent::Content)
                }
                fn visit_str<F>(self, value: &str) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new().visit_str(value).map(TagOrContent::Content)
                    }
                }
                fn visit_borrowed_str<F>(self, value: &'de str) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new()
                            .visit_borrowed_str(value)
                            .map(TagOrContent::Content)
                    }
                }
                fn visit_string<F>(self, value: String) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new()
                            .visit_string(value)
                            .map(TagOrContent::Content)
                    }
                }
                fn visit_bytes<F>(self, value: &[u8]) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name.as_bytes() {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new()
                            .visit_bytes(value)
                            .map(TagOrContent::Content)
                    }
                }
                fn visit_borrowed_bytes<F>(
                    self,
                    value: &'de [u8],
                ) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name.as_bytes() {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new()
                            .visit_borrowed_bytes(value)
                            .map(TagOrContent::Content)
                    }
                }
                fn visit_byte_buf<F>(self, value: Vec<u8>) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    if value == self.name.as_bytes() {
                        Ok(TagOrContent::Tag)
                    } else {
                        ContentVisitor::new()
                            .visit_byte_buf(value)
                            .map(TagOrContent::Content)
                    }
                }
                fn visit_unit<F>(self) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_unit().map(TagOrContent::Content)
                }
                fn visit_none<F>(self) -> Result<Self::Value, F>
                where
                    F: de::Error,
                {
                    ContentVisitor::new().visit_none().map(TagOrContent::Content)
                }
                fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    ContentVisitor::new()
                        .visit_some(deserializer)
                        .map(TagOrContent::Content)
                }
                fn visit_newtype_struct<D>(
                    self,
                    deserializer: D,
                ) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    ContentVisitor::new()
                        .visit_newtype_struct(deserializer)
                        .map(TagOrContent::Content)
                }
                fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    ContentVisitor::new().visit_seq(visitor).map(TagOrContent::Content)
                }
                fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    ContentVisitor::new().visit_map(visitor).map(TagOrContent::Content)
                }
                fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: EnumAccess<'de>,
                {
                    ContentVisitor::new().visit_enum(visitor).map(TagOrContent::Content)
                }
            }
            /// Used by generated code to deserialize an internally tagged enum.
            ///
            /// Captures map or sequence from the original deserializer and searches
            /// a tag in it (in case of sequence, tag is the first element of sequence).
            ///
            /// Not public API.
            pub struct TaggedContentVisitor<T> {
                tag_name: &'static str,
                expecting: &'static str,
                value: PhantomData<T>,
            }
            impl<T> TaggedContentVisitor<T> {
                /// Visitor for the content of an internally tagged enum with the given
                /// tag name.
                pub fn new(name: &'static str, expecting: &'static str) -> Self {
                    TaggedContentVisitor {
                        tag_name: name,
                        expecting,
                        value: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, T> Visitor<'de> for TaggedContentVisitor<T>
            where
                T: Deserialize<'de>,
            {
                type Value = (T, Content<'de>);
                fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                    fmt.write_str(self.expecting)
                }
                fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
                where
                    S: SeqAccess<'de>,
                {
                    let tag = match match seq.next_element() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    } {
                        Some(tag) => tag,
                        None => {
                            return Err(de::Error::missing_field(self.tag_name));
                        }
                    };
                    let rest = de::value::SeqAccessDeserializer::new(seq);
                    Ok((
                        tag,
                        match ContentVisitor::new().deserialize(rest) {
                            Ok(val) => val,
                            Err(err) => return Err(err),
                        },
                    ))
                }
                fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                where
                    M: MapAccess<'de>,
                {
                    let mut tag = None;
                    let mut vec = Vec::<
                        (Content, Content),
                    >::with_capacity(
                        size_hint::cautious::<(Content, Content)>(map.size_hint()),
                    );
                    while let Some(k) = match map
                        .next_key_seed(TagOrContentVisitor::new(self.tag_name))
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    } {
                        match k {
                            TagOrContent::Tag => {
                                if tag.is_some() {
                                    return Err(de::Error::duplicate_field(self.tag_name));
                                }
                                tag = Some(
                                    match map.next_value() {
                                        Ok(val) => val,
                                        Err(err) => return Err(err),
                                    },
                                );
                            }
                            TagOrContent::Content(k) => {
                                let v = match map.next_value_seed(ContentVisitor::new()) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                                vec.push((k, v));
                            }
                        }
                    }
                    match tag {
                        None => Err(de::Error::missing_field(self.tag_name)),
                        Some(tag) => Ok((tag, Content::Map(vec))),
                    }
                }
            }
            /// Used by generated code to deserialize an adjacently tagged enum.
            ///
            /// Not public API.
            pub enum TagOrContentField {
                Tag,
                Content,
            }
            /// Not public API.
            pub struct TagOrContentFieldVisitor {
                /// Name of the tag field of the adjacently tagged enum
                pub tag: &'static str,
                /// Name of the content field of the adjacently tagged enum
                pub content: &'static str,
            }
            #[diagnostic::do_not_recommend]
            impl<'de> DeserializeSeed<'de> for TagOrContentFieldVisitor {
                type Value = TagOrContentField;
                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_identifier(self)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> Visitor<'de> for TagOrContentFieldVisitor {
                type Value = TagOrContentField;
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter
                        .write_fmt(
                            format_args!("{0:?} or {1:?}", self.tag, self.content),
                        )
                }
                fn visit_u64<E>(self, field_index: u64) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    match field_index {
                        0 => Ok(TagOrContentField::Tag),
                        1 => Ok(TagOrContentField::Content),
                        _ => {
                            Err(
                                de::Error::invalid_value(
                                    Unexpected::Unsigned(field_index),
                                    &self,
                                ),
                            )
                        }
                    }
                }
                fn visit_str<E>(self, field: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    if field == self.tag {
                        Ok(TagOrContentField::Tag)
                    } else if field == self.content {
                        Ok(TagOrContentField::Content)
                    } else {
                        Err(de::Error::invalid_value(Unexpected::Str(field), &self))
                    }
                }
                fn visit_bytes<E>(self, field: &[u8]) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    if field == self.tag.as_bytes() {
                        Ok(TagOrContentField::Tag)
                    } else if field == self.content.as_bytes() {
                        Ok(TagOrContentField::Content)
                    } else {
                        Err(de::Error::invalid_value(Unexpected::Bytes(field), &self))
                    }
                }
            }
            /// Used by generated code to deserialize an adjacently tagged enum when
            /// ignoring unrelated fields is allowed.
            ///
            /// Not public API.
            pub enum TagContentOtherField {
                Tag,
                Content,
                Other,
            }
            /// Not public API.
            pub struct TagContentOtherFieldVisitor {
                /// Name of the tag field of the adjacently tagged enum
                pub tag: &'static str,
                /// Name of the content field of the adjacently tagged enum
                pub content: &'static str,
            }
            #[diagnostic::do_not_recommend]
            impl<'de> DeserializeSeed<'de> for TagContentOtherFieldVisitor {
                type Value = TagContentOtherField;
                fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_identifier(self)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de> Visitor<'de> for TagContentOtherFieldVisitor {
                type Value = TagContentOtherField;
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter
                        .write_fmt(
                            format_args!(
                                "{0:?}, {1:?}, or other ignored fields",
                                self.tag,
                                self.content,
                            ),
                        )
                }
                fn visit_u64<E>(self, field_index: u64) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    match field_index {
                        0 => Ok(TagContentOtherField::Tag),
                        1 => Ok(TagContentOtherField::Content),
                        _ => Ok(TagContentOtherField::Other),
                    }
                }
                fn visit_str<E>(self, field: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_bytes(field.as_bytes())
                }
                fn visit_bytes<E>(self, field: &[u8]) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    if field == self.tag.as_bytes() {
                        Ok(TagContentOtherField::Tag)
                    } else if field == self.content.as_bytes() {
                        Ok(TagContentOtherField::Content)
                    } else {
                        Ok(TagContentOtherField::Other)
                    }
                }
            }
            /// Not public API
            pub struct ContentDeserializer<'de, E> {
                content: Content<'de>,
                err: PhantomData<E>,
            }
            impl<'de, E> ContentDeserializer<'de, E>
            where
                E: de::Error,
            {
                #[cold]
                fn invalid_type(self, exp: &dyn Expected) -> E {
                    de::Error::invalid_type(content_unexpected(&self.content), exp)
                }
                fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_float<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::F32(v) => visitor.visit_f32(v),
                        Content::F64(v) => visitor.visit_f64(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
            }
            fn visit_content_seq<'de, V, E>(
                content: Vec<Content<'de>>,
                visitor: V,
            ) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
                E: de::Error,
            {
                let mut seq_visitor = SeqDeserializer::new(content);
                let value = match visitor.visit_seq(&mut seq_visitor) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match seq_visitor.end() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(value)
            }
            fn visit_content_map<'de, V, E>(
                content: Vec<(Content<'de>, Content<'de>)>,
                visitor: V,
            ) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
                E: de::Error,
            {
                let mut map_visitor = MapDeserializer::new(content);
                let value = match visitor.visit_map(&mut map_visitor) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map_visitor.end() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(value)
            }
            /// Used when deserializing an internally tagged enum because the content
            /// will be used exactly once.
            #[diagnostic::do_not_recommend]
            impl<'de, E> Deserializer<'de> for ContentDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Bool(v) => visitor.visit_bool(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        Content::F32(v) => visitor.visit_f32(v),
                        Content::F64(v) => visitor.visit_f64(v),
                        Content::Char(v) => visitor.visit_char(v),
                        Content::String(v) => visitor.visit_string(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(v) => visitor.visit_byte_buf(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::Unit => visitor.visit_unit(),
                        Content::None => visitor.visit_none(),
                        Content::Some(v) => {
                            visitor.visit_some(ContentDeserializer::new(*v))
                        }
                        Content::Newtype(v) => {
                            visitor.visit_newtype_struct(ContentDeserializer::new(*v))
                        }
                        Content::Seq(v) => visit_content_seq(v, visitor),
                        Content::Map(v) => visit_content_map(v, visitor),
                    }
                }
                fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Bool(v) => visitor.visit_bool(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_float(visitor)
                }
                fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_float(visitor)
                }
                fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Char(v) => visitor.visit_char(v),
                        Content::String(v) => visitor.visit_string(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_string(visitor)
                }
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::String(v) => visitor.visit_string(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(v) => visitor.visit_byte_buf(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_byte_buf(visitor)
                }
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::String(v) => visitor.visit_string(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(v) => visitor.visit_byte_buf(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::Seq(v) => visit_content_seq(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::None => visitor.visit_none(),
                        Content::Some(v) => {
                            visitor.visit_some(ContentDeserializer::new(*v))
                        }
                        Content::Unit => visitor.visit_unit(),
                        _ => visitor.visit_some(self),
                    }
                }
                fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Unit => visitor.visit_unit(),
                        Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_unit_struct<V>(
                    self,
                    _name: &'static str,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Map(ref v) if v.is_empty() => visitor.visit_unit(),
                        Content::Seq(ref v) if v.is_empty() => visitor.visit_unit(),
                        _ => self.deserialize_any(visitor),
                    }
                }
                fn deserialize_newtype_struct<V>(
                    self,
                    _name: &str,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Newtype(v) => {
                            visitor.visit_newtype_struct(ContentDeserializer::new(*v))
                        }
                        _ => visitor.visit_newtype_struct(self),
                    }
                }
                fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Seq(v) => visit_content_seq(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_tuple<V>(
                    self,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_tuple_struct<V>(
                    self,
                    _name: &'static str,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Map(v) => visit_content_map(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_struct<V>(
                    self,
                    _name: &'static str,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::Seq(v) => visit_content_seq(v, visitor),
                        Content::Map(v) => visit_content_map(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_enum<V>(
                    self,
                    _name: &str,
                    _variants: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let (variant, value) = match self.content {
                        Content::Map(value) => {
                            let mut iter = value.into_iter();
                            let (variant, value) = match iter.next() {
                                Some(v) => v,
                                None => {
                                    return Err(
                                        de::Error::invalid_value(
                                            de::Unexpected::Map,
                                            &"map with a single key",
                                        ),
                                    );
                                }
                            };
                            if iter.next().is_some() {
                                return Err(
                                    de::Error::invalid_value(
                                        de::Unexpected::Map,
                                        &"map with a single key",
                                    ),
                                );
                            }
                            (variant, Some(value))
                        }
                        s @ Content::String(_) | s @ Content::Str(_) => (s, None),
                        other => {
                            return Err(
                                de::Error::invalid_type(
                                    content_unexpected(&other),
                                    &"string or map",
                                ),
                            );
                        }
                    };
                    visitor.visit_enum(EnumDeserializer::new(variant, value))
                }
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match self.content {
                        Content::String(v) => visitor.visit_string(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(v) => visitor.visit_byte_buf(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    drop(self);
                    visitor.visit_unit()
                }
                fn __deserialize_content_v1<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de, Value = Content<'de>>,
                {
                    let _ = visitor;
                    Ok(self.content)
                }
            }
            impl<'de, E> ContentDeserializer<'de, E> {
                /// private API, don't use
                pub fn new(content: Content<'de>) -> Self {
                    ContentDeserializer {
                        content,
                        err: PhantomData,
                    }
                }
            }
            struct SeqDeserializer<'de, E> {
                iter: <Vec<Content<'de>> as IntoIterator>::IntoIter,
                count: usize,
                marker: PhantomData<E>,
            }
            impl<'de, E> SeqDeserializer<'de, E> {
                fn new(content: Vec<Content<'de>>) -> Self {
                    SeqDeserializer {
                        iter: content.into_iter(),
                        count: 0,
                        marker: PhantomData,
                    }
                }
            }
            impl<'de, E> SeqDeserializer<'de, E>
            where
                E: de::Error,
            {
                fn end(self) -> Result<(), E> {
                    let remaining = self.iter.count();
                    if remaining == 0 {
                        Ok(())
                    } else {
                        Err(
                            de::Error::invalid_length(
                                self.count + remaining,
                                &ExpectedInSeq(self.count),
                            ),
                        )
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> Deserializer<'de> for SeqDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let v = match visitor.visit_seq(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(v)
                }
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_seq<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> SeqAccess<'de> for SeqDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<V>(
                    &mut self,
                    seed: V,
                ) -> Result<Option<V::Value>, Self::Error>
                where
                    V: DeserializeSeed<'de>,
                {
                    match self.iter.next() {
                        Some(value) => {
                            self.count += 1;
                            seed.deserialize(ContentDeserializer::new(value)).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            struct ExpectedInSeq(usize);
            #[diagnostic::do_not_recommend]
            impl Expected for ExpectedInSeq {
                fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        formatter.write_str("1 element in sequence")
                    } else {
                        formatter
                            .write_fmt(format_args!("{0} elements in sequence", self.0))
                    }
                }
            }
            struct MapDeserializer<'de, E> {
                iter: <Vec<(Content<'de>, Content<'de>)> as IntoIterator>::IntoIter,
                value: Option<Content<'de>>,
                count: usize,
                error: PhantomData<E>,
            }
            impl<'de, E> MapDeserializer<'de, E> {
                fn new(content: Vec<(Content<'de>, Content<'de>)>) -> Self {
                    MapDeserializer {
                        iter: content.into_iter(),
                        value: None,
                        count: 0,
                        error: PhantomData,
                    }
                }
            }
            impl<'de, E> MapDeserializer<'de, E>
            where
                E: de::Error,
            {
                fn end(self) -> Result<(), E> {
                    let remaining = self.iter.count();
                    if remaining == 0 {
                        Ok(())
                    } else {
                        Err(
                            de::Error::invalid_length(
                                self.count + remaining,
                                &ExpectedInMap(self.count),
                            ),
                        )
                    }
                }
            }
            impl<'de, E> MapDeserializer<'de, E> {
                fn next_pair(&mut self) -> Option<(Content<'de>, Content<'de>)> {
                    match self.iter.next() {
                        Some((k, v)) => {
                            self.count += 1;
                            Some((k, v))
                        }
                        None => None,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> Deserializer<'de> for MapDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let value = match visitor.visit_map(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(value)
                }
                fn deserialize_seq<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let value = match visitor.visit_seq(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(value)
                }
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let _ = len;
                    self.deserialize_seq(visitor)
                }
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> MapAccess<'de> for MapDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_key_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((key, value)) => {
                            self.value = Some(value);
                            seed.deserialize(ContentDeserializer::new(key)).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn next_value_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<T::Value, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    let value = self.value.take();
                    let value = value
                        .expect("MapAccess::next_value called before next_key");
                    seed.deserialize(ContentDeserializer::new(value))
                }
                fn next_entry_seed<TK, TV>(
                    &mut self,
                    kseed: TK,
                    vseed: TV,
                ) -> Result<Option<(TK::Value, TV::Value)>, Self::Error>
                where
                    TK: DeserializeSeed<'de>,
                    TV: DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((key, value)) => {
                            let key = match kseed
                                .deserialize(ContentDeserializer::new(key))
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            let value = match vseed
                                .deserialize(ContentDeserializer::new(value))
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            Ok(Some((key, value)))
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> SeqAccess<'de> for MapDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: de::DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((k, v)) => {
                            let de = PairDeserializer(k, v, PhantomData);
                            seed.deserialize(de).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            struct PairDeserializer<'de, E>(Content<'de>, Content<'de>, PhantomData<E>);
            #[diagnostic::do_not_recommend]
            impl<'de, E> Deserializer<'de> for PairDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let mut pair_visitor = PairVisitor(
                        Some(self.0),
                        Some(self.1),
                        PhantomData,
                    );
                    let pair = match visitor.visit_seq(&mut pair_visitor) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    if pair_visitor.1.is_none() {
                        Ok(pair)
                    } else {
                        let remaining = pair_visitor.size_hint().unwrap();
                        Err(de::Error::invalid_length(2, &ExpectedInSeq(2 - remaining)))
                    }
                }
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    if len == 2 {
                        self.deserialize_seq(visitor)
                    } else {
                        Err(de::Error::invalid_length(2, &ExpectedInSeq(len)))
                    }
                }
            }
            struct PairVisitor<'de, E>(
                Option<Content<'de>>,
                Option<Content<'de>>,
                PhantomData<E>,
            );
            #[diagnostic::do_not_recommend]
            impl<'de, E> SeqAccess<'de> for PairVisitor<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    if let Some(k) = self.0.take() {
                        seed.deserialize(ContentDeserializer::new(k)).map(Some)
                    } else if let Some(v) = self.1.take() {
                        seed.deserialize(ContentDeserializer::new(v)).map(Some)
                    } else {
                        Ok(None)
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    if self.0.is_some() {
                        Some(2)
                    } else if self.1.is_some() {
                        Some(1)
                    } else {
                        Some(0)
                    }
                }
            }
            struct ExpectedInMap(usize);
            #[diagnostic::do_not_recommend]
            impl Expected for ExpectedInMap {
                fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        formatter.write_str("1 element in map")
                    } else {
                        formatter.write_fmt(format_args!("{0} elements in map", self.0))
                    }
                }
            }
            pub struct EnumDeserializer<'de, E>
            where
                E: de::Error,
            {
                variant: Content<'de>,
                value: Option<Content<'de>>,
                err: PhantomData<E>,
            }
            impl<'de, E> EnumDeserializer<'de, E>
            where
                E: de::Error,
            {
                pub fn new(
                    variant: Content<'de>,
                    value: Option<Content<'de>>,
                ) -> EnumDeserializer<'de, E> {
                    EnumDeserializer {
                        variant,
                        value,
                        err: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> de::EnumAccess<'de> for EnumDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                type Variant = VariantDeserializer<'de, Self::Error>;
                fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), E>
                where
                    V: de::DeserializeSeed<'de>,
                {
                    let visitor = VariantDeserializer {
                        value: self.value,
                        err: PhantomData,
                    };
                    seed.deserialize(ContentDeserializer::new(self.variant))
                        .map(|v| (v, visitor))
                }
            }
            pub struct VariantDeserializer<'de, E>
            where
                E: de::Error,
            {
                value: Option<Content<'de>>,
                err: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> de::VariantAccess<'de> for VariantDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn unit_variant(self) -> Result<(), E> {
                    match self.value {
                        Some(value) => {
                            de::Deserialize::deserialize(ContentDeserializer::new(value))
                        }
                        None => Ok(()),
                    }
                }
                fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
                where
                    T: de::DeserializeSeed<'de>,
                {
                    match self.value {
                        Some(value) => seed.deserialize(ContentDeserializer::new(value)),
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"newtype variant",
                                ),
                            )
                        }
                    }
                }
                fn tuple_variant<V>(
                    self,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    match self.value {
                        Some(Content::Seq(v)) => {
                            de::Deserializer::deserialize_any(
                                SeqDeserializer::new(v),
                                visitor,
                            )
                        }
                        Some(other) => {
                            Err(
                                de::Error::invalid_type(
                                    content_unexpected(&other),
                                    &"tuple variant",
                                ),
                            )
                        }
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"tuple variant",
                                ),
                            )
                        }
                    }
                }
                fn struct_variant<V>(
                    self,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    match self.value {
                        Some(Content::Map(v)) => {
                            de::Deserializer::deserialize_any(
                                MapDeserializer::new(v),
                                visitor,
                            )
                        }
                        Some(Content::Seq(v)) => {
                            de::Deserializer::deserialize_any(
                                SeqDeserializer::new(v),
                                visitor,
                            )
                        }
                        Some(other) => {
                            Err(
                                de::Error::invalid_type(
                                    content_unexpected(&other),
                                    &"struct variant",
                                ),
                            )
                        }
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"struct variant",
                                ),
                            )
                        }
                    }
                }
            }
            /// Not public API.
            pub struct ContentRefDeserializer<'a, 'de: 'a, E> {
                content: &'a Content<'de>,
                err: PhantomData<E>,
            }
            impl<'a, 'de, E> ContentRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                #[cold]
                fn invalid_type(self, exp: &dyn Expected) -> E {
                    de::Error::invalid_type(content_unexpected(self.content), exp)
                }
                fn deserialize_integer<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_float<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::F32(v) => visitor.visit_f32(v),
                        Content::F64(v) => visitor.visit_f64(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
            }
            fn visit_content_seq_ref<'a, 'de, V, E>(
                content: &'a [Content<'de>],
                visitor: V,
            ) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
                E: de::Error,
            {
                let mut seq_visitor = SeqRefDeserializer::new(content);
                let value = match visitor.visit_seq(&mut seq_visitor) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match seq_visitor.end() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(value)
            }
            fn visit_content_map_ref<'a, 'de, V, E>(
                content: &'a [(Content<'de>, Content<'de>)],
                visitor: V,
            ) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
                E: de::Error,
            {
                let mut map_visitor = MapRefDeserializer::new(content);
                let value = match visitor.visit_map(&mut map_visitor) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map_visitor.end() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(value)
            }
            /// Used when deserializing an untagged enum because the content may need
            /// to be used more than once.
            #[diagnostic::do_not_recommend]
            impl<'de, 'a, E> Deserializer<'de> for ContentRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Bool(v) => visitor.visit_bool(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U16(v) => visitor.visit_u16(v),
                        Content::U32(v) => visitor.visit_u32(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        Content::I8(v) => visitor.visit_i8(v),
                        Content::I16(v) => visitor.visit_i16(v),
                        Content::I32(v) => visitor.visit_i32(v),
                        Content::I64(v) => visitor.visit_i64(v),
                        Content::F32(v) => visitor.visit_f32(v),
                        Content::F64(v) => visitor.visit_f64(v),
                        Content::Char(v) => visitor.visit_char(v),
                        Content::String(ref v) => visitor.visit_str(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(ref v) => visitor.visit_bytes(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::Unit => visitor.visit_unit(),
                        Content::None => visitor.visit_none(),
                        Content::Some(ref v) => {
                            visitor.visit_some(ContentRefDeserializer::new(v))
                        }
                        Content::Newtype(ref v) => {
                            visitor.visit_newtype_struct(ContentRefDeserializer::new(v))
                        }
                        Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
                        Content::Map(ref v) => visit_content_map_ref(v, visitor),
                    }
                }
                fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Bool(v) => visitor.visit_bool(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_integer(visitor)
                }
                fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_float(visitor)
                }
                fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_float(visitor)
                }
                fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Char(v) => visitor.visit_char(v),
                        Content::String(ref v) => visitor.visit_str(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::String(ref v) => visitor.visit_str(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(ref v) => visitor.visit_bytes(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_str(visitor)
                }
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::String(ref v) => visitor.visit_str(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(ref v) => visitor.visit_bytes(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_bytes(visitor)
                }
                fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::None => visitor.visit_none(),
                        Content::Some(ref v) => {
                            visitor.visit_some(ContentRefDeserializer::new(v))
                        }
                        Content::Unit => visitor.visit_unit(),
                        _ => visitor.visit_some(self),
                    }
                }
                fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Unit => visitor.visit_unit(),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_unit_struct<V>(
                    self,
                    _name: &'static str,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_unit(visitor)
                }
                fn deserialize_newtype_struct<V>(
                    self,
                    _name: &str,
                    visitor: V,
                ) -> Result<V::Value, E>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Newtype(ref v) => {
                            visitor.visit_newtype_struct(ContentRefDeserializer::new(v))
                        }
                        _ => visitor.visit_newtype_struct(self),
                    }
                }
                fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_tuple<V>(
                    self,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_tuple_struct<V>(
                    self,
                    _name: &'static str,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Map(ref v) => visit_content_map_ref(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_struct<V>(
                    self,
                    _name: &'static str,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::Seq(ref v) => visit_content_seq_ref(v, visitor),
                        Content::Map(ref v) => visit_content_map_ref(v, visitor),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_enum<V>(
                    self,
                    _name: &str,
                    _variants: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let (variant, value) = match *self.content {
                        Content::Map(ref value) => {
                            let mut iter = value.iter();
                            let (variant, value) = match iter.next() {
                                Some(v) => v,
                                None => {
                                    return Err(
                                        de::Error::invalid_value(
                                            de::Unexpected::Map,
                                            &"map with a single key",
                                        ),
                                    );
                                }
                            };
                            if iter.next().is_some() {
                                return Err(
                                    de::Error::invalid_value(
                                        de::Unexpected::Map,
                                        &"map with a single key",
                                    ),
                                );
                            }
                            (variant, Some(value))
                        }
                        ref s @ Content::String(_) | ref s @ Content::Str(_) => (s, None),
                        ref other => {
                            return Err(
                                de::Error::invalid_type(
                                    content_unexpected(other),
                                    &"string or map",
                                ),
                            );
                        }
                    };
                    visitor
                        .visit_enum(EnumRefDeserializer {
                            variant,
                            value,
                            err: PhantomData,
                        })
                }
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    match *self.content {
                        Content::String(ref v) => visitor.visit_str(v),
                        Content::Str(v) => visitor.visit_borrowed_str(v),
                        Content::ByteBuf(ref v) => visitor.visit_bytes(v),
                        Content::Bytes(v) => visitor.visit_borrowed_bytes(v),
                        Content::U8(v) => visitor.visit_u8(v),
                        Content::U64(v) => visitor.visit_u64(v),
                        _ => Err(self.invalid_type(&visitor)),
                    }
                }
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    visitor.visit_unit()
                }
                fn __deserialize_content_v1<V>(
                    self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de, Value = Content<'de>>,
                {
                    let _ = visitor;
                    Ok(content_clone(self.content))
                }
            }
            impl<'a, 'de, E> ContentRefDeserializer<'a, 'de, E> {
                /// private API, don't use
                pub fn new(content: &'a Content<'de>) -> Self {
                    ContentRefDeserializer {
                        content,
                        err: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de: 'a, E> Copy for ContentRefDeserializer<'a, 'de, E> {}
            #[diagnostic::do_not_recommend]
            impl<'a, 'de: 'a, E> Clone for ContentRefDeserializer<'a, 'de, E> {
                fn clone(&self) -> Self {
                    *self
                }
            }
            struct SeqRefDeserializer<'a, 'de, E> {
                iter: <&'a [Content<'de>] as IntoIterator>::IntoIter,
                count: usize,
                marker: PhantomData<E>,
            }
            impl<'a, 'de, E> SeqRefDeserializer<'a, 'de, E> {
                fn new(content: &'a [Content<'de>]) -> Self {
                    SeqRefDeserializer {
                        iter: content.iter(),
                        count: 0,
                        marker: PhantomData,
                    }
                }
            }
            impl<'a, 'de, E> SeqRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                fn end(self) -> Result<(), E> {
                    let remaining = self.iter.count();
                    if remaining == 0 {
                        Ok(())
                    } else {
                        Err(
                            de::Error::invalid_length(
                                self.count + remaining,
                                &ExpectedInSeq(self.count),
                            ),
                        )
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> Deserializer<'de> for SeqRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let v = match visitor.visit_seq(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(v)
                }
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_seq<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> SeqAccess<'de> for SeqRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<V>(
                    &mut self,
                    seed: V,
                ) -> Result<Option<V::Value>, Self::Error>
                where
                    V: DeserializeSeed<'de>,
                {
                    match self.iter.next() {
                        Some(value) => {
                            self.count += 1;
                            seed.deserialize(ContentRefDeserializer::new(value))
                                .map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            struct MapRefDeserializer<'a, 'de, E> {
                iter: <&'a [(Content<'de>, Content<'de>)] as IntoIterator>::IntoIter,
                value: Option<&'a Content<'de>>,
                count: usize,
                error: PhantomData<E>,
            }
            impl<'a, 'de, E> MapRefDeserializer<'a, 'de, E> {
                fn new(content: &'a [(Content<'de>, Content<'de>)]) -> Self {
                    MapRefDeserializer {
                        iter: content.iter(),
                        value: None,
                        count: 0,
                        error: PhantomData,
                    }
                }
            }
            impl<'a, 'de, E> MapRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                fn end(self) -> Result<(), E> {
                    let remaining = self.iter.count();
                    if remaining == 0 {
                        Ok(())
                    } else {
                        Err(
                            de::Error::invalid_length(
                                self.count + remaining,
                                &ExpectedInMap(self.count),
                            ),
                        )
                    }
                }
            }
            impl<'a, 'de, E> MapRefDeserializer<'a, 'de, E> {
                fn next_pair(&mut self) -> Option<(&'a Content<'de>, &'a Content<'de>)> {
                    match self.iter.next() {
                        Some((k, v)) => {
                            self.count += 1;
                            Some((k, v))
                        }
                        None => None,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> Deserializer<'de> for MapRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn deserialize_any<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let value = match visitor.visit_map(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(value)
                }
                fn deserialize_seq<V>(
                    mut self,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let value = match visitor.visit_seq(&mut self) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    match self.end() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    Ok(value)
                }
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let _ = len;
                    self.deserialize_seq(visitor)
                }
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> MapAccess<'de> for MapRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_key_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((key, value)) => {
                            self.value = Some(value);
                            seed.deserialize(ContentRefDeserializer::new(key)).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn next_value_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<T::Value, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    let value = self.value.take();
                    let value = value
                        .expect("MapAccess::next_value called before next_key");
                    seed.deserialize(ContentRefDeserializer::new(value))
                }
                fn next_entry_seed<TK, TV>(
                    &mut self,
                    kseed: TK,
                    vseed: TV,
                ) -> Result<Option<(TK::Value, TV::Value)>, Self::Error>
                where
                    TK: DeserializeSeed<'de>,
                    TV: DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((key, value)) => {
                            let key = match kseed
                                .deserialize(ContentRefDeserializer::new(key))
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            let value = match vseed
                                .deserialize(ContentRefDeserializer::new(value))
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            Ok(Some((key, value)))
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> SeqAccess<'de> for MapRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: de::DeserializeSeed<'de>,
                {
                    match self.next_pair() {
                        Some((k, v)) => {
                            let de = PairRefDeserializer(k, v, PhantomData);
                            seed.deserialize(de).map(Some)
                        }
                        None => Ok(None),
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    size_hint::from_bounds(&self.iter)
                }
            }
            struct PairRefDeserializer<'a, 'de, E>(
                &'a Content<'de>,
                &'a Content<'de>,
                PhantomData<E>,
            );
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> Deserializer<'de> for PairRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                #[inline]
                fn deserialize_bool<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_i128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u8<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u16<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_u128<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f32<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_f64<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_char<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_str<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_string<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_bytes<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_byte_buf<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_option<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_unit_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_newtype_struct<V>(
                    self,
                    name: &'static str,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_tuple_struct<V>(
                    self,
                    name: &'static str,
                    len: usize,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = len;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_map<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = fields;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_identifier<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                #[inline]
                fn deserialize_ignored_any<V>(
                    self,
                    visitor: V,
                ) -> ::serde_core::__private::Result<
                    V::Value,
                    <Self as ::serde_core::de::Deserializer<'de>>::Error,
                >
                where
                    V: ::serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    self.deserialize_seq(visitor)
                }
                fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'de>,
                {
                    let mut pair_visitor = PairRefVisitor(
                        Some(self.0),
                        Some(self.1),
                        PhantomData,
                    );
                    let pair = match visitor.visit_seq(&mut pair_visitor) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    if pair_visitor.1.is_none() {
                        Ok(pair)
                    } else {
                        let remaining = pair_visitor.size_hint().unwrap();
                        Err(de::Error::invalid_length(2, &ExpectedInSeq(2 - remaining)))
                    }
                }
                fn deserialize_tuple<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    if len == 2 {
                        self.deserialize_seq(visitor)
                    } else {
                        Err(de::Error::invalid_length(2, &ExpectedInSeq(len)))
                    }
                }
            }
            struct PairRefVisitor<'a, 'de, E>(
                Option<&'a Content<'de>>,
                Option<&'a Content<'de>>,
                PhantomData<E>,
            );
            #[diagnostic::do_not_recommend]
            impl<'a, 'de, E> SeqAccess<'de> for PairRefVisitor<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn next_element_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'de>,
                {
                    if let Some(k) = self.0.take() {
                        seed.deserialize(ContentRefDeserializer::new(k)).map(Some)
                    } else if let Some(v) = self.1.take() {
                        seed.deserialize(ContentRefDeserializer::new(v)).map(Some)
                    } else {
                        Ok(None)
                    }
                }
                fn size_hint(&self) -> Option<usize> {
                    if self.0.is_some() {
                        Some(2)
                    } else if self.1.is_some() {
                        Some(1)
                    } else {
                        Some(0)
                    }
                }
            }
            struct EnumRefDeserializer<'a, 'de: 'a, E>
            where
                E: de::Error,
            {
                variant: &'a Content<'de>,
                value: Option<&'a Content<'de>>,
                err: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<'de, 'a, E> de::EnumAccess<'de> for EnumRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                type Variant = VariantRefDeserializer<'a, 'de, Self::Error>;
                fn variant_seed<V>(
                    self,
                    seed: V,
                ) -> Result<(V::Value, Self::Variant), Self::Error>
                where
                    V: de::DeserializeSeed<'de>,
                {
                    let visitor = VariantRefDeserializer {
                        value: self.value,
                        err: PhantomData,
                    };
                    seed.deserialize(ContentRefDeserializer::new(self.variant))
                        .map(|v| (v, visitor))
                }
            }
            struct VariantRefDeserializer<'a, 'de: 'a, E>
            where
                E: de::Error,
            {
                value: Option<&'a Content<'de>>,
                err: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<'de, 'a, E> de::VariantAccess<'de>
            for VariantRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Error = E;
                fn unit_variant(self) -> Result<(), E> {
                    match self.value {
                        Some(value) => {
                            de::Deserialize::deserialize(
                                ContentRefDeserializer::new(value),
                            )
                        }
                        None => Ok(()),
                    }
                }
                fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, E>
                where
                    T: de::DeserializeSeed<'de>,
                {
                    match self.value {
                        Some(value) => {
                            seed.deserialize(ContentRefDeserializer::new(value))
                        }
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"newtype variant",
                                ),
                            )
                        }
                    }
                }
                fn tuple_variant<V>(
                    self,
                    _len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    match self.value {
                        Some(Content::Seq(v)) => visit_content_seq_ref(v, visitor),
                        Some(other) => {
                            Err(
                                de::Error::invalid_type(
                                    content_unexpected(other),
                                    &"tuple variant",
                                ),
                            )
                        }
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"tuple variant",
                                ),
                            )
                        }
                    }
                }
                fn struct_variant<V>(
                    self,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: de::Visitor<'de>,
                {
                    match self.value {
                        Some(Content::Map(v)) => visit_content_map_ref(v, visitor),
                        Some(Content::Seq(v)) => visit_content_seq_ref(v, visitor),
                        Some(other) => {
                            Err(
                                de::Error::invalid_type(
                                    content_unexpected(other),
                                    &"struct variant",
                                ),
                            )
                        }
                        None => {
                            Err(
                                de::Error::invalid_type(
                                    de::Unexpected::UnitVariant,
                                    &"struct variant",
                                ),
                            )
                        }
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, E> de::IntoDeserializer<'de, E> for ContentDeserializer<'de, E>
            where
                E: de::Error,
            {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self {
                    self
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, 'a, E> de::IntoDeserializer<'de, E>
            for ContentRefDeserializer<'a, 'de, E>
            where
                E: de::Error,
            {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self {
                    self
                }
            }
            /// Visitor for deserializing an internally tagged unit variant.
            ///
            /// Not public API.
            pub struct InternallyTaggedUnitVisitor<'a> {
                type_name: &'a str,
                variant_name: &'a str,
            }
            impl<'a> InternallyTaggedUnitVisitor<'a> {
                /// Not public API.
                pub fn new(type_name: &'a str, variant_name: &'a str) -> Self {
                    InternallyTaggedUnitVisitor {
                        type_name,
                        variant_name,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, 'a> Visitor<'de> for InternallyTaggedUnitVisitor<'a> {
                type Value = ();
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter
                        .write_fmt(
                            format_args!(
                                "unit variant {0}::{1}",
                                self.type_name,
                                self.variant_name,
                            ),
                        )
                }
                fn visit_seq<S>(self, _: S) -> Result<(), S::Error>
                where
                    S: SeqAccess<'de>,
                {
                    Ok(())
                }
                fn visit_map<M>(self, mut access: M) -> Result<(), M::Error>
                where
                    M: MapAccess<'de>,
                {
                    while match access.next_entry::<IgnoredAny, IgnoredAny>() {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    }
                        .is_some()
                    {}
                    Ok(())
                }
            }
            /// Visitor for deserializing an untagged unit variant.
            ///
            /// Not public API.
            pub struct UntaggedUnitVisitor<'a> {
                type_name: &'a str,
                variant_name: &'a str,
            }
            impl<'a> UntaggedUnitVisitor<'a> {
                /// Not public API.
                pub fn new(type_name: &'a str, variant_name: &'a str) -> Self {
                    UntaggedUnitVisitor {
                        type_name,
                        variant_name,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<'de, 'a> Visitor<'de> for UntaggedUnitVisitor<'a> {
                type Value = ();
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter
                        .write_fmt(
                            format_args!(
                                "unit variant {0}::{1}",
                                self.type_name,
                                self.variant_name,
                            ),
                        )
                }
                fn visit_unit<E>(self) -> Result<(), E>
                where
                    E: de::Error,
                {
                    Ok(())
                }
                fn visit_none<E>(self) -> Result<(), E>
                where
                    E: de::Error,
                {
                    Ok(())
                }
            }
        }
        pub trait IdentifierDeserializer<'de, E: Error> {
            type Deserializer: Deserializer<'de, Error = E>;
            fn from(self) -> Self::Deserializer;
        }
        pub struct Borrowed<'de, T: 'de + ?Sized>(pub &'de T);
        #[diagnostic::do_not_recommend]
        impl<'de, E> IdentifierDeserializer<'de, E> for u64
        where
            E: Error,
        {
            type Deserializer = <u64 as IntoDeserializer<'de, E>>::Deserializer;
            fn from(self) -> Self::Deserializer {
                self.into_deserializer()
            }
        }
        pub struct StrDeserializer<'a, E> {
            value: &'a str,
            marker: PhantomData<E>,
        }
        #[diagnostic::do_not_recommend]
        impl<'de, 'a, E> Deserializer<'de> for StrDeserializer<'a, E>
        where
            E: Error,
        {
            type Error = E;
            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_str(self.value)
            }
            #[inline]
            fn deserialize_bool<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i8<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i16<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i128<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u8<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u16<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u128<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_f32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_f64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_char<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_str<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_string<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_bytes<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_byte_buf<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_option<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_unit<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_unit_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_newtype_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_seq<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_tuple<V>(
                self,
                len: usize,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = len;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_tuple_struct<V>(
                self,
                name: &'static str,
                len: usize,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = len;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_map<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_struct<V>(
                self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = fields;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_enum<V>(
                self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = variants;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_identifier<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_ignored_any<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
        }
        pub struct BorrowedStrDeserializer<'de, E> {
            value: &'de str,
            marker: PhantomData<E>,
        }
        #[diagnostic::do_not_recommend]
        impl<'de, E> Deserializer<'de> for BorrowedStrDeserializer<'de, E>
        where
            E: Error,
        {
            type Error = E;
            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_borrowed_str(self.value)
            }
            #[inline]
            fn deserialize_bool<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i8<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i16<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_i128<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u8<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u16<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_u128<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_f32<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_f64<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_char<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_str<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_string<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_bytes<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_byte_buf<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_option<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_unit<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_unit_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_newtype_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_seq<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_tuple<V>(
                self,
                len: usize,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = len;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_tuple_struct<V>(
                self,
                name: &'static str,
                len: usize,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = len;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_map<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_struct<V>(
                self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = fields;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_enum<V>(
                self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                let _ = name;
                let _ = variants;
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_identifier<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
            #[inline]
            fn deserialize_ignored_any<V>(
                self,
                visitor: V,
            ) -> ::serde_core::__private::Result<
                V::Value,
                <Self as ::serde_core::de::Deserializer<'de>>::Error,
            >
            where
                V: ::serde_core::de::Visitor<'de>,
            {
                self.deserialize_any(visitor)
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, E> IdentifierDeserializer<'a, E> for &'a str
        where
            E: Error,
        {
            type Deserializer = StrDeserializer<'a, E>;
            fn from(self) -> Self::Deserializer {
                StrDeserializer {
                    value: self,
                    marker: PhantomData,
                }
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'de, E> IdentifierDeserializer<'de, E> for Borrowed<'de, str>
        where
            E: Error,
        {
            type Deserializer = BorrowedStrDeserializer<'de, E>;
            fn from(self) -> Self::Deserializer {
                BorrowedStrDeserializer {
                    value: self.0,
                    marker: PhantomData,
                }
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, E> IdentifierDeserializer<'a, E> for &'a [u8]
        where
            E: Error,
        {
            type Deserializer = BytesDeserializer<'a, E>;
            fn from(self) -> Self::Deserializer {
                BytesDeserializer::new(self)
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'de, E> IdentifierDeserializer<'de, E> for Borrowed<'de, [u8]>
        where
            E: Error,
        {
            type Deserializer = BorrowedBytesDeserializer<'de, E>;
            fn from(self) -> Self::Deserializer {
                BorrowedBytesDeserializer::new(self.0)
            }
        }
        pub struct FlatMapDeserializer<'a, 'de: 'a, E>(
            pub &'a mut Vec<Option<(Content<'de>, Content<'de>)>>,
            pub PhantomData<E>,
        );
        impl<'a, 'de, E> FlatMapDeserializer<'a, 'de, E>
        where
            E: Error,
        {
            fn deserialize_other<V>() -> Result<V, E> {
                Err(Error::custom("can only flatten structs and maps"))
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, 'de, E> Deserializer<'de> for FlatMapDeserializer<'a, 'de, E>
        where
            E: Error,
        {
            type Error = E;
            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                self.deserialize_map(visitor)
            }
            fn deserialize_enum<V>(
                self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                for entry in self.0 {
                    if let Some((key, value)) = flat_map_take_entry(entry, variants) {
                        return visitor
                            .visit_enum(EnumDeserializer::new(key, Some(value)));
                    }
                }
                Err(
                    Error::custom(
                        format_args!(
                            "no variant of enum {0} found in flattened data",
                            name,
                        ),
                    ),
                )
            }
            fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor
                    .visit_map(FlatMapAccess {
                        iter: self.0.iter(),
                        pending_content: None,
                        _marker: PhantomData,
                    })
            }
            fn deserialize_struct<V>(
                self,
                _: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor
                    .visit_map(FlatStructAccess {
                        iter: self.0.iter_mut(),
                        pending_content: None,
                        fields,
                        _marker: PhantomData,
                    })
            }
            fn deserialize_newtype_struct<V>(
                self,
                _name: &str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_newtype_struct(self)
            }
            fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                match visitor.__private_visit_untagged_option(self) {
                    Ok(value) => Ok(value),
                    Err(()) => Self::deserialize_other(),
                }
            }
            fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_unit()
            }
            fn deserialize_unit_struct<V>(
                self,
                _name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_unit()
            }
            fn deserialize_ignored_any<V>(
                self,
                visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                visitor.visit_unit()
            }
            fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_byte_buf<V>(
                self,
                _visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_tuple<V>(
                self,
                _: usize,
                _visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_tuple_struct<V>(
                self,
                _: &'static str,
                _: usize,
                _visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
            fn deserialize_identifier<V>(
                self,
                _visitor: V,
            ) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                Self::deserialize_other()
            }
        }
        struct FlatMapAccess<'a, 'de: 'a, E> {
            iter: slice::Iter<'a, Option<(Content<'de>, Content<'de>)>>,
            pending_content: Option<&'a Content<'de>>,
            _marker: PhantomData<E>,
        }
        #[diagnostic::do_not_recommend]
        impl<'a, 'de, E> MapAccess<'de> for FlatMapAccess<'a, 'de, E>
        where
            E: Error,
        {
            type Error = E;
            fn next_key_seed<T>(
                &mut self,
                seed: T,
            ) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                for item in &mut self.iter {
                    if let Some((ref key, ref content)) = *item {
                        self.pending_content = Some(content);
                        return seed
                            .deserialize(ContentRefDeserializer::new(key))
                            .map(Some);
                    }
                }
                Ok(None)
            }
            fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                match self.pending_content.take() {
                    Some(value) => seed.deserialize(ContentRefDeserializer::new(value)),
                    None => Err(Error::custom("value is missing")),
                }
            }
        }
        struct FlatStructAccess<'a, 'de: 'a, E> {
            iter: slice::IterMut<'a, Option<(Content<'de>, Content<'de>)>>,
            pending_content: Option<Content<'de>>,
            fields: &'static [&'static str],
            _marker: PhantomData<E>,
        }
        #[diagnostic::do_not_recommend]
        impl<'a, 'de, E> MapAccess<'de> for FlatStructAccess<'a, 'de, E>
        where
            E: Error,
        {
            type Error = E;
            fn next_key_seed<T>(
                &mut self,
                seed: T,
            ) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                for entry in self.iter.by_ref() {
                    if let Some((key, content)) = flat_map_take_entry(
                        entry,
                        self.fields,
                    ) {
                        self.pending_content = Some(content);
                        return seed.deserialize(ContentDeserializer::new(key)).map(Some);
                    }
                }
                Ok(None)
            }
            fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                match self.pending_content.take() {
                    Some(value) => seed.deserialize(ContentDeserializer::new(value)),
                    None => Err(Error::custom("value is missing")),
                }
            }
        }
        /// Claims one key-value pair from a FlatMapDeserializer's field buffer if the
        /// field name matches any of the recognized ones.
        fn flat_map_take_entry<'de>(
            entry: &mut Option<(Content<'de>, Content<'de>)>,
            recognized: &[&str],
        ) -> Option<(Content<'de>, Content<'de>)> {
            let is_recognized = match entry {
                None => false,
                Some((k, _v)) => {
                    content_as_str(k).map_or(false, |name| recognized.contains(&name))
                }
            };
            if is_recognized { entry.take() } else { None }
        }
        pub struct AdjacentlyTaggedEnumVariantSeed<F> {
            pub enum_name: &'static str,
            pub variants: &'static [&'static str],
            pub fields_enum: PhantomData<F>,
        }
        pub struct AdjacentlyTaggedEnumVariantVisitor<F> {
            enum_name: &'static str,
            fields_enum: PhantomData<F>,
        }
        #[diagnostic::do_not_recommend]
        impl<'de, F> Visitor<'de> for AdjacentlyTaggedEnumVariantVisitor<F>
        where
            F: Deserialize<'de>,
        {
            type Value = F;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!("variant of enum {0}", self.enum_name))
            }
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (variant, variant_access) = match data.variant() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match variant_access.unit_variant() {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(variant)
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'de, F> DeserializeSeed<'de> for AdjacentlyTaggedEnumVariantSeed<F>
        where
            F: Deserialize<'de>,
        {
            type Value = F;
            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer
                    .deserialize_enum(
                        self.enum_name,
                        self.variants,
                        AdjacentlyTaggedEnumVariantVisitor {
                            enum_name: self.enum_name,
                            fields_enum: PhantomData,
                        },
                    )
            }
        }
    }
    pub mod ser {
        use crate::lib::*;
        use crate::ser::{
            self, Impossible, Serialize, SerializeMap, SerializeStruct, Serializer,
        };
        use self::content::{
            Content, ContentSerializer, SerializeStructVariantAsMapValue,
            SerializeTupleVariantAsMapValue,
        };
        /// Used to check that serde(getter) attributes return the expected type.
        /// Not public API.
        pub fn constrain<T: ?Sized>(t: &T) -> &T {
            t
        }
        /// Not public API.
        pub fn serialize_tagged_newtype<S, T>(
            serializer: S,
            type_ident: &'static str,
            variant_ident: &'static str,
            tag: &'static str,
            variant_name: &'static str,
            value: &T,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
            T: Serialize,
        {
            value
                .serialize(TaggedSerializer {
                    type_ident,
                    variant_ident,
                    tag,
                    variant_name,
                    delegate: serializer,
                })
        }
        struct TaggedSerializer<S> {
            type_ident: &'static str,
            variant_ident: &'static str,
            tag: &'static str,
            variant_name: &'static str,
            delegate: S,
        }
        enum Unsupported {
            Boolean,
            Integer,
            Float,
            Char,
            String,
            ByteArray,
            Optional,
            Sequence,
            Tuple,
            TupleStruct,
        }
        #[diagnostic::do_not_recommend]
        impl Display for Unsupported {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    Unsupported::Boolean => formatter.write_str("a boolean"),
                    Unsupported::Integer => formatter.write_str("an integer"),
                    Unsupported::Float => formatter.write_str("a float"),
                    Unsupported::Char => formatter.write_str("a char"),
                    Unsupported::String => formatter.write_str("a string"),
                    Unsupported::ByteArray => formatter.write_str("a byte array"),
                    Unsupported::Optional => formatter.write_str("an optional"),
                    Unsupported::Sequence => formatter.write_str("a sequence"),
                    Unsupported::Tuple => formatter.write_str("a tuple"),
                    Unsupported::TupleStruct => formatter.write_str("a tuple struct"),
                }
            }
        }
        impl<S> TaggedSerializer<S>
        where
            S: Serializer,
        {
            fn bad_type(self, what: Unsupported) -> S::Error {
                ser::Error::custom(
                    format_args!(
                        "cannot serialize tagged newtype variant {0}::{1} containing {2}",
                        self.type_ident,
                        self.variant_ident,
                        what,
                    ),
                )
            }
        }
        #[diagnostic::do_not_recommend]
        impl<S> Serializer for TaggedSerializer<S>
        where
            S: Serializer,
        {
            type Ok = S::Ok;
            type Error = S::Error;
            type SerializeSeq = Impossible<S::Ok, S::Error>;
            type SerializeTuple = Impossible<S::Ok, S::Error>;
            type SerializeTupleStruct = Impossible<S::Ok, S::Error>;
            type SerializeMap = S::SerializeMap;
            type SerializeStruct = S::SerializeStruct;
            type SerializeTupleVariant = SerializeTupleVariantAsMapValue<
                S::SerializeMap,
            >;
            type SerializeStructVariant = SerializeStructVariantAsMapValue<
                S::SerializeMap,
            >;
            fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Boolean))
            }
            fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Integer))
            }
            fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Float))
            }
            fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Float))
            }
            fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Char))
            }
            fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::String))
            }
            fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::ByteArray))
            }
            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                Err(self.bad_type(Unsupported::Optional))
            }
            fn serialize_some<T>(self, _: &T) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                Err(self.bad_type(Unsupported::Optional))
            }
            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                let mut map = match self.delegate.serialize_map(Some(1)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                map.end()
            }
            fn serialize_unit_struct(
                self,
                _: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                let mut map = match self.delegate.serialize_map(Some(1)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                map.end()
            }
            fn serialize_unit_variant(
                self,
                _: &'static str,
                _: u32,
                inner_variant: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                let mut map = match self.delegate.serialize_map(Some(2)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(inner_variant, &()) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                map.end()
            }
            fn serialize_newtype_struct<T>(
                self,
                _: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                value.serialize(self)
            }
            fn serialize_newtype_variant<T>(
                self,
                _: &'static str,
                _: u32,
                inner_variant: &'static str,
                inner_value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                let mut map = match self.delegate.serialize_map(Some(2)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(inner_variant, inner_value) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                map.end()
            }
            fn serialize_seq(
                self,
                _: Option<usize>,
            ) -> Result<Self::SerializeSeq, Self::Error> {
                Err(self.bad_type(Unsupported::Sequence))
            }
            fn serialize_tuple(
                self,
                _: usize,
            ) -> Result<Self::SerializeTuple, Self::Error> {
                Err(self.bad_type(Unsupported::Tuple))
            }
            fn serialize_tuple_struct(
                self,
                _: &'static str,
                _: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                Err(self.bad_type(Unsupported::TupleStruct))
            }
            fn serialize_tuple_variant(
                self,
                _: &'static str,
                _: u32,
                inner_variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                let mut map = match self.delegate.serialize_map(Some(2)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_key(inner_variant) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(SerializeTupleVariantAsMapValue::new(map, inner_variant, len))
            }
            fn serialize_map(
                self,
                len: Option<usize>,
            ) -> Result<Self::SerializeMap, Self::Error> {
                let mut map = match self.delegate.serialize_map(len.map(|len| len + 1)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(map)
            }
            fn serialize_struct(
                self,
                name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                let mut state = match self.delegate.serialize_struct(name, len + 1) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match state.serialize_field(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(state)
            }
            fn serialize_struct_variant(
                self,
                _: &'static str,
                _: u32,
                inner_variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                let mut map = match self.delegate.serialize_map(Some(2)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_entry(self.tag, self.variant_name) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                match map.serialize_key(inner_variant) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(SerializeStructVariantAsMapValue::new(map, inner_variant, len))
            }
        }
        mod content {
            use crate::lib::*;
            use crate::ser::{self, Serialize, Serializer};
            pub struct SerializeTupleVariantAsMapValue<M> {
                map: M,
                name: &'static str,
                fields: Vec<Content>,
            }
            impl<M> SerializeTupleVariantAsMapValue<M> {
                pub fn new(map: M, name: &'static str, len: usize) -> Self {
                    SerializeTupleVariantAsMapValue {
                        map,
                        name,
                        fields: Vec::with_capacity(len),
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<M> ser::SerializeTupleVariant for SerializeTupleVariantAsMapValue<M>
            where
                M: ser::SerializeMap,
            {
                type Ok = M::Ok;
                type Error = M::Error;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), M::Error>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value
                        .serialize(ContentSerializer::<M::Error>::new())
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push(value);
                    Ok(())
                }
                fn end(mut self) -> Result<M::Ok, M::Error> {
                    match self
                        .map
                        .serialize_value(&Content::TupleStruct(self.name, self.fields))
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.map.end()
                }
            }
            pub struct SerializeStructVariantAsMapValue<M> {
                map: M,
                name: &'static str,
                fields: Vec<(&'static str, Content)>,
            }
            impl<M> SerializeStructVariantAsMapValue<M> {
                pub fn new(map: M, name: &'static str, len: usize) -> Self {
                    SerializeStructVariantAsMapValue {
                        map,
                        name,
                        fields: Vec::with_capacity(len),
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<M> ser::SerializeStructVariant for SerializeStructVariantAsMapValue<M>
            where
                M: ser::SerializeMap,
            {
                type Ok = M::Ok;
                type Error = M::Error;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), M::Error>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value
                        .serialize(ContentSerializer::<M::Error>::new())
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push((key, value));
                    Ok(())
                }
                fn end(mut self) -> Result<M::Ok, M::Error> {
                    match self
                        .map
                        .serialize_value(&Content::Struct(self.name, self.fields))
                    {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.map.end()
                }
            }
            pub enum Content {
                Bool(bool),
                U8(u8),
                U16(u16),
                U32(u32),
                U64(u64),
                I8(i8),
                I16(i16),
                I32(i32),
                I64(i64),
                F32(f32),
                F64(f64),
                Char(char),
                String(String),
                Bytes(Vec<u8>),
                None,
                Some(Box<Content>),
                Unit,
                UnitStruct(&'static str),
                UnitVariant(&'static str, u32, &'static str),
                NewtypeStruct(&'static str, Box<Content>),
                NewtypeVariant(&'static str, u32, &'static str, Box<Content>),
                Seq(Vec<Content>),
                Tuple(Vec<Content>),
                TupleStruct(&'static str, Vec<Content>),
                TupleVariant(&'static str, u32, &'static str, Vec<Content>),
                Map(Vec<(Content, Content)>),
                Struct(&'static str, Vec<(&'static str, Content)>),
                StructVariant(
                    &'static str,
                    u32,
                    &'static str,
                    Vec<(&'static str, Content)>,
                ),
            }
            #[diagnostic::do_not_recommend]
            impl Serialize for Content {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    match *self {
                        Content::Bool(b) => serializer.serialize_bool(b),
                        Content::U8(u) => serializer.serialize_u8(u),
                        Content::U16(u) => serializer.serialize_u16(u),
                        Content::U32(u) => serializer.serialize_u32(u),
                        Content::U64(u) => serializer.serialize_u64(u),
                        Content::I8(i) => serializer.serialize_i8(i),
                        Content::I16(i) => serializer.serialize_i16(i),
                        Content::I32(i) => serializer.serialize_i32(i),
                        Content::I64(i) => serializer.serialize_i64(i),
                        Content::F32(f) => serializer.serialize_f32(f),
                        Content::F64(f) => serializer.serialize_f64(f),
                        Content::Char(c) => serializer.serialize_char(c),
                        Content::String(ref s) => serializer.serialize_str(s),
                        Content::Bytes(ref b) => serializer.serialize_bytes(b),
                        Content::None => serializer.serialize_none(),
                        Content::Some(ref c) => serializer.serialize_some(&**c),
                        Content::Unit => serializer.serialize_unit(),
                        Content::UnitStruct(n) => serializer.serialize_unit_struct(n),
                        Content::UnitVariant(n, i, v) => {
                            serializer.serialize_unit_variant(n, i, v)
                        }
                        Content::NewtypeStruct(n, ref c) => {
                            serializer.serialize_newtype_struct(n, &**c)
                        }
                        Content::NewtypeVariant(n, i, v, ref c) => {
                            serializer.serialize_newtype_variant(n, i, v, &**c)
                        }
                        Content::Seq(ref elements) => elements.serialize(serializer),
                        Content::Tuple(ref elements) => {
                            use crate::ser::SerializeTuple;
                            let mut tuple = match serializer
                                .serialize_tuple(elements.len())
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for e in elements {
                                match tuple.serialize_element(e) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            tuple.end()
                        }
                        Content::TupleStruct(n, ref fields) => {
                            use crate::ser::SerializeTupleStruct;
                            let mut ts = match serializer
                                .serialize_tuple_struct(n, fields.len())
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for f in fields {
                                match ts.serialize_field(f) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            ts.end()
                        }
                        Content::TupleVariant(n, i, v, ref fields) => {
                            use crate::ser::SerializeTupleVariant;
                            let mut tv = match serializer
                                .serialize_tuple_variant(n, i, v, fields.len())
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for f in fields {
                                match tv.serialize_field(f) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            tv.end()
                        }
                        Content::Map(ref entries) => {
                            use crate::ser::SerializeMap;
                            let mut map = match serializer
                                .serialize_map(Some(entries.len()))
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for (k, v) in entries {
                                match map.serialize_entry(k, v) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            map.end()
                        }
                        Content::Struct(n, ref fields) => {
                            use crate::ser::SerializeStruct;
                            let mut s = match serializer
                                .serialize_struct(n, fields.len())
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for &(k, ref v) in fields {
                                match s.serialize_field(k, v) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            s.end()
                        }
                        Content::StructVariant(n, i, v, ref fields) => {
                            use crate::ser::SerializeStructVariant;
                            let mut sv = match serializer
                                .serialize_struct_variant(n, i, v, fields.len())
                            {
                                Ok(val) => val,
                                Err(err) => return Err(err),
                            };
                            for &(k, ref v) in fields {
                                match sv.serialize_field(k, v) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                };
                            }
                            sv.end()
                        }
                    }
                }
            }
            pub struct ContentSerializer<E> {
                error: PhantomData<E>,
            }
            impl<E> ContentSerializer<E> {
                pub fn new() -> Self {
                    ContentSerializer {
                        error: PhantomData,
                    }
                }
            }
            #[diagnostic::do_not_recommend]
            impl<E> Serializer for ContentSerializer<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                type SerializeSeq = SerializeSeq<E>;
                type SerializeTuple = SerializeTuple<E>;
                type SerializeTupleStruct = SerializeTupleStruct<E>;
                type SerializeTupleVariant = SerializeTupleVariant<E>;
                type SerializeMap = SerializeMap<E>;
                type SerializeStruct = SerializeStruct<E>;
                type SerializeStructVariant = SerializeStructVariant<E>;
                fn serialize_bool(self, v: bool) -> Result<Content, E> {
                    Ok(Content::Bool(v))
                }
                fn serialize_i8(self, v: i8) -> Result<Content, E> {
                    Ok(Content::I8(v))
                }
                fn serialize_i16(self, v: i16) -> Result<Content, E> {
                    Ok(Content::I16(v))
                }
                fn serialize_i32(self, v: i32) -> Result<Content, E> {
                    Ok(Content::I32(v))
                }
                fn serialize_i64(self, v: i64) -> Result<Content, E> {
                    Ok(Content::I64(v))
                }
                fn serialize_u8(self, v: u8) -> Result<Content, E> {
                    Ok(Content::U8(v))
                }
                fn serialize_u16(self, v: u16) -> Result<Content, E> {
                    Ok(Content::U16(v))
                }
                fn serialize_u32(self, v: u32) -> Result<Content, E> {
                    Ok(Content::U32(v))
                }
                fn serialize_u64(self, v: u64) -> Result<Content, E> {
                    Ok(Content::U64(v))
                }
                fn serialize_f32(self, v: f32) -> Result<Content, E> {
                    Ok(Content::F32(v))
                }
                fn serialize_f64(self, v: f64) -> Result<Content, E> {
                    Ok(Content::F64(v))
                }
                fn serialize_char(self, v: char) -> Result<Content, E> {
                    Ok(Content::Char(v))
                }
                fn serialize_str(self, value: &str) -> Result<Content, E> {
                    Ok(Content::String(value.to_owned()))
                }
                fn serialize_bytes(self, value: &[u8]) -> Result<Content, E> {
                    Ok(Content::Bytes(value.to_owned()))
                }
                fn serialize_none(self) -> Result<Content, E> {
                    Ok(Content::None)
                }
                fn serialize_some<T>(self, value: &T) -> Result<Content, E>
                where
                    T: ?Sized + Serialize,
                {
                    Ok(
                        Content::Some(
                            Box::new(
                                match value.serialize(self) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                },
                            ),
                        ),
                    )
                }
                fn serialize_unit(self) -> Result<Content, E> {
                    Ok(Content::Unit)
                }
                fn serialize_unit_struct(
                    self,
                    name: &'static str,
                ) -> Result<Content, E> {
                    Ok(Content::UnitStruct(name))
                }
                fn serialize_unit_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                ) -> Result<Content, E> {
                    Ok(Content::UnitVariant(name, variant_index, variant))
                }
                fn serialize_newtype_struct<T>(
                    self,
                    name: &'static str,
                    value: &T,
                ) -> Result<Content, E>
                where
                    T: ?Sized + Serialize,
                {
                    Ok(
                        Content::NewtypeStruct(
                            name,
                            Box::new(
                                match value.serialize(self) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                },
                            ),
                        ),
                    )
                }
                fn serialize_newtype_variant<T>(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    value: &T,
                ) -> Result<Content, E>
                where
                    T: ?Sized + Serialize,
                {
                    Ok(
                        Content::NewtypeVariant(
                            name,
                            variant_index,
                            variant,
                            Box::new(
                                match value.serialize(self) {
                                    Ok(val) => val,
                                    Err(err) => return Err(err),
                                },
                            ),
                        ),
                    )
                }
                fn serialize_seq(
                    self,
                    len: Option<usize>,
                ) -> Result<Self::SerializeSeq, E> {
                    Ok(SerializeSeq {
                        elements: Vec::with_capacity(len.unwrap_or(0)),
                        error: PhantomData,
                    })
                }
                fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, E> {
                    Ok(SerializeTuple {
                        elements: Vec::with_capacity(len),
                        error: PhantomData,
                    })
                }
                fn serialize_tuple_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleStruct, E> {
                    Ok(SerializeTupleStruct {
                        name,
                        fields: Vec::with_capacity(len),
                        error: PhantomData,
                    })
                }
                fn serialize_tuple_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleVariant, E> {
                    Ok(SerializeTupleVariant {
                        name,
                        variant_index,
                        variant,
                        fields: Vec::with_capacity(len),
                        error: PhantomData,
                    })
                }
                fn serialize_map(
                    self,
                    len: Option<usize>,
                ) -> Result<Self::SerializeMap, E> {
                    Ok(SerializeMap {
                        entries: Vec::with_capacity(len.unwrap_or(0)),
                        key: None,
                        error: PhantomData,
                    })
                }
                fn serialize_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStruct, E> {
                    Ok(SerializeStruct {
                        name,
                        fields: Vec::with_capacity(len),
                        error: PhantomData,
                    })
                }
                fn serialize_struct_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStructVariant, E> {
                    Ok(SerializeStructVariant {
                        name,
                        variant_index,
                        variant,
                        fields: Vec::with_capacity(len),
                        error: PhantomData,
                    })
                }
            }
            pub struct SerializeSeq<E> {
                elements: Vec<Content>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeSeq for SerializeSeq<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.elements.push(value);
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(Content::Seq(self.elements))
                }
            }
            pub struct SerializeTuple<E> {
                elements: Vec<Content>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeTuple for SerializeTuple<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.elements.push(value);
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(Content::Tuple(self.elements))
                }
            }
            pub struct SerializeTupleStruct<E> {
                name: &'static str,
                fields: Vec<Content>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeTupleStruct for SerializeTupleStruct<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push(value);
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(Content::TupleStruct(self.name, self.fields))
                }
            }
            pub struct SerializeTupleVariant<E> {
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                fields: Vec<Content>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeTupleVariant for SerializeTupleVariant<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push(value);
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(
                        Content::TupleVariant(
                            self.name,
                            self.variant_index,
                            self.variant,
                            self.fields,
                        ),
                    )
                }
            }
            pub struct SerializeMap<E> {
                entries: Vec<(Content, Content)>,
                key: Option<Content>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeMap for SerializeMap<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_key<T>(&mut self, key: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let key = match key.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.key = Some(key);
                    Ok(())
                }
                fn serialize_value<T>(&mut self, value: &T) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let key = self
                        .key
                        .take()
                        .expect("serialize_value called before serialize_key");
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.entries.push((key, value));
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(Content::Map(self.entries))
                }
                fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), E>
                where
                    K: ?Sized + Serialize,
                    V: ?Sized + Serialize,
                {
                    let key = match key.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.entries.push((key, value));
                    Ok(())
                }
            }
            pub struct SerializeStruct<E> {
                name: &'static str,
                fields: Vec<(&'static str, Content)>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeStruct for SerializeStruct<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push((key, value));
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(Content::Struct(self.name, self.fields))
                }
            }
            pub struct SerializeStructVariant<E> {
                name: &'static str,
                variant_index: u32,
                variant: &'static str,
                fields: Vec<(&'static str, Content)>,
                error: PhantomData<E>,
            }
            #[diagnostic::do_not_recommend]
            impl<E> ser::SerializeStructVariant for SerializeStructVariant<E>
            where
                E: ser::Error,
            {
                type Ok = Content;
                type Error = E;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), E>
                where
                    T: ?Sized + Serialize,
                {
                    let value = match value.serialize(ContentSerializer::<E>::new()) {
                        Ok(val) => val,
                        Err(err) => return Err(err),
                    };
                    self.fields.push((key, value));
                    Ok(())
                }
                fn end(self) -> Result<Content, E> {
                    Ok(
                        Content::StructVariant(
                            self.name,
                            self.variant_index,
                            self.variant,
                            self.fields,
                        ),
                    )
                }
            }
        }
        pub struct FlatMapSerializer<'a, M: 'a>(pub &'a mut M);
        impl<'a, M> FlatMapSerializer<'a, M>
        where
            M: SerializeMap + 'a,
        {
            fn bad_type(what: Unsupported) -> M::Error {
                ser::Error::custom(
                    format_args!("can only flatten structs and maps (got {0})", what),
                )
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, M> Serializer for FlatMapSerializer<'a, M>
        where
            M: SerializeMap + 'a,
        {
            type Ok = ();
            type Error = M::Error;
            type SerializeSeq = Impossible<Self::Ok, M::Error>;
            type SerializeTuple = Impossible<Self::Ok, M::Error>;
            type SerializeTupleStruct = Impossible<Self::Ok, M::Error>;
            type SerializeMap = FlatMapSerializeMap<'a, M>;
            type SerializeStruct = FlatMapSerializeStruct<'a, M>;
            type SerializeTupleVariant = FlatMapSerializeTupleVariantAsMapValue<'a, M>;
            type SerializeStructVariant = FlatMapSerializeStructVariantAsMapValue<'a, M>;
            fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Boolean))
            }
            fn serialize_i8(self, _: i8) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_i16(self, _: i16) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_i32(self, _: i32) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_i64(self, _: i64) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_u8(self, _: u8) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_u16(self, _: u16) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_u32(self, _: u32) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_u64(self, _: u64) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Integer))
            }
            fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Float))
            }
            fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Float))
            }
            fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::Char))
            }
            fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::String))
            }
            fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
                Err(Self::bad_type(Unsupported::ByteArray))
            }
            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                Ok(())
            }
            fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                value.serialize(self)
            }
            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                Ok(())
            }
            fn serialize_unit_struct(
                self,
                _: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                Ok(())
            }
            fn serialize_unit_variant(
                self,
                _: &'static str,
                _: u32,
                variant: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                self.0.serialize_entry(variant, &())
            }
            fn serialize_newtype_struct<T>(
                self,
                _: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                value.serialize(self)
            }
            fn serialize_newtype_variant<T>(
                self,
                _: &'static str,
                _: u32,
                variant: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: ?Sized + Serialize,
            {
                self.0.serialize_entry(variant, value)
            }
            fn serialize_seq(
                self,
                _: Option<usize>,
            ) -> Result<Self::SerializeSeq, Self::Error> {
                Err(Self::bad_type(Unsupported::Sequence))
            }
            fn serialize_tuple(
                self,
                _: usize,
            ) -> Result<Self::SerializeTuple, Self::Error> {
                Err(Self::bad_type(Unsupported::Tuple))
            }
            fn serialize_tuple_struct(
                self,
                _: &'static str,
                _: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                Err(Self::bad_type(Unsupported::TupleStruct))
            }
            fn serialize_tuple_variant(
                self,
                _: &'static str,
                _: u32,
                variant: &'static str,
                _: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                match self.0.serialize_key(variant) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(FlatMapSerializeTupleVariantAsMapValue::new(self.0))
            }
            fn serialize_map(
                self,
                _: Option<usize>,
            ) -> Result<Self::SerializeMap, Self::Error> {
                Ok(FlatMapSerializeMap(self.0))
            }
            fn serialize_struct(
                self,
                _: &'static str,
                _: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                Ok(FlatMapSerializeStruct(self.0))
            }
            fn serialize_struct_variant(
                self,
                _: &'static str,
                _: u32,
                inner_variant: &'static str,
                _: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                match self.0.serialize_key(inner_variant) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(FlatMapSerializeStructVariantAsMapValue::new(self.0, inner_variant))
            }
        }
        pub struct FlatMapSerializeMap<'a, M: 'a>(&'a mut M);
        #[diagnostic::do_not_recommend]
        impl<'a, M> ser::SerializeMap for FlatMapSerializeMap<'a, M>
        where
            M: SerializeMap + 'a,
        {
            type Ok = ();
            type Error = M::Error;
            fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
            where
                T: ?Sized + Serialize,
            {
                self.0.serialize_key(key)
            }
            fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
            where
                T: ?Sized + Serialize,
            {
                self.0.serialize_value(value)
            }
            fn serialize_entry<K, V>(
                &mut self,
                key: &K,
                value: &V,
            ) -> Result<(), Self::Error>
            where
                K: ?Sized + Serialize,
                V: ?Sized + Serialize,
            {
                self.0.serialize_entry(key, value)
            }
            fn end(self) -> Result<(), Self::Error> {
                Ok(())
            }
        }
        pub struct FlatMapSerializeStruct<'a, M: 'a>(&'a mut M);
        #[diagnostic::do_not_recommend]
        impl<'a, M> ser::SerializeStruct for FlatMapSerializeStruct<'a, M>
        where
            M: SerializeMap + 'a,
        {
            type Ok = ();
            type Error = M::Error;
            fn serialize_field<T>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Self::Error>
            where
                T: ?Sized + Serialize,
            {
                self.0.serialize_entry(key, value)
            }
            fn end(self) -> Result<(), Self::Error> {
                Ok(())
            }
        }
        pub struct FlatMapSerializeTupleVariantAsMapValue<'a, M: 'a> {
            map: &'a mut M,
            fields: Vec<Content>,
        }
        impl<'a, M> FlatMapSerializeTupleVariantAsMapValue<'a, M>
        where
            M: SerializeMap + 'a,
        {
            fn new(map: &'a mut M) -> Self {
                FlatMapSerializeTupleVariantAsMapValue {
                    map,
                    fields: Vec::new(),
                }
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, M> ser::SerializeTupleVariant
        for FlatMapSerializeTupleVariantAsMapValue<'a, M>
        where
            M: SerializeMap + 'a,
        {
            type Ok = ();
            type Error = M::Error;
            fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
            where
                T: ?Sized + Serialize,
            {
                let value = match value.serialize(ContentSerializer::<M::Error>::new()) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                self.fields.push(value);
                Ok(())
            }
            fn end(self) -> Result<(), Self::Error> {
                match self.map.serialize_value(&Content::Seq(self.fields)) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(())
            }
        }
        pub struct FlatMapSerializeStructVariantAsMapValue<'a, M: 'a> {
            map: &'a mut M,
            name: &'static str,
            fields: Vec<(&'static str, Content)>,
        }
        impl<'a, M> FlatMapSerializeStructVariantAsMapValue<'a, M>
        where
            M: SerializeMap + 'a,
        {
            fn new(
                map: &'a mut M,
                name: &'static str,
            ) -> FlatMapSerializeStructVariantAsMapValue<'a, M> {
                FlatMapSerializeStructVariantAsMapValue {
                    map,
                    name,
                    fields: Vec::new(),
                }
            }
        }
        #[diagnostic::do_not_recommend]
        impl<'a, M> ser::SerializeStructVariant
        for FlatMapSerializeStructVariantAsMapValue<'a, M>
        where
            M: SerializeMap + 'a,
        {
            type Ok = ();
            type Error = M::Error;
            fn serialize_field<T>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Self::Error>
            where
                T: ?Sized + Serialize,
            {
                let value = match value.serialize(ContentSerializer::<M::Error>::new()) {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                self.fields.push((key, value));
                Ok(())
            }
            fn end(self) -> Result<(), Self::Error> {
                match self.map.serialize_value(&Content::Struct(self.name, self.fields))
                {
                    Ok(val) => val,
                    Err(err) => return Err(err),
                };
                Ok(())
            }
        }
        pub struct AdjacentlyTaggedEnumVariant {
            pub enum_name: &'static str,
            pub variant_index: u32,
            pub variant_name: &'static str,
        }
        #[diagnostic::do_not_recommend]
        impl Serialize for AdjacentlyTaggedEnumVariant {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer
                    .serialize_unit_variant(
                        self.enum_name,
                        self.variant_index,
                        self.variant_name,
                    )
            }
        }
        pub struct CannotSerializeVariant<T>(pub T);
        #[diagnostic::do_not_recommend]
        impl<T> Display for CannotSerializeVariant<T>
        where
            T: Debug,
        {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_fmt(
                        format_args!("enum variant cannot be serialized: {0:?}", self.0),
                    )
            }
        }
    }
    pub use crate::lib::clone::Clone;
    pub use crate::lib::convert::{From, Into, TryFrom};
    pub use crate::lib::default::Default;
    pub use crate::lib::fmt::{self, Formatter};
    pub use crate::lib::marker::PhantomData;
    pub use crate::lib::option::Option::{self, None, Some};
    pub use crate::lib::ptr;
    pub use crate::lib::result::Result::{self, Err, Ok};
    pub use crate::serde_core_private::string::from_utf8_lossy;
    pub use crate::lib::{ToString, Vec};
}
#[doc(hidden)]
pub mod __private228 {
    #[doc(hidden)]
    pub use crate::private::*;
}
use serde_core::__private228 as serde_core_private;
mod integer128 {}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
