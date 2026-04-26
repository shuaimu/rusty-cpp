#![feature(prelude_import)]
//! A [serde]-compatible spanned Value
//!
//! This allows capturing the location, in bytes, for a value in the original parsed document for
//! compatible deserializers.
//!
//! [serde]: https://serde.rs/
#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![forbid(unsafe_code)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
#[allow(unused_extern_crates)]
extern crate alloc;
mod spanned {
    use core::cmp::Ordering;
    use core::hash::{Hash, Hasher};
    pub(crate) const NAME: &str = "$__serde_spanned_private_Spanned";
    pub(crate) const START_FIELD: &str = "$__serde_spanned_private_start";
    pub(crate) const END_FIELD: &str = "$__serde_spanned_private_end";
    pub(crate) const VALUE_FIELD: &str = "$__serde_spanned_private_value";
    pub(crate) fn is_spanned(name: &'static str) -> bool {
        name == NAME
    }
    /// A spanned value, indicating the range at which it is defined in the source.
    pub struct Spanned<T> {
        /// Byte range
        span: core::ops::Range<usize>,
        /// The spanned value.
        value: T,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for Spanned<T> {
        #[inline]
        fn clone(&self) -> Spanned<T> {
            Spanned {
                span: ::core::clone::Clone::clone(&self.span),
                value: ::core::clone::Clone::clone(&self.value),
            }
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Spanned<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Spanned",
                "span",
                &self.span,
                "value",
                &&self.value,
            )
        }
    }
    impl<T> Spanned<T> {
        /// Create a spanned value encompassing the given byte range.
        ///
        /// # Example
        ///
        /// Transposing a `Spanned<Enum<T>>` into `Enum<Spanned<T>>`:
        ///
        /// ```
        /// use serde::de::{Deserialize, Deserializer};
        /// use serde_untagged::UntaggedEnumVisitor;
        /// use toml::Spanned;
        ///
        /// pub enum Dependency {
        ///     Simple(Spanned<String>),
        ///     Detailed(Spanned<DetailedDependency>),
        /// }
        ///
        /// impl<'de> Deserialize<'de> for Dependency {
        ///     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        ///     where
        ///         D: Deserializer<'de>,
        ///     {
        ///         enum DependencyKind {
        ///             Simple(String),
        ///             Detailed(DetailedDependency),
        ///         }
        ///
        ///         impl<'de> Deserialize<'de> for DependencyKind {
        ///             fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        ///             where
        ///                 D: Deserializer<'de>,
        ///             {
        ///                 UntaggedEnumVisitor::new()
        ///                     .expecting(
        ///                         "a version string like \"0.9.8\" or a \
        ///                             detailed dependency like { version = \"0.9.8\" }",
        ///                     )
        ///                     .string(|value| Ok(DependencyKind::Simple(value.to_owned())))
        ///                     .map(|value| value.deserialize().map(DependencyKind::Detailed))
        ///                     .deserialize(deserializer)
        ///             }
        ///         }
        ///
        ///         let spanned: Spanned<DependencyKind> = Deserialize::deserialize(deserializer)?;
        ///         let range = spanned.span();
        ///         Ok(match spanned.into_inner() {
        ///             DependencyKind::Simple(simple) => Dependency::Simple(Spanned::new(range, simple)),
        ///             DependencyKind::Detailed(detailed) => Dependency::Detailed(Spanned::new(range, detailed)),
        ///         })
        ///     }
        /// }
        /// #
        /// # type DetailedDependency = std::collections::BTreeMap<String, String>;
        /// ```
        pub fn new(range: core::ops::Range<usize>, value: T) -> Self {
            Self { span: range, value }
        }
        /// Byte range
        pub fn span(&self) -> core::ops::Range<usize> {
            self.span.clone()
        }
        /// Consumes the spanned value and returns the contained value.
        pub fn into_inner(self) -> T {
            self.value
        }
        /// Returns a reference to the contained value.
        pub fn get_ref(&self) -> &T {
            &self.value
        }
        /// Returns a mutable reference to the contained value.
        pub fn get_mut(&mut self) -> &mut T {
            &mut self.value
        }
    }
    impl<T> Spanned<T> {
        pub(crate) const START_FIELD: &str = START_FIELD;
        pub(crate) const END_FIELD: &str = END_FIELD;
        pub(crate) const VALUE_FIELD: &str = VALUE_FIELD;
    }
    impl<T: core::fmt::Display> core::fmt::Display for Spanned<T> {
        fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.get_ref().fmt(fmt)
        }
    }
    #[allow(unused_qualifications)]
    impl core::borrow::Borrow<str> for Spanned<alloc::string::String> {
        fn borrow(&self) -> &str {
            self.get_ref()
        }
    }
    impl core::borrow::Borrow<str> for Spanned<alloc::borrow::Cow<'_, str>> {
        fn borrow(&self) -> &str {
            self.get_ref()
        }
    }
    impl<T> AsRef<T> for Spanned<T> {
        fn as_ref(&self) -> &T {
            self.get_ref()
        }
    }
    impl<T> AsMut<T> for Spanned<T> {
        fn as_mut(&mut self) -> &mut T {
            self.get_mut()
        }
    }
    impl<T: PartialEq> PartialEq for Spanned<T> {
        fn eq(&self, other: &Self) -> bool {
            self.value.eq(&other.value)
        }
    }
    impl<T: Eq> Eq for Spanned<T> {}
    impl<T: Hash> Hash for Spanned<T> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.value.hash(state);
        }
    }
    impl<T: PartialOrd> PartialOrd for Spanned<T> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            self.value.partial_cmp(&other.value)
        }
    }
    impl<T: Ord> Ord for Spanned<T> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.value.cmp(&other.value)
        }
    }
    impl<'de, T> serde_core::de::Deserialize<'de> for Spanned<T>
    where
        T: serde_core::de::Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde_core::de::Deserializer<'de>,
        {
            struct SpannedVisitor<T>(::core::marker::PhantomData<T>);
            impl<'de, T> serde_core::de::Visitor<'de> for SpannedVisitor<T>
            where
                T: serde_core::de::Deserialize<'de>,
            {
                type Value = Spanned<T>;
                fn expecting(
                    &self,
                    formatter: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    formatter.write_str("a spanned value")
                }
                fn visit_map<V>(self, mut visitor: V) -> Result<Spanned<T>, V::Error>
                where
                    V: serde_core::de::MapAccess<'de>,
                {
                    let mut start: Option<usize> = None;
                    let mut end: Option<usize> = None;
                    let mut value: Option<T> = None;
                    while let Some(key) = visitor.next_key()? {
                        match key {
                            START_FIELD => {
                                if start.is_some() {
                                    return Err(
                                        serde_core::de::Error::duplicate_field(START_FIELD),
                                    );
                                }
                                start = Some(visitor.next_value()?);
                            }
                            END_FIELD => {
                                if end.is_some() {
                                    return Err(
                                        serde_core::de::Error::duplicate_field(END_FIELD),
                                    );
                                }
                                end = Some(visitor.next_value()?);
                            }
                            VALUE_FIELD => {
                                if value.is_some() {
                                    return Err(
                                        serde_core::de::Error::duplicate_field(VALUE_FIELD),
                                    );
                                }
                                value = Some(visitor.next_value()?);
                            }
                            field => {
                                return Err(
                                    serde_core::de::Error::unknown_field(
                                        field,
                                        &[START_FIELD, END_FIELD, VALUE_FIELD],
                                    ),
                                );
                            }
                        }
                    }
                    match (start, end, value) {
                        (Some(start), Some(end), Some(value)) => {
                            Ok(Spanned { span: start..end, value })
                        }
                        (None, _, _) => {
                            Err(serde_core::de::Error::missing_field(START_FIELD))
                        }
                        (_, None, _) => {
                            Err(serde_core::de::Error::missing_field(END_FIELD))
                        }
                        (_, _, None) => {
                            Err(serde_core::de::Error::missing_field(VALUE_FIELD))
                        }
                    }
                }
            }
            static FIELDS: [&str; 3] = [START_FIELD, END_FIELD, VALUE_FIELD];
            let visitor = SpannedVisitor(::core::marker::PhantomData);
            deserializer.deserialize_struct(NAME, &FIELDS, visitor)
        }
    }
    impl<T: serde_core::ser::Serialize> serde_core::ser::Serialize for Spanned<T> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde_core::ser::Serializer,
        {
            self.value.serialize(serializer)
        }
    }
}
pub use crate::spanned::Spanned;
pub mod de {
    //! Deserialization support for [`Spanned`]
    use serde_core::de::IntoDeserializer as _;
    use serde_core::de::value::BorrowedStrDeserializer;
    use crate::Spanned;
    /// Check if deserializing a [`Spanned`]
    pub fn is_spanned(name: &'static str) -> bool {
        crate::spanned::is_spanned(name)
    }
    /// Deserializer / format support for emitting [`Spanned`]
    pub struct SpannedDeserializer<'de, T, E>
    where
        T: serde_core::de::IntoDeserializer<'de, E>,
        E: serde_core::de::Error,
    {
        start: Option<usize>,
        end: Option<usize>,
        value: Option<T>,
        _lifetime: core::marker::PhantomData<&'de ()>,
        _error: core::marker::PhantomData<E>,
    }
    impl<'de, T, E> SpannedDeserializer<'de, T, E>
    where
        T: serde_core::de::IntoDeserializer<'de, E>,
        E: serde_core::de::Error,
    {
        /// Create a deserializer to emit [`Spanned`]
        pub fn new(value: T, span: core::ops::Range<usize>) -> Self {
            Self {
                start: Some(span.start),
                end: Some(span.end),
                value: Some(value),
                _lifetime: Default::default(),
                _error: Default::default(),
            }
        }
    }
    impl<'de, T, E> serde_core::de::MapAccess<'de> for SpannedDeserializer<'de, T, E>
    where
        T: serde_core::de::IntoDeserializer<'de, E>,
        E: serde_core::de::Error,
    {
        type Error = E;
        fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: serde_core::de::DeserializeSeed<'de>,
        {
            if self.start.is_some() {
                seed.deserialize(BorrowedStrDeserializer::new(Spanned::<T>::START_FIELD))
                    .map(Some)
            } else if self.end.is_some() {
                seed.deserialize(BorrowedStrDeserializer::new(Spanned::<T>::END_FIELD))
                    .map(Some)
            } else if self.value.is_some() {
                seed.deserialize(BorrowedStrDeserializer::new(Spanned::<T>::VALUE_FIELD))
                    .map(Some)
            } else {
                Ok(None)
            }
        }
        fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: serde_core::de::DeserializeSeed<'de>,
        {
            if let Some(start) = self.start.take() {
                seed.deserialize(start.into_deserializer())
            } else if let Some(end) = self.end.take() {
                seed.deserialize(end.into_deserializer())
            } else if let Some(value) = self.value.take() {
                seed.deserialize(value.into_deserializer())
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("next_value_seed called before next_key_seed"),
                    );
                }
            }
        }
    }
}
