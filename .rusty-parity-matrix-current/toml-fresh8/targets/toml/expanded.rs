#![feature(prelude_import)]
//! A [serde]-compatible [TOML]-parsing library
//!
//! TOML itself is a simple, ergonomic, and readable configuration format:
//!
//! ```toml
//! [package]
//! name = "toml"
//!
//! [dependencies]
//! serde = "1.0"
//! ```
//!
//! The TOML format tends to be relatively common throughout the Rust community
//! for configuration, notably being used by [Cargo], Rust's package manager.
//!
//! ## TOML values
//!
//! A TOML document is represented with the [`Table`] type which maps `String` to the [`Value`] enum:
//!
//! ```
//! # use toml::value::{Datetime, Array, Table};
//! pub enum Value {
//!     String(String),
//!     Integer(i64),
//!     Float(f64),
//!     Boolean(bool),
//!     Datetime(Datetime),
//!     Array(Array),
//!     Table(Table),
//! }
//! ```
//!
//! ## Parsing TOML
//!
//! The easiest way to parse a TOML document is via the [`Table`] type:
//!
//! ```
//! use toml::Table;
//!
//! let value = "foo = 'bar'".parse::<Table>().unwrap();
//!
//! assert_eq!(value["foo"].as_str(), Some("bar"));
//! ```
//!
//! The [`Table`] type implements a number of convenience methods and
//! traits; the example above uses [`FromStr`] to parse a [`str`] into a
//! [`Table`].
//!
//! ## Deserialization and Serialization
//!
//! This crate supports [`serde`] 1.0 with a number of
//! implementations of the `Deserialize`, `Serialize`, `Deserializer`, and
//! `Serializer` traits. Namely, you'll find:
//!
//! * `Deserialize for Table`
//! * `Serialize for Table`
//! * `Deserialize for Value`
//! * `Serialize for Value`
//! * `Deserialize for Datetime`
//! * `Serialize for Datetime`
//! * `Deserializer for de::Deserializer`
//! * `Serializer for ser::Serializer`
//! * `Deserializer for Table`
//! * `Deserializer for Value`
//!
//! This means that you can use Serde to deserialize/serialize the
//! [`Table`] type as well as [`Value`] and [`Datetime`] type in this crate. You can also
//! use the [`Deserializer`], [`Serializer`], or [`Table`] type itself to act as
//! a deserializer/serializer for arbitrary types.
//!
//! An example of deserializing with TOML is:
//!
//! ```
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     ip: String,
//!     port: Option<u16>,
//!     keys: Keys,
//! }
//!
//! #[derive(Deserialize)]
//! struct Keys {
//!     github: String,
//!     travis: Option<String>,
//! }
//!
//! let config: Config = toml::from_str(r#"
//!     ip = '127.0.0.1'
//!
//!     [keys]
//!     github = 'xxxxxxxxxxxxxxxxx'
//!     travis = 'yyyyyyyyyyyyyyyyy'
//! "#).unwrap();
//!
//! assert_eq!(config.ip, "127.0.0.1");
//! assert_eq!(config.port, None);
//! assert_eq!(config.keys.github, "xxxxxxxxxxxxxxxxx");
//! assert_eq!(config.keys.travis.as_ref().unwrap(), "yyyyyyyyyyyyyyyyy");
//! ```
//!
//! You can serialize types in a similar fashion:
//!
//! ```
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Config {
//!     ip: String,
//!     port: Option<u16>,
//!     keys: Keys,
//! }
//!
//! #[derive(Serialize)]
//! struct Keys {
//!     github: String,
//!     travis: Option<String>,
//! }
//!
//! let config = Config {
//!     ip: "127.0.0.1".to_string(),
//!     port: None,
//!     keys: Keys {
//!         github: "xxxxxxxxxxxxxxxxx".to_string(),
//!         travis: Some("yyyyyyyyyyyyyyyyy".to_string()),
//!     },
//! };
//!
//! let toml = toml::to_string(&config).unwrap();
//! ```
//!
//! [TOML]: https://github.com/toml-lang/toml
//! [Cargo]: https://crates.io/
//! [`serde`]: https://serde.rs/
//! [serde]: https://serde.rs/
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
#[allow(unused_extern_crates)]
extern crate alloc;
pub(crate) mod alloc_prelude {
    pub(crate) use alloc::borrow::ToOwned as _;
    pub(crate) use alloc::format;
    pub(crate) use alloc::string::String;
    pub(crate) use alloc::string::ToString as _;
    pub(crate) use alloc::vec::Vec;
}
pub mod map {
    //! A map of `String` to [Value][crate::Value].
    //!
    //! By default the map is backed by a [`BTreeMap`]. Enable the `preserve_order`
    //! feature of toml-rs to use [`IndexMap`] instead.
    //!
    //! [`BTreeMap`]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
    //! [`IndexMap`]: https://docs.rs/indexmap
    use alloc::collections::{BTreeMap, btree_map};
    use core::borrow::Borrow;
    use core::fmt::{self, Debug};
    use core::hash::Hash;
    use core::iter::FromIterator;
    use core::ops;
    /// Represents a TOML key/value type.
    pub struct Map<K, V> {
        map: MapImpl<K, V>,
        dotted: bool,
        implicit: bool,
        inline: bool,
    }
    type MapImpl<K, V> = BTreeMap<K, V>;
    impl<K, V> Map<K, V>
    where
        K: Ord + Hash,
    {
        /// Makes a new empty Map.
        #[inline]
        pub fn new() -> Self {
            Self {
                map: MapImpl::new(),
                dotted: false,
                implicit: false,
                inline: false,
            }
        }
        /// Makes a new empty Map with the given initial capacity.
        #[inline]
        pub fn with_capacity(capacity: usize) -> Self {
            let _ = capacity;
            Self::new()
        }
        /// Clears the map, removing all values.
        #[inline]
        pub fn clear(&mut self) {
            self.map.clear();
        }
        /// Returns a reference to the value corresponding to the key.
        ///
        /// The key may be any borrowed form of the map's key type, but the ordering
        /// on the borrowed form *must* match the ordering on the key type.
        #[inline]
        pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where
            K: Borrow<Q>,
            Q: Ord + Eq + Hash + ?Sized,
        {
            self.map.get(key)
        }
        /// Returns true if the map contains a value for the specified key.
        ///
        /// The key may be any borrowed form of the map's key type, but the ordering
        /// on the borrowed form *must* match the ordering on the key type.
        #[inline]
        pub fn contains_key<Q>(&self, key: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Ord + Eq + Hash + ?Sized,
        {
            self.map.contains_key(key)
        }
        /// Returns a mutable reference to the value corresponding to the key.
        ///
        /// The key may be any borrowed form of the map's key type, but the ordering
        /// on the borrowed form *must* match the ordering on the key type.
        #[inline]
        pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where
            K: Borrow<Q>,
            Q: Ord + Eq + Hash + ?Sized,
        {
            self.map.get_mut(key)
        }
        /// Returns the key-value pair matching the given key.
        ///
        /// The key may be any borrowed form of the map's key type, but the ordering
        /// on the borrowed form *must* match the ordering on the key type.
        #[inline]
        pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
        where
            K: Borrow<Q>,
            Q: ?Sized + Ord + Eq + Hash,
        {
            self.map.get_key_value(key)
        }
        /// Inserts a key-value pair into the map.
        ///
        /// If the map did not have this key present, `None` is returned.
        ///
        /// If the map did have this key present, the value is updated, and the old
        /// value is returned. The key is not updated, though; this matters for
        /// types that can be `==` without being identical.
        #[inline]
        pub fn insert(&mut self, k: K, v: V) -> Option<V> {
            self.map.insert(k, v)
        }
        /// Removes a key from the map, returning the value at the key if the key
        /// was previously in the map.
        ///
        /// The key may be any borrowed form of the map's key type, but the ordering
        /// on the borrowed form *must* match the ordering on the key type.
        #[inline]
        pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
        where
            K: Borrow<Q>,
            Q: Ord + Eq + Hash + ?Sized,
        {
            { self.map.remove(key) }
        }
        /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
        #[inline]
        pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            K: Borrow<Q>,
            Q: Ord + Eq + Hash + ?Sized,
        {
            { self.map.remove_entry(key) }
        }
        /// Retains only the elements specified by the `keep` predicate.
        ///
        /// In other words, remove all pairs `(k, v)` for which `keep(&k, &mut v)`
        /// returns `false`.
        ///
        /// The elements are visited in iteration order.
        #[inline]
        pub fn retain<F>(&mut self, mut keep: F)
        where
            K: AsRef<str>,
            F: FnMut(&str, &mut V) -> bool,
        {
            self.map.retain(|key, value| keep(key.as_ref(), value));
        }
        /// Gets the given key's corresponding entry in the map for in-place
        /// manipulation.
        pub fn entry<S>(&mut self, key: S) -> Entry<'_, K, V>
        where
            S: Into<K>,
        {
            use alloc::collections::btree_map::Entry as EntryImpl;
            match self.map.entry(key.into()) {
                EntryImpl::Vacant(vacant) => Entry::Vacant(VacantEntry { vacant }),
                EntryImpl::Occupied(occupied) => {
                    Entry::Occupied(OccupiedEntry { occupied })
                }
            }
        }
        /// Returns the number of elements in the map.
        #[inline]
        pub fn len(&self) -> usize {
            self.map.len()
        }
        /// Returns true if the map contains no elements.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
        /// Gets an iterator over the entries of the map.
        #[inline]
        pub fn iter(&self) -> Iter<'_, K, V> {
            Iter { iter: self.map.iter() }
        }
        /// Gets a mutable iterator over the entries of the map.
        #[inline]
        pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
            IterMut {
                iter: self.map.iter_mut(),
            }
        }
        /// Gets an iterator over the keys of the map.
        #[inline]
        pub fn keys(&self) -> Keys<'_, K, V> {
            Keys { iter: self.map.keys() }
        }
        /// Gets an iterator over the values of the map.
        #[inline]
        pub fn values(&self) -> Values<'_, K, V> {
            Values { iter: self.map.values() }
        }
        /// Scan through each key-value pair in the map and keep those where the
        /// closure `keep` returns `true`.
        ///
        /// The elements are visited in order, and remaining elements keep their
        /// order.
        ///
        /// Computes in **O(n)** time (average).
        #[allow(unused_mut)]
        pub(crate) fn mut_entries<F>(&mut self, mut op: F)
        where
            F: FnMut(&mut K, &mut V),
        {
            {
                self.map = core::mem::take(&mut self.map)
                    .into_iter()
                    .map(move |(mut k, mut v)| {
                        op(&mut k, &mut v);
                        (k, v)
                    })
                    .collect();
            }
        }
    }
    impl<K, V> Map<K, V>
    where
        K: Ord,
    {
        pub(crate) fn is_dotted(&self) -> bool {
            self.dotted
        }
        pub(crate) fn is_implicit(&self) -> bool {
            self.implicit
        }
        pub(crate) fn is_inline(&self) -> bool {
            self.inline
        }
        pub(crate) fn set_implicit(&mut self, yes: bool) {
            self.implicit = yes;
        }
        pub(crate) fn set_dotted(&mut self, yes: bool) {
            self.dotted = yes;
        }
        pub(crate) fn set_inline(&mut self, yes: bool) {
            self.inline = yes;
        }
    }
    impl<K, V> Default for Map<K, V>
    where
        K: Ord + Hash,
    {
        #[inline]
        fn default() -> Self {
            Self::new()
        }
    }
    impl<K: Clone, V: Clone> Clone for Map<K, V> {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                map: self.map.clone(),
                dotted: self.dotted,
                implicit: self.implicit,
                inline: self.inline,
            }
        }
    }
    impl<K: Eq + Hash, V: PartialEq> PartialEq for Map<K, V> {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.map.eq(&other.map)
        }
    }
    /// Access an element of this map. Panics if the given key is not present in the
    /// map.
    impl<K, V, Q> ops::Index<&Q> for Map<K, V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + Eq + Hash + ?Sized,
    {
        type Output = V;
        fn index(&self, index: &Q) -> &V {
            self.map.index(index)
        }
    }
    /// Mutably access an element of this map. Panics if the given key is not
    /// present in the map.
    impl<K, V, Q> ops::IndexMut<&Q> for Map<K, V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + Eq + Hash + ?Sized,
    {
        fn index_mut(&mut self, index: &Q) -> &mut V {
            self.map.get_mut(index).expect("no entry found for key")
        }
    }
    impl<K: Debug, V: Debug> Debug for Map<K, V> {
        #[inline]
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
            self.map.fmt(formatter)
        }
    }
    impl<K: Ord + Hash, V> FromIterator<(K, V)> for Map<K, V> {
        fn from_iter<T>(iter: T) -> Self
        where
            T: IntoIterator<Item = (K, V)>,
        {
            Self {
                map: FromIterator::from_iter(iter),
                dotted: false,
                implicit: false,
                inline: false,
            }
        }
    }
    impl<K: Ord + Hash, V> Extend<(K, V)> for Map<K, V> {
        fn extend<T>(&mut self, iter: T)
        where
            T: IntoIterator<Item = (K, V)>,
        {
            self.map.extend(iter);
        }
    }
    /// A view into a single entry in a map, which may either be vacant or occupied.
    /// This enum is constructed from the [`entry`] method on [`Map`].
    ///
    /// [`entry`]: struct.Map.html#method.entry
    /// [`Map`]: struct.Map.html
    pub enum Entry<'a, K, V> {
        /// A vacant Entry.
        Vacant(VacantEntry<'a, K, V>),
        /// An occupied Entry.
        Occupied(OccupiedEntry<'a, K, V>),
    }
    /// A vacant Entry. It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    pub struct VacantEntry<'a, K, V> {
        vacant: VacantEntryImpl<'a, K, V>,
    }
    /// An occupied Entry. It is part of the [`Entry`] enum.
    ///
    /// [`Entry`]: enum.Entry.html
    pub struct OccupiedEntry<'a, K, V> {
        occupied: OccupiedEntryImpl<'a, K, V>,
    }
    type VacantEntryImpl<'a, K, V> = btree_map::VacantEntry<'a, K, V>;
    type OccupiedEntryImpl<'a, K, V> = btree_map::OccupiedEntry<'a, K, V>;
    impl<'a, K: Ord, V> Entry<'a, K, V> {
        /// Returns a reference to this entry's key.
        pub fn key(&self) -> &K {
            match *self {
                Entry::Vacant(ref e) => e.key(),
                Entry::Occupied(ref e) => e.key(),
            }
        }
        /// Ensures a value is in the entry by inserting the default if empty, and
        /// returns a mutable reference to the value in the entry.
        pub fn or_insert(self, default: V) -> &'a mut V {
            match self {
                Entry::Vacant(entry) => entry.insert(default),
                Entry::Occupied(entry) => entry.into_mut(),
            }
        }
        /// Ensures a value is in the entry by inserting the result of the default
        /// function if empty, and returns a mutable reference to the value in the
        /// entry.
        pub fn or_insert_with<F>(self, default: F) -> &'a mut V
        where
            F: FnOnce() -> V,
        {
            match self {
                Entry::Vacant(entry) => entry.insert(default()),
                Entry::Occupied(entry) => entry.into_mut(),
            }
        }
    }
    impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
        /// Gets a reference to the key that would be used when inserting a value
        /// through the `VacantEntry`.
        #[inline]
        pub fn key(&self) -> &K {
            self.vacant.key()
        }
        /// Sets the value of the entry with the `VacantEntry`'s key, and returns a
        /// mutable reference to it.
        #[inline]
        pub fn insert(self, value: V) -> &'a mut V {
            self.vacant.insert(value)
        }
    }
    impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
        /// Gets a reference to the key in the entry.
        #[inline]
        pub fn key(&self) -> &K {
            self.occupied.key()
        }
        /// Gets a reference to the value in the entry.
        #[inline]
        pub fn get(&self) -> &V {
            self.occupied.get()
        }
        /// Gets a mutable reference to the value in the entry.
        #[inline]
        pub fn get_mut(&mut self) -> &mut V {
            self.occupied.get_mut()
        }
        /// Converts the entry into a mutable reference to its value.
        #[inline]
        pub fn into_mut(self) -> &'a mut V {
            self.occupied.into_mut()
        }
        /// Sets the value of the entry with the `OccupiedEntry`'s key, and returns
        /// the entry's old value.
        #[inline]
        pub fn insert(&mut self, value: V) -> V {
            self.occupied.insert(value)
        }
        /// Takes the value of the entry out of the map, and returns it.
        #[inline]
        pub fn remove(self) -> V {
            { self.occupied.remove() }
        }
    }
    impl<'a, K, V> IntoIterator for &'a Map<K, V> {
        type Item = (&'a K, &'a V);
        type IntoIter = Iter<'a, K, V>;
        #[inline]
        fn into_iter(self) -> Self::IntoIter {
            Iter { iter: self.map.iter() }
        }
    }
    /// An iterator over a `toml::Map`'s entries.
    pub struct Iter<'a, K, V> {
        iter: IterImpl<'a, K, V>,
    }
    type IterImpl<'a, K, V> = btree_map::Iter<'a, K, V>;
    impl<'a, K, V> Iterator for Iter<'a, K, V> {
        type Item = (&'a K, &'a V);
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter.next_back()
        }
    }
    impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
        #[inline]
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    impl<'a, K, V> IntoIterator for &'a mut Map<K, V> {
        type Item = (&'a K, &'a mut V);
        type IntoIter = IterMut<'a, K, V>;
        #[inline]
        fn into_iter(self) -> Self::IntoIter {
            IterMut {
                iter: self.map.iter_mut(),
            }
        }
    }
    /// A mutable iterator over a `toml::Map`'s entries.
    pub struct IterMut<'a, K, V> {
        iter: IterMutImpl<'a, K, V>,
    }
    type IterMutImpl<'a, K, V> = btree_map::IterMut<'a, K, V>;
    impl<'a, K, V> Iterator for IterMut<'a, K, V> {
        type Item = (&'a K, &'a mut V);
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter.next_back()
        }
    }
    impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
        #[inline]
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    impl<K, V> IntoIterator for Map<K, V> {
        type Item = (K, V);
        type IntoIter = IntoIter<K, V>;
        #[inline]
        fn into_iter(self) -> Self::IntoIter {
            IntoIter {
                iter: self.map.into_iter(),
            }
        }
    }
    /// An owning iterator over a `toml::Map`'s entries.
    pub struct IntoIter<K, V> {
        iter: IntoIterImpl<K, V>,
    }
    type IntoIterImpl<K, V> = btree_map::IntoIter<K, V>;
    impl<K, V> Iterator for IntoIter<K, V> {
        type Item = (K, V);
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter.next_back()
        }
    }
    impl<K, V> ExactSizeIterator for IntoIter<K, V> {
        #[inline]
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    /// An iterator over a `toml::Map`'s keys.
    pub struct Keys<'a, K, V> {
        iter: KeysImpl<'a, K, V>,
    }
    type KeysImpl<'a, K, V> = btree_map::Keys<'a, K, V>;
    impl<'a, K, V> Iterator for Keys<'a, K, V> {
        type Item = &'a K;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter.next_back()
        }
    }
    impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {
        #[inline]
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
    /// An iterator over a `toml::Map`'s values.
    pub struct Values<'a, K, V> {
        iter: ValuesImpl<'a, K, V>,
    }
    type ValuesImpl<'a, K, V> = btree_map::Values<'a, K, V>;
    impl<'a, K, V> Iterator for Values<'a, K, V> {
        type Item = &'a V;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
    }
    impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.iter.next_back()
        }
    }
    impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {
        #[inline]
        fn len(&self) -> usize {
            self.iter.len()
        }
    }
}
pub mod value {
    //! Definition of a TOML [value][Value]
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use core::fmt;
    use core::hash::Hash;
    use core::mem::discriminant;
    use core::ops;
    use std::collections::HashMap;
    use serde_core::de;
    use serde_core::de::IntoDeserializer;
    use serde_core::ser;
    use crate::alloc_prelude::*;
    pub use toml_datetime::{Date, Datetime, DatetimeParseError, Offset, Time};
    /// Type representing a TOML array, payload of the `Value::Array` variant
    pub type Array = Vec<Value>;
    #[doc(no_inline)]
    pub use crate::Table;
    /// Representation of a TOML value.
    pub enum Value {
        /// Represents a TOML string
        String(String),
        /// Represents a TOML integer
        Integer(i64),
        /// Represents a TOML float
        Float(f64),
        /// Represents a TOML boolean
        Boolean(bool),
        /// Represents a TOML datetime
        Datetime(Datetime),
        /// Represents a TOML array
        Array(Array),
        /// Represents a TOML table
        Table(Table),
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Value {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Value {
        #[inline]
        fn eq(&self, other: &Value) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (Value::String(__self_0), Value::String(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Integer(__self_0), Value::Integer(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Float(__self_0), Value::Float(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Boolean(__self_0), Value::Boolean(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Datetime(__self_0), Value::Datetime(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Array(__self_0), Value::Array(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (Value::Table(__self_0), Value::Table(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Value {
        #[inline]
        fn clone(&self) -> Value {
            match self {
                Value::String(__self_0) => {
                    Value::String(::core::clone::Clone::clone(__self_0))
                }
                Value::Integer(__self_0) => {
                    Value::Integer(::core::clone::Clone::clone(__self_0))
                }
                Value::Float(__self_0) => {
                    Value::Float(::core::clone::Clone::clone(__self_0))
                }
                Value::Boolean(__self_0) => {
                    Value::Boolean(::core::clone::Clone::clone(__self_0))
                }
                Value::Datetime(__self_0) => {
                    Value::Datetime(::core::clone::Clone::clone(__self_0))
                }
                Value::Array(__self_0) => {
                    Value::Array(::core::clone::Clone::clone(__self_0))
                }
                Value::Table(__self_0) => {
                    Value::Table(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Value {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Value::String(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "String",
                        &__self_0,
                    )
                }
                Value::Integer(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Integer",
                        &__self_0,
                    )
                }
                Value::Float(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Float",
                        &__self_0,
                    )
                }
                Value::Boolean(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Boolean",
                        &__self_0,
                    )
                }
                Value::Datetime(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Datetime",
                        &__self_0,
                    )
                }
                Value::Array(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Array",
                        &__self_0,
                    )
                }
                Value::Table(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Table",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl Value {
        /// Convert a `T` into `toml::Value` which is an enum that can represent
        /// any valid TOML data.
        ///
        /// This conversion can fail if `T`'s implementation of `Serialize` decides to
        /// fail, or if `T` contains a map with non-string keys.
        pub fn try_from<T>(value: T) -> Result<Self, crate::ser::Error>
        where
            T: ser::Serialize,
        {
            value.serialize(ValueSerializer)
        }
        /// Interpret a `toml::Value` as an instance of type `T`.
        ///
        /// This conversion can fail if the structure of the `Value` does not match the
        /// structure expected by `T`, for example if `T` is a struct type but the
        /// `Value` contains something other than a TOML table. It can also fail if the
        /// structure is correct but `T`'s implementation of `Deserialize` decides that
        /// something is wrong with the data, for example required struct fields are
        /// missing from the TOML map or some number is too big to fit in the expected
        /// primitive type.
        pub fn try_into<'de, T>(self) -> Result<T, crate::de::Error>
        where
            T: de::Deserialize<'de>,
        {
            de::Deserialize::deserialize(self)
        }
        /// Index into a TOML array or map. A string index can be used to access a
        /// value in a map, and a usize index can be used to access an element of an
        /// array.
        ///
        /// Returns `None` if the type of `self` does not match the type of the
        /// index, for example if the index is a string and `self` is an array or a
        /// number. Also returns `None` if the given key does not exist in the map
        /// or the given index is not within the bounds of the array.
        pub fn get<I: Index>(&self, index: I) -> Option<&Self> {
            index.index(self)
        }
        /// Mutably index into a TOML array or map. A string index can be used to
        /// access a value in a map, and a usize index can be used to access an
        /// element of an array.
        ///
        /// Returns `None` if the type of `self` does not match the type of the
        /// index, for example if the index is a string and `self` is an array or a
        /// number. Also returns `None` if the given key does not exist in the map
        /// or the given index is not within the bounds of the array.
        pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Self> {
            index.index_mut(self)
        }
        /// Extracts the integer value if it is an integer.
        pub fn as_integer(&self) -> Option<i64> {
            match *self {
                Self::Integer(i) => Some(i),
                _ => None,
            }
        }
        /// Tests whether this value is an integer.
        pub fn is_integer(&self) -> bool {
            self.as_integer().is_some()
        }
        /// Extracts the float value if it is a float.
        pub fn as_float(&self) -> Option<f64> {
            match *self {
                Self::Float(f) => Some(f),
                _ => None,
            }
        }
        /// Tests whether this value is a float.
        pub fn is_float(&self) -> bool {
            self.as_float().is_some()
        }
        /// Extracts the boolean value if it is a boolean.
        pub fn as_bool(&self) -> Option<bool> {
            match *self {
                Self::Boolean(b) => Some(b),
                _ => None,
            }
        }
        /// Tests whether this value is a boolean.
        pub fn is_bool(&self) -> bool {
            self.as_bool().is_some()
        }
        /// Extracts the string of this value if it is a string.
        pub fn as_str(&self) -> Option<&str> {
            match *self {
                Self::String(ref s) => Some(&**s),
                _ => None,
            }
        }
        /// Tests if this value is a string.
        pub fn is_str(&self) -> bool {
            self.as_str().is_some()
        }
        /// Extracts the datetime value if it is a datetime.
        ///
        /// Note that a parsed TOML value will only contain ISO 8601 dates. An
        /// example date is:
        ///
        /// ```notrust
        /// 1979-05-27T07:32:00Z
        /// ```
        pub fn as_datetime(&self) -> Option<&Datetime> {
            match *self {
                Self::Datetime(ref s) => Some(s),
                _ => None,
            }
        }
        /// Tests whether this value is a datetime.
        pub fn is_datetime(&self) -> bool {
            self.as_datetime().is_some()
        }
        /// Extracts the array value if it is an array.
        pub fn as_array(&self) -> Option<&Vec<Self>> {
            match *self {
                Self::Array(ref s) => Some(s),
                _ => None,
            }
        }
        /// Extracts the array value if it is an array.
        pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
            match *self {
                Self::Array(ref mut s) => Some(s),
                _ => None,
            }
        }
        /// Tests whether this value is an array.
        pub fn is_array(&self) -> bool {
            self.as_array().is_some()
        }
        /// Extracts the table value if it is a table.
        pub fn as_table(&self) -> Option<&Table> {
            match *self {
                Self::Table(ref s) => Some(s),
                _ => None,
            }
        }
        /// Extracts the table value if it is a table.
        pub fn as_table_mut(&mut self) -> Option<&mut Table> {
            match *self {
                Self::Table(ref mut s) => Some(s),
                _ => None,
            }
        }
        /// Tests whether this value is a table.
        pub fn is_table(&self) -> bool {
            self.as_table().is_some()
        }
        /// Tests whether this and another value have the same type.
        pub fn same_type(&self, other: &Self) -> bool {
            discriminant(self) == discriminant(other)
        }
        /// Returns a human-readable representation of the type of this value.
        pub fn type_str(&self) -> &'static str {
            match *self {
                Self::String(..) => "string",
                Self::Integer(..) => "integer",
                Self::Float(..) => "float",
                Self::Boolean(..) => "boolean",
                Self::Datetime(..) => "datetime",
                Self::Array(..) => "array",
                Self::Table(..) => "table",
            }
        }
    }
    impl<I> ops::Index<I> for Value
    where
        I: Index,
    {
        type Output = Self;
        fn index(&self, index: I) -> &Self {
            self.get(index).expect("index not found")
        }
    }
    impl<I> ops::IndexMut<I> for Value
    where
        I: Index,
    {
        fn index_mut(&mut self, index: I) -> &mut Self {
            self.get_mut(index).expect("index not found")
        }
    }
    impl<'a> From<&'a str> for Value {
        #[inline]
        fn from(val: &'a str) -> Self {
            Self::String(val.to_owned())
        }
    }
    impl<V: Into<Self>> From<Vec<V>> for Value {
        fn from(val: Vec<V>) -> Self {
            Self::Array(val.into_iter().map(|v| v.into()).collect())
        }
    }
    impl<S: Into<String>, V: Into<Self>> From<BTreeMap<S, V>> for Value {
        fn from(val: BTreeMap<S, V>) -> Self {
            let table = val.into_iter().map(|(s, v)| (s.into(), v.into())).collect();
            Self::Table(table)
        }
    }
    impl<S: Into<String> + Hash + Eq, V: Into<Self>> From<HashMap<S, V>> for Value {
        fn from(val: HashMap<S, V>) -> Self {
            let table = val.into_iter().map(|(s, v)| (s.into(), v.into())).collect();
            Self::Table(table)
        }
    }
    impl From<String> for Value {
        #[inline]
        fn from(val: String) -> Value {
            Value::String(val.into())
        }
    }
    impl From<i64> for Value {
        #[inline]
        fn from(val: i64) -> Value {
            Value::Integer(val.into())
        }
    }
    impl From<i32> for Value {
        #[inline]
        fn from(val: i32) -> Value {
            Value::Integer(val.into())
        }
    }
    impl From<i8> for Value {
        #[inline]
        fn from(val: i8) -> Value {
            Value::Integer(val.into())
        }
    }
    impl From<u8> for Value {
        #[inline]
        fn from(val: u8) -> Value {
            Value::Integer(val.into())
        }
    }
    impl From<u32> for Value {
        #[inline]
        fn from(val: u32) -> Value {
            Value::Integer(val.into())
        }
    }
    impl From<f64> for Value {
        #[inline]
        fn from(val: f64) -> Value {
            Value::Float(val.into())
        }
    }
    impl From<f32> for Value {
        #[inline]
        fn from(val: f32) -> Value {
            Value::Float(val.into())
        }
    }
    impl From<bool> for Value {
        #[inline]
        fn from(val: bool) -> Value {
            Value::Boolean(val.into())
        }
    }
    impl From<Datetime> for Value {
        #[inline]
        fn from(val: Datetime) -> Value {
            Value::Datetime(val.into())
        }
    }
    impl From<Table> for Value {
        #[inline]
        fn from(val: Table) -> Value {
            Value::Table(val.into())
        }
    }
    /// Types that can be used to index a `toml::Value`
    ///
    /// Currently this is implemented for `usize` to index arrays and `str` to index
    /// tables.
    ///
    /// This trait is sealed and not intended for implementation outside of the
    /// `toml` crate.
    pub trait Index: Sealed {
        #[doc(hidden)]
        fn index<'a>(&self, val: &'a Value) -> Option<&'a Value>;
        #[doc(hidden)]
        fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value>;
    }
    /// An implementation detail that should not be implemented, this will change in
    /// the future and break code otherwise.
    #[doc(hidden)]
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T: Sealed + ?Sized> Sealed for &T {}
    impl Index for usize {
        fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
            match *val {
                Value::Array(ref a) => a.get(*self),
                _ => None,
            }
        }
        fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
            match *val {
                Value::Array(ref mut a) => a.get_mut(*self),
                _ => None,
            }
        }
    }
    impl Index for str {
        fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
            match *val {
                Value::Table(ref a) => a.get(self),
                _ => None,
            }
        }
        fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
            match *val {
                Value::Table(ref mut a) => a.get_mut(self),
                _ => None,
            }
        }
    }
    impl Index for String {
        fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
            self[..].index(val)
        }
        fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
            self[..].index_mut(val)
        }
    }
    impl<T> Index for &T
    where
        T: Index + ?Sized,
    {
        fn index<'a>(&self, val: &'a Value) -> Option<&'a Value> {
            (**self).index(val)
        }
        fn index_mut<'a>(&self, val: &'a mut Value) -> Option<&'a mut Value> {
            (**self).index_mut(val)
        }
    }
    impl fmt::Display for Value {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use serde_core::Serialize as _;
            let mut output = String::new();
            let serializer = crate::ser::ValueSerializer::new(&mut output);
            self.serialize(serializer).unwrap();
            output.fmt(f)
        }
    }
    impl core::str::FromStr for Value {
        type Err = crate::de::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            use serde_core::Deserialize as _;
            Self::deserialize(crate::de::ValueDeserializer::parse(s)?)
        }
    }
    impl ser::Serialize for Value {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            match *self {
                Self::String(ref s) => serializer.serialize_str(s),
                Self::Integer(i) => serializer.serialize_i64(i),
                Self::Float(f) => serializer.serialize_f64(f),
                Self::Boolean(b) => serializer.serialize_bool(b),
                Self::Datetime(ref s) => s.serialize(serializer),
                Self::Array(ref a) => a.serialize(serializer),
                Self::Table(ref t) => t.serialize(serializer),
            }
        }
    }
    impl<'de> de::Deserialize<'de> for Value {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            struct ValueVisitor;
            impl<'de> de::Visitor<'de> for ValueVisitor {
                type Value = Value;
                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("any valid TOML value")
                }
                fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                    Ok(Value::Boolean(value))
                }
                fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                    Ok(Value::Integer(value))
                }
                fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
                    if i64::try_from(value).is_ok() {
                        Ok(Value::Integer(value as i64))
                    } else {
                        Err(de::Error::custom("u64 value was too large"))
                    }
                }
                fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
                    Ok(Value::Integer(value.into()))
                }
                fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
                    Ok(Value::Integer(value.into()))
                }
                fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                    Ok(Value::Float(value))
                }
                fn visit_str<E>(self, value: &str) -> Result<Value, E> {
                    Ok(Value::String(value.into()))
                }
                fn visit_string<E>(self, value: String) -> Result<Value, E> {
                    Ok(Value::String(value))
                }
                fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
                where
                    D: de::Deserializer<'de>,
                {
                    de::Deserialize::deserialize(deserializer)
                }
                fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
                where
                    V: de::SeqAccess<'de>,
                {
                    let mut vec = Vec::new();
                    while let Some(elem) = visitor.next_element()? {
                        vec.push(elem);
                    }
                    Ok(Value::Array(vec))
                }
                fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
                where
                    V: de::MapAccess<'de>,
                {
                    let key = match toml_datetime::de::VisitMap::next_key_seed(
                        &mut visitor,
                    )? {
                        Some(toml_datetime::de::VisitMap::Datetime(datetime)) => {
                            return Ok(Value::Datetime(datetime));
                        }
                        None => return Ok(Value::Table(Table::new())),
                        Some(toml_datetime::de::VisitMap::Key(key)) => key,
                    };
                    let mut map = Table::new();
                    map.insert(key.into_owned(), visitor.next_value()?);
                    while let Some(key) = visitor.next_key::<String>()? {
                        if let crate::map::Entry::Vacant(vacant) = map.entry(&key) {
                            vacant.insert(visitor.next_value()?);
                        } else {
                            let msg = ::alloc::__export::must_use({
                                ::alloc::fmt::format(
                                    format_args!("duplicate key: `{0}`", key),
                                )
                            });
                            return Err(de::Error::custom(msg));
                        }
                    }
                    Ok(Value::Table(map))
                }
            }
            deserializer.deserialize_any(ValueVisitor)
        }
    }
    impl<'de> de::Deserializer<'de> for Value {
        type Error = crate::de::Error;
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            match self {
                Self::Boolean(v) => visitor.visit_bool(v),
                Self::Integer(n) => visitor.visit_i64(n),
                Self::Float(n) => visitor.visit_f64(n),
                Self::String(v) => visitor.visit_string(v),
                Self::Datetime(v) => visitor.visit_string(v.to_string()),
                Self::Array(v) => {
                    let len = v.len();
                    let mut deserializer = SeqDeserializer::new(v);
                    let seq = visitor.visit_seq(&mut deserializer)?;
                    let remaining = deserializer.iter.len();
                    if remaining == 0 {
                        Ok(seq)
                    } else {
                        Err(de::Error::invalid_length(len, &"fewer elements in array"))
                    }
                }
                Self::Table(v) => {
                    let len = v.len();
                    let mut deserializer = MapDeserializer::new(v);
                    let map = visitor.visit_map(&mut deserializer)?;
                    let remaining = deserializer.iter.len();
                    if remaining == 0 {
                        Ok(map)
                    } else {
                        Err(de::Error::invalid_length(len, &"fewer elements in map"))
                    }
                }
            }
        }
        #[inline]
        fn deserialize_enum<V>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            match self {
                Self::String(variant) => visitor.visit_enum(variant.into_deserializer()),
                Self::Table(variant) => {
                    if variant.is_empty() {
                        Err(
                            crate::de::Error::custom(
                                "wanted exactly 1 element, found 0 elements",
                                None,
                            ),
                        )
                    } else if variant.len() != 1 {
                        Err(
                            crate::de::Error::custom(
                                "wanted exactly 1 element, more than 1 element",
                                None,
                            ),
                        )
                    } else {
                        let deserializer = MapDeserializer::new(variant);
                        visitor.visit_enum(deserializer)
                    }
                }
                _ => {
                    Err(
                        de::Error::invalid_type(
                            de::Unexpected::UnitVariant,
                            &"string only",
                        ),
                    )
                }
            }
        }
        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            visitor.visit_some(self)
        }
        fn deserialize_newtype_struct<V>(
            self,
            _name: &'static str,
            visitor: V,
        ) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            visitor.visit_newtype_struct(self)
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
    }
    pub(crate) struct SeqDeserializer {
        iter: vec::IntoIter<Value>,
    }
    impl SeqDeserializer {
        fn new(vec: Vec<Value>) -> Self {
            Self { iter: vec.into_iter() }
        }
    }
    impl<'de> de::SeqAccess<'de> for SeqDeserializer {
        type Error = crate::de::Error;
        fn next_element_seed<T>(
            &mut self,
            seed: T,
        ) -> Result<Option<T::Value>, crate::de::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            match self.iter.next() {
                Some(value) => seed.deserialize(value).map(Some),
                None => Ok(None),
            }
        }
        fn size_hint(&self) -> Option<usize> {
            match self.iter.size_hint() {
                (lower, Some(upper)) if lower == upper => Some(upper),
                _ => None,
            }
        }
    }
    pub(crate) struct MapDeserializer {
        iter: <Table as IntoIterator>::IntoIter,
        value: Option<(String, Value)>,
    }
    impl MapDeserializer {
        fn new(map: Table) -> Self {
            Self {
                iter: map.into_iter(),
                value: None,
            }
        }
    }
    impl<'de> de::MapAccess<'de> for MapDeserializer {
        type Error = crate::de::Error;
        fn next_key_seed<T>(
            &mut self,
            seed: T,
        ) -> Result<Option<T::Value>, crate::de::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            match self.iter.next() {
                Some((key, value)) => {
                    self.value = Some((key.clone(), value));
                    seed.deserialize(Value::String(key)).map(Some)
                }
                None => Ok(None),
            }
        }
        fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, crate::de::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            let (key, res) = match self.value.take() {
                Some((key, value)) => (key, seed.deserialize(value)),
                None => return Err(de::Error::custom("value is missing")),
            };
            res.map_err(|mut error| {
                error.add_key(key);
                error
            })
        }
        fn size_hint(&self) -> Option<usize> {
            match self.iter.size_hint() {
                (lower, Some(upper)) if lower == upper => Some(upper),
                _ => None,
            }
        }
    }
    impl<'de> de::EnumAccess<'de> for MapDeserializer {
        type Error = crate::de::Error;
        type Variant = MapEnumDeserializer;
        fn variant_seed<V>(
            mut self,
            seed: V,
        ) -> Result<(V::Value, Self::Variant), Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            use de::Error;
            let (key, value) = match self.iter.next() {
                Some(pair) => pair,
                None => {
                    return Err(
                        Error::custom(
                            "expected table with exactly 1 entry, found empty table",
                        ),
                    );
                }
            };
            let val = seed.deserialize(key.into_deserializer())?;
            let variant = MapEnumDeserializer::new(value);
            Ok((val, variant))
        }
    }
    /// Deserializes table values into enum variants.
    pub(crate) struct MapEnumDeserializer {
        value: Value,
    }
    impl MapEnumDeserializer {
        pub(crate) fn new(value: Value) -> Self {
            Self { value }
        }
    }
    impl<'de> de::VariantAccess<'de> for MapEnumDeserializer {
        type Error = crate::de::Error;
        fn unit_variant(self) -> Result<(), Self::Error> {
            use de::Error;
            match self.value {
                Value::Array(values) => {
                    if values.is_empty() {
                        Ok(())
                    } else {
                        Err(Error::custom("expected empty array"))
                    }
                }
                Value::Table(values) => {
                    if values.is_empty() {
                        Ok(())
                    } else {
                        Err(Error::custom("expected empty table"))
                    }
                }
                e => {
                    Err(
                        Error::custom(
                            ::alloc::__export::must_use({
                                ::alloc::fmt::format(
                                    format_args!("expected table, found {0}", e.type_str()),
                                )
                            }),
                        ),
                    )
                }
            }
        }
        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            seed.deserialize(self.value.into_deserializer())
        }
        fn tuple_variant<V>(
            self,
            len: usize,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            use de::Error;
            match self.value {
                Value::Array(values) => {
                    if values.len() == len {
                        de::Deserializer::deserialize_seq(
                            values.into_deserializer(),
                            visitor,
                        )
                    } else {
                        Err(
                            Error::custom(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(
                                        format_args!("expected tuple with length {0}", len),
                                    )
                                }),
                            ),
                        )
                    }
                }
                Value::Table(values) => {
                    let tuple_values: Result<Vec<_>, _> = values
                        .into_iter()
                        .enumerate()
                        .map(|(index, (key, value))| match key.parse::<usize>() {
                            Ok(key_index) if key_index == index => Ok(value),
                            Ok(_) | Err(_) => {
                                Err(
                                    Error::custom(
                                        ::alloc::__export::must_use({
                                            ::alloc::fmt::format(
                                                format_args!(
                                                    "expected table key `{0}`, but was `{1}`",
                                                    index,
                                                    key,
                                                ),
                                            )
                                        }),
                                    ),
                                )
                            }
                        })
                        .collect();
                    let tuple_values = tuple_values?;
                    if tuple_values.len() == len {
                        de::Deserializer::deserialize_seq(
                            tuple_values.into_deserializer(),
                            visitor,
                        )
                    } else {
                        Err(
                            Error::custom(
                                ::alloc::__export::must_use({
                                    ::alloc::fmt::format(
                                        format_args!("expected tuple with length {0}", len),
                                    )
                                }),
                            ),
                        )
                    }
                }
                e => {
                    Err(
                        Error::custom(
                            ::alloc::__export::must_use({
                                ::alloc::fmt::format(
                                    format_args!("expected table, found {0}", e.type_str()),
                                )
                            }),
                        ),
                    )
                }
            }
        }
        fn struct_variant<V>(
            self,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            de::Deserializer::deserialize_struct(
                self.value.into_deserializer(),
                "",
                fields,
                visitor,
            )
        }
    }
    impl IntoDeserializer<'_, crate::de::Error> for Value {
        type Deserializer = Self;
        fn into_deserializer(self) -> Self {
            self
        }
    }
    pub(crate) struct ValueSerializer;
    impl ser::Serializer for ValueSerializer {
        type Ok = Value;
        type Error = crate::ser::Error;
        type SerializeSeq = ValueSerializeVec;
        type SerializeTuple = ValueSerializeVec;
        type SerializeTupleStruct = ValueSerializeVec;
        type SerializeTupleVariant = ValueSerializeTupleVariant;
        type SerializeMap = ValueSerializeMap;
        type SerializeStruct = ValueSerializeMap;
        type SerializeStructVariant = ValueSerializeStructVariant;
        fn serialize_bool(self, value: bool) -> Result<Value, crate::ser::Error> {
            Ok(Value::Boolean(value))
        }
        fn serialize_i8(self, value: i8) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_i16(self, value: i16) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_i32(self, value: i32) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_i64(self, value: i64) -> Result<Value, crate::ser::Error> {
            Ok(Value::Integer(value))
        }
        fn serialize_u8(self, value: u8) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_u16(self, value: u16) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_u32(self, value: u32) -> Result<Value, crate::ser::Error> {
            self.serialize_i64(value.into())
        }
        fn serialize_u64(self, value: u64) -> Result<Value, crate::ser::Error> {
            if i64::try_from(value).is_ok() {
                self.serialize_i64(value as i64)
            } else {
                Err(ser::Error::custom("u64 value was too large"))
            }
        }
        fn serialize_f32(self, value: f32) -> Result<Value, crate::ser::Error> {
            self.serialize_f64(value as f64)
        }
        fn serialize_f64(self, mut value: f64) -> Result<Value, crate::ser::Error> {
            if value.is_nan() {
                value = value.copysign(1.0);
            }
            Ok(Value::Float(value))
        }
        fn serialize_char(self, value: char) -> Result<Value, crate::ser::Error> {
            let mut s = String::new();
            s.push(value);
            self.serialize_str(&s)
        }
        fn serialize_str(self, value: &str) -> Result<Value, crate::ser::Error> {
            Ok(Value::String(value.to_owned()))
        }
        fn serialize_bytes(self, value: &[u8]) -> Result<Value, crate::ser::Error> {
            let vec = value.iter().map(|&b| Value::Integer(b.into())).collect();
            Ok(Value::Array(vec))
        }
        fn serialize_unit(self) -> Result<Value, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some("unit")))
        }
        fn serialize_unit_struct(
            self,
            name: &'static str,
        ) -> Result<Value, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some(name)))
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Value, crate::ser::Error> {
            self.serialize_str(_variant)
        }
        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Value, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(self)
        }
        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Value, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Self)?;
            let mut table = Table::new();
            table.insert(variant.to_owned(), value);
            Ok(table.into())
        }
        fn serialize_none(self) -> Result<Value, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_none())
        }
        fn serialize_some<T>(self, value: &T) -> Result<Value, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(self)
        }
        fn serialize_seq(
            self,
            len: Option<usize>,
        ) -> Result<Self::SerializeSeq, crate::ser::Error> {
            Ok(ValueSerializeVec {
                vec: Vec::with_capacity(len.unwrap_or(0)),
            })
        }
        fn serialize_tuple(
            self,
            len: usize,
        ) -> Result<Self::SerializeTuple, crate::ser::Error> {
            self.serialize_seq(Some(len))
        }
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, crate::ser::Error> {
            self.serialize_seq(Some(len))
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, crate::ser::Error> {
            Ok(ValueSerializeTupleVariant::tuple(variant, len))
        }
        fn serialize_map(
            self,
            _len: Option<usize>,
        ) -> Result<Self::SerializeMap, crate::ser::Error> {
            Ok(ValueSerializeMap {
                ser: crate::table::SerializeMap::new(),
            })
        }
        fn serialize_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, crate::ser::Error> {
            self.serialize_map(Some(len))
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStructVariant, crate::ser::Error> {
            Ok(ValueSerializeStructVariant::struct_(variant, len))
        }
    }
    pub(crate) struct ValueSerializeVec {
        vec: Vec<Value>,
    }
    impl ser::SerializeSeq for ValueSerializeVec {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_element<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            self.vec.push(Value::try_from(value)?);
            Ok(())
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            Ok(Value::Array(self.vec))
        }
    }
    impl ser::SerializeTuple for ValueSerializeVec {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_element<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeSeq::serialize_element(self, value)
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            ser::SerializeSeq::end(self)
        }
    }
    impl ser::SerializeTupleStruct for ValueSerializeVec {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_field<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeSeq::serialize_element(self, value)
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            ser::SerializeSeq::end(self)
        }
    }
    impl ser::SerializeTupleVariant for ValueSerializeVec {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_field<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeSeq::serialize_element(self, value)
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            ser::SerializeSeq::end(self)
        }
    }
    pub(crate) struct ValueSerializeMap {
        ser: crate::table::SerializeMap,
    }
    impl ser::SerializeMap for ValueSerializeMap {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_key<T>(&mut self, key: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            self.ser.serialize_key(key)
        }
        fn serialize_value<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            self.ser.serialize_value(value)
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            self.ser.end().map(Value::Table)
        }
    }
    impl ser::SerializeStruct for ValueSerializeMap {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_field<T>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeMap::serialize_key(self, key)?;
            ser::SerializeMap::serialize_value(self, value)
        }
        fn end(self) -> Result<Value, crate::ser::Error> {
            ser::SerializeMap::end(self)
        }
    }
    type ValueSerializeTupleVariant = ValueSerializeVariant<ValueSerializeVec>;
    type ValueSerializeStructVariant = ValueSerializeVariant<ValueSerializeMap>;
    pub(crate) struct ValueSerializeVariant<T> {
        variant: &'static str,
        inner: T,
    }
    impl ValueSerializeVariant<ValueSerializeVec> {
        pub(crate) fn tuple(variant: &'static str, len: usize) -> Self {
            Self {
                variant,
                inner: ValueSerializeVec {
                    vec: Vec::with_capacity(len),
                },
            }
        }
    }
    impl ValueSerializeVariant<ValueSerializeMap> {
        pub(crate) fn struct_(variant: &'static str, len: usize) -> Self {
            Self {
                variant,
                inner: ValueSerializeMap {
                    ser: crate::table::SerializeMap::with_capacity(len),
                },
            }
        }
    }
    impl ser::SerializeTupleVariant for ValueSerializeVariant<ValueSerializeVec> {
        type Ok = Value;
        type Error = crate::ser::Error;
        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeSeq::serialize_element(&mut self.inner, value)
        }
        fn end(self) -> Result<Self::Ok, Self::Error> {
            let inner = ser::SerializeSeq::end(self.inner)?;
            let mut table = Table::new();
            table.insert(self.variant.to_owned(), inner);
            Ok(Value::Table(table))
        }
    }
    impl ser::SerializeStructVariant for ValueSerializeVariant<ValueSerializeMap> {
        type Ok = Value;
        type Error = crate::ser::Error;
        #[inline]
        fn serialize_field<T>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeStruct::serialize_field(&mut self.inner, key, value)
        }
        #[inline]
        fn end(self) -> Result<Self::Ok, Self::Error> {
            let inner = ser::SerializeStruct::end(self.inner)?;
            let mut table = Table::new();
            table.insert(self.variant.to_owned(), inner);
            Ok(Value::Table(table))
        }
    }
}
pub mod de {
    //! Deserializing TOML into Rust structures.
    //!
    //! This module contains all the Serde support for deserializing TOML documents
    //! into Rust structures. Note that some top-level functions here are also
    //! provided at the top of the crate.
    mod deserializer {
        //! Deserializing TOML into Rust structures.
        //!
        //! This module contains all the Serde support for deserializing TOML documents
        //! into Rust structures. Note that some top-level functions here are also
        //! provided at the top of the crate.
        mod array {
            use serde_spanned::Spanned;
            use crate::de::DeArray;
            use crate::de::DeValue;
            use crate::de::Error;
            pub(crate) struct ArrayDeserializer<'i> {
                input: DeArray<'i>,
                span: core::ops::Range<usize>,
            }
            impl<'i> ArrayDeserializer<'i> {
                pub(crate) fn new(
                    input: DeArray<'i>,
                    span: core::ops::Range<usize>,
                ) -> Self {
                    Self { input, span }
                }
            }
            impl<'de> serde_core::Deserializer<'de> for ArrayDeserializer<'de> {
                type Error = Error;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    visitor.visit_seq(ArraySeqAccess::new(self.input))
                }
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    if serde_spanned::de::is_spanned(name) {
                        let span = self.span.clone();
                        return visitor
                            .visit_map(super::SpannedDeserializer::new(self, span));
                    }
                    self.deserialize_any(visitor)
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
            }
            impl<'de> serde_core::de::IntoDeserializer<'de, Error>
            for ArrayDeserializer<'de> {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self::Deserializer {
                    self
                }
            }
            pub(crate) struct ArraySeqAccess<'i> {
                iter: alloc::vec::IntoIter<Spanned<DeValue<'i>>>,
            }
            impl<'i> ArraySeqAccess<'i> {
                pub(crate) fn new(input: DeArray<'i>) -> Self {
                    Self { iter: input.into_iter() }
                }
            }
            impl<'de> serde_core::de::SeqAccess<'de> for ArraySeqAccess<'de> {
                type Error = Error;
                fn next_element_seed<T>(
                    &mut self,
                    seed: T,
                ) -> Result<Option<T::Value>, Self::Error>
                where
                    T: serde_core::de::DeserializeSeed<'de>,
                {
                    match self.iter.next() {
                        Some(v) => {
                            let span = v.span();
                            let v = v.into_inner();
                            seed.deserialize(
                                    crate::de::ValueDeserializer::with_parts(v, span),
                                )
                                .map(Some)
                        }
                        None => Ok(None),
                    }
                }
            }
        }
        mod key {
            use serde_core::de::IntoDeserializer;
            use crate::de::DeString;
            use crate::de::Error;
            pub(crate) struct KeyDeserializer<'i> {
                span: Option<core::ops::Range<usize>>,
                key: DeString<'i>,
            }
            impl<'i> KeyDeserializer<'i> {
                pub(crate) fn new(
                    key: DeString<'i>,
                    span: Option<core::ops::Range<usize>>,
                ) -> Self {
                    KeyDeserializer { span, key }
                }
            }
            impl<'de> IntoDeserializer<'de, Error> for KeyDeserializer<'de> {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self::Deserializer {
                    self
                }
            }
            impl<'de> serde_core::de::Deserializer<'de> for KeyDeserializer<'de> {
                type Error = Error;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    self.key.into_deserializer().deserialize_any(visitor)
                }
                fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: bool = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_bool(visitor)
                }
                fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: i8 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_i8(visitor)
                }
                fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: i16 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_i16(visitor)
                }
                fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: i32 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_i32(visitor)
                }
                fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: i64 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_i64(visitor)
                }
                fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: i128 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_i128(visitor)
                }
                fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: u8 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_u8(visitor)
                }
                fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: u16 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_u16(visitor)
                }
                fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: u32 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_u32(visitor)
                }
                fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: u64 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_u64(visitor)
                }
                fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: u128 = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_u128(visitor)
                }
                fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let key: char = self
                        .key
                        .parse()
                        .map_err(serde_core::de::Error::custom)?;
                    key.into_deserializer().deserialize_char(visitor)
                }
                fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    match self.key {
                        DeString::Borrowed(s) => visitor.visit_borrowed_str(s),
                        DeString::Owned(s) => visitor.visit_string(s),
                    }
                }
                fn deserialize_enum<V>(
                    self,
                    name: &str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let _ = name;
                    let _ = variants;
                    visitor.visit_enum(self)
                }
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    if serde_spanned::de::is_spanned(name) {
                        if let Some(span) = self.span.clone() {
                            return visitor
                                .visit_map(super::SpannedDeserializer::new(self.key, span));
                        } else {
                            return Err(Error::custom("value is missing a span", None));
                        }
                    }
                    self.deserialize_any(visitor)
                }
                fn deserialize_newtype_struct<V>(
                    self,
                    _name: &'static str,
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    visitor.visit_newtype_struct(self)
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
            }
            impl<'de> serde_core::de::EnumAccess<'de> for KeyDeserializer<'de> {
                type Error = Error;
                type Variant = UnitOnly<Self::Error>;
                fn variant_seed<T>(
                    self,
                    seed: T,
                ) -> Result<(T::Value, Self::Variant), Self::Error>
                where
                    T: serde_core::de::DeserializeSeed<'de>,
                {
                    seed.deserialize(self).map(unit_only)
                }
            }
            pub(crate) struct UnitOnly<E> {
                marker: core::marker::PhantomData<E>,
            }
            fn unit_only<T, E>(t: T) -> (T, UnitOnly<E>) {
                (
                    t,
                    UnitOnly {
                        marker: core::marker::PhantomData,
                    },
                )
            }
            impl<'de, E> serde_core::de::VariantAccess<'de> for UnitOnly<E>
            where
                E: serde_core::de::Error,
            {
                type Error = E;
                fn unit_variant(self) -> Result<(), Self::Error> {
                    Ok(())
                }
                fn newtype_variant_seed<T>(
                    self,
                    _seed: T,
                ) -> Result<T::Value, Self::Error>
                where
                    T: serde_core::de::DeserializeSeed<'de>,
                {
                    Err(
                        serde_core::de::Error::invalid_type(
                            serde_core::de::Unexpected::UnitVariant,
                            &"newtype variant",
                        ),
                    )
                }
                fn tuple_variant<V>(
                    self,
                    _len: usize,
                    _visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    Err(
                        serde_core::de::Error::invalid_type(
                            serde_core::de::Unexpected::UnitVariant,
                            &"tuple variant",
                        ),
                    )
                }
                fn struct_variant<V>(
                    self,
                    _fields: &'static [&'static str],
                    _visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    Err(
                        serde_core::de::Error::invalid_type(
                            serde_core::de::Unexpected::UnitVariant,
                            &"struct variant",
                        ),
                    )
                }
            }
        }
        mod table {
            use serde_core::de::IntoDeserializer;
            use serde_spanned::Spanned;
            use crate::de::DeString;
            use crate::de::DeTable;
            use crate::de::DeValue;
            use crate::de::Error;
            use crate::map::IntoIter;
            pub(crate) struct TableDeserializer<'i> {
                span: core::ops::Range<usize>,
                items: DeTable<'i>,
            }
            impl<'i> TableDeserializer<'i> {
                pub(crate) fn new(
                    items: DeTable<'i>,
                    span: core::ops::Range<usize>,
                ) -> Self {
                    Self { span, items }
                }
            }
            impl<'de> serde_core::Deserializer<'de> for TableDeserializer<'de> {
                type Error = Error;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    visitor.visit_map(TableMapAccess::new(self))
                }
                fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    visitor.visit_some(self)
                }
                fn deserialize_newtype_struct<V>(
                    self,
                    _name: &'static str,
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    visitor.visit_newtype_struct(self)
                }
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    _fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    if serde_spanned::de::is_spanned(name) {
                        let span = self.span.clone();
                        return visitor
                            .visit_map(super::SpannedDeserializer::new(self, span));
                    }
                    self.deserialize_any(visitor)
                }
                fn deserialize_enum<V>(
                    self,
                    _name: &'static str,
                    _variants: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    if self.items.is_empty() {
                        Err(
                            Error::custom(
                                "wanted exactly 1 element, found 0 elements",
                                Some(self.span),
                            ),
                        )
                    } else if self.items.len() != 1 {
                        Err(
                            Error::custom(
                                "wanted exactly 1 element, more than 1 element",
                                Some(self.span),
                            ),
                        )
                    } else {
                        visitor.visit_enum(TableMapAccess::new(self))
                    }
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
            }
            impl<'de> IntoDeserializer<'de, Error> for TableDeserializer<'de> {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self::Deserializer {
                    self
                }
            }
            pub(crate) struct TableMapAccess<'i> {
                iter: IntoIter<Spanned<DeString<'i>>, Spanned<DeValue<'i>>>,
                span: core::ops::Range<usize>,
                value: Option<(Spanned<DeString<'i>>, Spanned<DeValue<'i>>)>,
            }
            impl<'i> TableMapAccess<'i> {
                pub(crate) fn new(input: TableDeserializer<'i>) -> Self {
                    Self {
                        iter: input.items.into_iter(),
                        span: input.span,
                        value: None,
                    }
                }
            }
            impl<'de> serde_core::de::MapAccess<'de> for TableMapAccess<'de> {
                type Error = Error;
                fn next_key_seed<K>(
                    &mut self,
                    seed: K,
                ) -> Result<Option<K::Value>, Self::Error>
                where
                    K: serde_core::de::DeserializeSeed<'de>,
                {
                    match self.iter.next() {
                        Some((k, v)) => {
                            let key_span = k.span();
                            let ret = seed
                                .deserialize(
                                    super::KeyDeserializer::new(
                                        k.clone().into_inner(),
                                        Some(key_span.clone()),
                                    ),
                                )
                                .map(Some)
                                .map_err(|mut e: Self::Error| {
                                    if e.span().is_none() {
                                        e.set_span(Some(key_span));
                                    }
                                    e
                                });
                            self.value = Some((k, v));
                            ret
                        }
                        None => Ok(None),
                    }
                }
                fn next_value_seed<V>(
                    &mut self,
                    seed: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::DeserializeSeed<'de>,
                {
                    match self.value.take() {
                        Some((k, v)) => {
                            let span = v.span();
                            seed.deserialize(
                                    crate::de::ValueDeserializer::with_parts(
                                        v.into_inner(),
                                        span.clone(),
                                    ),
                                )
                                .map_err(|mut e: Self::Error| {
                                    if e.span().is_none() {
                                        e.set_span(Some(span));
                                    }
                                    e.add_key(k.into_inner().into_owned());
                                    e
                                })
                        }
                        None => {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "no more values in next_value_seed, internal error in ValueDeserializer",
                                ),
                            );
                        }
                    }
                }
            }
            impl<'de> serde_core::de::EnumAccess<'de> for TableMapAccess<'de> {
                type Error = Error;
                type Variant = super::TableEnumDeserializer<'de>;
                fn variant_seed<V>(
                    mut self,
                    seed: V,
                ) -> Result<(V::Value, Self::Variant), Self::Error>
                where
                    V: serde_core::de::DeserializeSeed<'de>,
                {
                    let (key, value) = match self.iter.next() {
                        Some(pair) => pair,
                        None => {
                            return Err(
                                Error::custom(
                                    "expected table with exactly 1 entry, found empty table",
                                    Some(self.span),
                                ),
                            );
                        }
                    };
                    let key_span = key.span();
                    let val = seed
                        .deserialize(
                            super::KeyDeserializer::new(
                                key.into_inner(),
                                Some(key_span.clone()),
                            ),
                        )
                        .map_err(|mut e: Self::Error| {
                            if e.span().is_none() {
                                e.set_span(Some(key_span));
                            }
                            e
                        })?;
                    let value_span = value.span();
                    let value = value.into_inner();
                    let variant = super::TableEnumDeserializer::new(value, value_span);
                    Ok((val, variant))
                }
            }
        }
        mod table_enum {
            use crate::alloc_prelude::*;
            use crate::de::DeArray;
            use crate::de::DeValue;
            use crate::de::Error;
            /// Deserializes table values into enum variants.
            pub(crate) struct TableEnumDeserializer<'i> {
                value: DeValue<'i>,
                span: core::ops::Range<usize>,
            }
            impl<'i> TableEnumDeserializer<'i> {
                pub(crate) fn new(
                    value: DeValue<'i>,
                    span: core::ops::Range<usize>,
                ) -> Self {
                    TableEnumDeserializer {
                        value,
                        span,
                    }
                }
            }
            impl<'de> serde_core::de::VariantAccess<'de> for TableEnumDeserializer<'de> {
                type Error = Error;
                fn unit_variant(self) -> Result<(), Self::Error> {
                    match self.value {
                        DeValue::Array(values) => {
                            if values.is_empty() {
                                Ok(())
                            } else {
                                Err(Error::custom("expected empty array", Some(self.span)))
                            }
                        }
                        DeValue::Table(values) => {
                            if values.is_empty() {
                                Ok(())
                            } else {
                                Err(Error::custom("expected empty table", Some(self.span)))
                            }
                        }
                        e => {
                            Err(
                                Error::custom(
                                    ::alloc::__export::must_use({
                                        ::alloc::fmt::format(
                                            format_args!("expected table, found {0}", e.type_str()),
                                        )
                                    }),
                                    Some(self.span),
                                ),
                            )
                        }
                    }
                }
                fn newtype_variant_seed<T>(
                    self,
                    seed: T,
                ) -> Result<T::Value, Self::Error>
                where
                    T: serde_core::de::DeserializeSeed<'de>,
                {
                    seed.deserialize(
                        super::ValueDeserializer::with_parts(self.value, self.span),
                    )
                }
                fn tuple_variant<V>(
                    self,
                    len: usize,
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    match self.value {
                        DeValue::Array(values) => {
                            let values_span = self.span.clone();
                            let tuple_values = values;
                            if tuple_values.len() == len {
                                serde_core::de::Deserializer::deserialize_seq(
                                    super::ArrayDeserializer::new(tuple_values, values_span),
                                    visitor,
                                )
                            } else {
                                Err(
                                    Error::custom(
                                        ::alloc::__export::must_use({
                                            ::alloc::fmt::format(
                                                format_args!("expected tuple with length {0}", len),
                                            )
                                        }),
                                        Some(values_span),
                                    ),
                                )
                            }
                        }
                        DeValue::Table(values) => {
                            let values_span = self.span.clone();
                            let tuple_values: Result<DeArray<'_>, _> = values
                                .into_iter()
                                .enumerate()
                                .map(|(index, (key, value))| match key
                                    .get_ref()
                                    .parse::<usize>()
                                {
                                    Ok(key_index) if key_index == index => Ok(value),
                                    Ok(_) | Err(_) => {
                                        Err(
                                            Error::custom(
                                                ::alloc::__export::must_use({
                                                    ::alloc::fmt::format(
                                                        format_args!(
                                                            "expected table key `{0}`, but was `{1}`",
                                                            index,
                                                            key,
                                                        ),
                                                    )
                                                }),
                                                Some(key.span()),
                                            ),
                                        )
                                    }
                                })
                                .collect();
                            let tuple_values = tuple_values?;
                            if tuple_values.len() == len {
                                serde_core::de::Deserializer::deserialize_seq(
                                    super::ArrayDeserializer::new(tuple_values, values_span),
                                    visitor,
                                )
                            } else {
                                Err(
                                    Error::custom(
                                        ::alloc::__export::must_use({
                                            ::alloc::fmt::format(
                                                format_args!("expected tuple with length {0}", len),
                                            )
                                        }),
                                        Some(values_span),
                                    ),
                                )
                            }
                        }
                        e => {
                            Err(
                                Error::custom(
                                    ::alloc::__export::must_use({
                                        ::alloc::fmt::format(
                                            format_args!("expected table, found {0}", e.type_str()),
                                        )
                                    }),
                                    Some(self.span),
                                ),
                            )
                        }
                    }
                }
                fn struct_variant<V>(
                    self,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    serde_core::de::Deserializer::deserialize_struct(
                        super::ValueDeserializer::with_parts(self.value, self.span)
                            .with_struct_key_validation(),
                        "",
                        fields,
                        visitor,
                    )
                }
            }
        }
        mod value {
            use serde_core::de::IntoDeserializer as _;
            use serde_spanned::Spanned;
            use super::ArrayDeserializer;
            use super::DatetimeDeserializer;
            use super::TableDeserializer;
            use crate::alloc_prelude::*;
            use crate::de::DeString;
            use crate::de::DeTable;
            use crate::de::DeValue;
            use crate::de::Error;
            /// Deserialization implementation for TOML [values][crate::Value].
            ///
            /// # Example
            ///
            /// ```
            /// # #[cfg(feature = "parse")] {
            /// # #[cfg(feature = "display")] {
            /// use serde::Deserialize;
            ///
            /// #[derive(Deserialize)]
            /// struct Config {
            ///     title: String,
            ///     owner: Owner,
            /// }
            ///
            /// #[derive(Deserialize)]
            /// struct Owner {
            ///     name: String,
            /// }
            ///
            /// let value = r#"{ title = 'TOML Example', owner = { name = 'Lisa' } }"#;
            /// let deserializer = toml::de::ValueDeserializer::parse(value).unwrap();
            /// let config = Config::deserialize(deserializer).unwrap();
            /// assert_eq!(config.title, "TOML Example");
            /// assert_eq!(config.owner.name, "Lisa");
            /// # }
            /// # }
            /// ```
            pub struct ValueDeserializer<'i> {
                span: core::ops::Range<usize>,
                input: DeValue<'i>,
                validate_struct_keys: bool,
            }
            impl<'i> ValueDeserializer<'i> {
                /// Parse a TOML value
                pub fn parse(raw: &'i str) -> Result<Self, Error> {
                    let input = DeValue::parse(raw)?;
                    let span = input.span();
                    let input = input.into_inner();
                    Ok(Self::with_parts(input, span))
                }
                /// Deprecated, replaced with [`ValueDeserializer::parse`]
                #[deprecated(
                    since = "0.9.0",
                    note = "replaced with `ValueDeserializer::parse`"
                )]
                pub fn new(raw: &'i str) -> Result<Self, Error> {
                    Self::parse(raw)
                }
                pub(crate) fn with_parts(
                    input: DeValue<'i>,
                    span: core::ops::Range<usize>,
                ) -> Self {
                    Self {
                        input,
                        span,
                        validate_struct_keys: false,
                    }
                }
                pub(crate) fn with_struct_key_validation(mut self) -> Self {
                    self.validate_struct_keys = true;
                    self
                }
            }
            impl<'i> From<Spanned<DeValue<'i>>> for ValueDeserializer<'i> {
                fn from(root: Spanned<DeValue<'i>>) -> Self {
                    let span = root.span();
                    let root = root.into_inner();
                    Self::with_parts(root, span)
                }
            }
            impl<'de> serde_core::Deserializer<'de> for ValueDeserializer<'de> {
                type Error = Error;
                fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let span = self.span.clone();
                    match self.input {
                        DeValue::String(DeString::Owned(v)) => visitor.visit_string(v),
                        DeValue::String(DeString::Borrowed(v)) => {
                            visitor.visit_borrowed_str(v)
                        }
                        DeValue::Integer(v) => {
                            if let Some(v) = v.to_i64() {
                                visitor.visit_i64(v)
                            } else if let Some(v) = v.to_u64() {
                                visitor.visit_u64(v)
                            } else if let Some(v) = v.to_i128() {
                                visitor.visit_i128(v)
                            } else if let Some(v) = v.to_u128() {
                                visitor.visit_u128(v)
                            } else {
                                Err(Error::custom("integer number overflowed", None))
                            }
                        }
                        DeValue::Float(v) => {
                            if let Some(v) = v.to_f64() {
                                visitor.visit_f64(v)
                            } else {
                                Err(Error::custom("floating-point number overflowed", None))
                            }
                        }
                        DeValue::Boolean(v) => visitor.visit_bool(v),
                        DeValue::Datetime(v) => {
                            visitor.visit_map(DatetimeDeserializer::new(v))
                        }
                        DeValue::Array(v) => {
                            ArrayDeserializer::new(v, span.clone())
                                .deserialize_any(visitor)
                        }
                        DeValue::Table(v) => {
                            TableDeserializer::new(v, span.clone())
                                .deserialize_any(visitor)
                        }
                    }
                        .map_err(|mut e: Self::Error| {
                            if e.span().is_none() {
                                e.set_span(Some(span));
                            }
                            e
                        })
                }
                fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    self.deserialize_any(visitor)
                }
                fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let span = self.span.clone();
                    visitor
                        .visit_some(self)
                        .map_err(|mut e: Self::Error| {
                            if e.span().is_none() {
                                e.set_span(Some(span));
                            }
                            e
                        })
                }
                fn deserialize_newtype_struct<V>(
                    self,
                    _name: &'static str,
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let span = self.span.clone();
                    visitor
                        .visit_newtype_struct(self)
                        .map_err(|mut e: Self::Error| {
                            if e.span().is_none() {
                                e.set_span(Some(span));
                            }
                            e
                        })
                }
                fn deserialize_struct<V>(
                    self,
                    name: &'static str,
                    fields: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    if serde_spanned::de::is_spanned(name) {
                        let span = self.span.clone();
                        return visitor
                            .visit_map(super::SpannedDeserializer::new(self, span));
                    }
                    if toml_datetime::de::is_datetime(name) {
                        let span = self.span.clone();
                        if let DeValue::Datetime(d) = self.input {
                            return visitor
                                .visit_map(DatetimeDeserializer::new(d))
                                .map_err(|mut e: Self::Error| {
                                    if e.span().is_none() {
                                        e.set_span(Some(span));
                                    }
                                    e
                                });
                        }
                    }
                    if self.validate_struct_keys {
                        let span = self.span.clone();
                        match &self.input {
                            DeValue::Table(values) => {
                                validate_struct_keys(values, fields)
                            }
                            _ => Ok(()),
                        }
                            .map_err(|mut e: Self::Error| {
                                if e.span().is_none() {
                                    e.set_span(Some(span));
                                }
                                e
                            })?;
                    }
                    self.deserialize_any(visitor)
                }
                fn deserialize_enum<V>(
                    self,
                    name: &'static str,
                    variants: &'static [&'static str],
                    visitor: V,
                ) -> Result<V::Value, Error>
                where
                    V: serde_core::de::Visitor<'de>,
                {
                    let span = self.span.clone();
                    match self.input {
                        DeValue::String(v) => visitor.visit_enum(v.into_deserializer()),
                        DeValue::Table(v) => {
                            TableDeserializer::new(v, span.clone())
                                .deserialize_enum(name, variants, visitor)
                        }
                        _ => {
                            Err(
                                Error::custom("wanted string or table", Some(span.clone())),
                            )
                        }
                    }
                        .map_err(|mut e: Self::Error| {
                            if e.span().is_none() {
                                e.set_span(Some(span));
                            }
                            e
                        })
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
            }
            impl<'de> serde_core::de::IntoDeserializer<'de, Error>
            for ValueDeserializer<'de> {
                type Deserializer = Self;
                fn into_deserializer(self) -> Self::Deserializer {
                    self
                }
            }
            impl<'de> serde_core::de::IntoDeserializer<'de, Error>
            for Spanned<DeValue<'de>> {
                type Deserializer = ValueDeserializer<'de>;
                fn into_deserializer(self) -> Self::Deserializer {
                    ValueDeserializer::from(self)
                }
            }
            pub(crate) fn validate_struct_keys(
                table: &DeTable<'_>,
                fields: &'static [&'static str],
            ) -> Result<(), Error> {
                let extra_fields = table
                    .keys()
                    .filter_map(|key| {
                        if !fields.contains(&key.get_ref().as_ref()) {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if extra_fields.is_empty() {
                    Ok(())
                } else {
                    Err(
                        Error::custom(
                            ::alloc::__export::must_use({
                                ::alloc::fmt::format(
                                    format_args!(
                                        "unexpected keys in table: {0}, available keys: {1}",
                                        extra_fields
                                            .iter()
                                            .map(|k| k.get_ref().as_ref())
                                            .collect::<Vec<_>>()
                                            .join(", "),
                                        fields.join(", "),
                                    ),
                                )
                            }),
                            Some(extra_fields[0].span()),
                        ),
                    )
                }
            }
        }
        pub use value::ValueDeserializer;
        use crate::de::DeTable;
        use crate::de::DeValue;
        use crate::de::Error;
        use array::ArrayDeserializer;
        use key::KeyDeserializer;
        use serde_spanned::Spanned;
        use serde_spanned::de::SpannedDeserializer;
        use table::TableDeserializer;
        use table_enum::TableEnumDeserializer;
        use toml_datetime::de::DatetimeDeserializer;
        /// Deserialization for TOML [documents][crate::Table].
        ///
        /// To deserializes TOML values, instead of documents, see [`ValueDeserializer`].
        pub struct Deserializer<'i> {
            span: core::ops::Range<usize>,
            root: DeTable<'i>,
            raw: Option<&'i str>,
        }
        impl<'i> Deserializer<'i> {
            /// Parse a TOML document
            pub fn parse(raw: &'i str) -> Result<Self, Error> {
                let root = DeTable::parse(raw)?;
                let span = root.span();
                let root = root.into_inner();
                Ok(Self { span, root, raw: Some(raw) })
            }
            /// Deprecated, replaced with [`Deserializer::parse`]
            #[deprecated(since = "0.9.0", note = "replaced with `Deserializer::parse`")]
            pub fn new(raw: &'i str) -> Result<Self, Error> {
                Self::parse(raw)
            }
            fn into_table_de(self) -> ValueDeserializer<'i> {
                ValueDeserializer::with_parts(DeValue::Table(self.root), self.span)
            }
        }
        impl<'i> From<Spanned<DeTable<'i>>> for Deserializer<'i> {
            fn from(root: Spanned<DeTable<'i>>) -> Self {
                let span = root.span();
                let root = root.into_inner();
                Self { span, root, raw: None }
            }
        }
        impl<'de> serde_core::Deserializer<'de> for Deserializer<'de> {
            type Error = Error;
            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: serde_core::de::Visitor<'de>,
            {
                let raw = self.raw;
                self.into_table_de()
                    .deserialize_any(visitor)
                    .map_err(|mut e: Self::Error| {
                        e.set_input(raw);
                        e
                    })
            }
            fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
            where
                V: serde_core::de::Visitor<'de>,
            {
                let raw = self.raw;
                self.into_table_de()
                    .deserialize_option(visitor)
                    .map_err(|mut e: Self::Error| {
                        e.set_input(raw);
                        e
                    })
            }
            fn deserialize_newtype_struct<V>(
                self,
                name: &'static str,
                visitor: V,
            ) -> Result<V::Value, Error>
            where
                V: serde_core::de::Visitor<'de>,
            {
                let raw = self.raw;
                self.into_table_de()
                    .deserialize_newtype_struct(name, visitor)
                    .map_err(|mut e: Self::Error| {
                        e.set_input(raw);
                        e
                    })
            }
            fn deserialize_struct<V>(
                self,
                name: &'static str,
                fields: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Error>
            where
                V: serde_core::de::Visitor<'de>,
            {
                let raw = self.raw;
                self.into_table_de()
                    .deserialize_struct(name, fields, visitor)
                    .map_err(|mut e: Self::Error| {
                        e.set_input(raw);
                        e
                    })
            }
            fn deserialize_enum<V>(
                self,
                name: &'static str,
                variants: &'static [&'static str],
                visitor: V,
            ) -> Result<V::Value, Error>
            where
                V: serde_core::de::Visitor<'de>,
            {
                let raw = self.raw;
                self.into_table_de()
                    .deserialize_enum(name, variants, visitor)
                    .map_err(|mut e: Self::Error| {
                        e.set_input(raw);
                        e
                    })
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
        }
        impl<'de> serde_core::de::IntoDeserializer<'de, Error> for Deserializer<'de> {
            type Deserializer = Self;
            fn into_deserializer(self) -> Self::Deserializer {
                self
            }
        }
        impl<'de> serde_core::de::IntoDeserializer<'de, Error>
        for Spanned<DeTable<'de>> {
            type Deserializer = Deserializer<'de>;
            fn into_deserializer(self) -> Self::Deserializer {
                Deserializer::from(self)
            }
        }
    }
    mod error {
        use crate::alloc_prelude::*;
        /// Errors that can occur when deserializing a type.
        pub struct Error {
            message: String,
            input: Option<alloc::sync::Arc<str>>,
            keys: Vec<String>,
            span: Option<core::ops::Range<usize>>,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Error {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field4_finish(
                    f,
                    "Error",
                    "message",
                    &self.message,
                    "input",
                    &self.input,
                    "keys",
                    &self.keys,
                    "span",
                    &&self.span,
                )
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error {
                Error {
                    message: ::core::clone::Clone::clone(&self.message),
                    input: ::core::clone::Clone::clone(&self.input),
                    keys: ::core::clone::Clone::clone(&self.keys),
                    span: ::core::clone::Clone::clone(&self.span),
                }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<String>;
                let _: ::core::cmp::AssertParamIsEq<Option<alloc::sync::Arc<str>>>;
                let _: ::core::cmp::AssertParamIsEq<Vec<String>>;
                let _: ::core::cmp::AssertParamIsEq<Option<core::ops::Range<usize>>>;
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Error {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                self.message == other.message && self.input == other.input
                    && self.keys == other.keys && self.span == other.span
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Error {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.message, state);
                ::core::hash::Hash::hash(&self.input, state);
                ::core::hash::Hash::hash(&self.keys, state);
                ::core::hash::Hash::hash(&self.span, state)
            }
        }
        impl Error {
            pub(crate) fn new(
                input: alloc::sync::Arc<str>,
                error: toml_parser::ParseError,
            ) -> Self {
                let mut message = String::new();
                message.push_str(error.description());
                if let Some(expected) = error.expected() {
                    message.push_str(", expected ");
                    if expected.is_empty() {
                        message.push_str("nothing");
                    } else {
                        for (i, expected) in expected.iter().enumerate() {
                            if i != 0 {
                                message.push_str(", ");
                            }
                            match expected {
                                toml_parser::Expected::Literal(desc) => {
                                    message.push_str(&render_literal(desc));
                                }
                                toml_parser::Expected::Description(desc) => {
                                    message.push_str(desc)
                                }
                                _ => message.push_str("etc"),
                            }
                        }
                    }
                }
                let span = error.unexpected().map(|span| span.start()..span.end());
                Self {
                    message,
                    input: Some(input),
                    keys: Vec::new(),
                    span,
                }
            }
            pub(crate) fn custom<T>(
                msg: T,
                span: Option<core::ops::Range<usize>>,
            ) -> Self
            where
                T: core::fmt::Display,
            {
                Self {
                    message: msg.to_string(),
                    input: None,
                    keys: Vec::new(),
                    span,
                }
            }
            pub(crate) fn add_key(&mut self, key: String) {
                self.keys.insert(0, key);
            }
            /// What went wrong
            pub fn message(&self) -> &str {
                &self.message
            }
            /// The start/end index into the original document where the error occurred
            pub fn span(&self) -> Option<core::ops::Range<usize>> {
                self.span.clone()
            }
            pub(crate) fn set_span(&mut self, span: Option<core::ops::Range<usize>>) {
                self.span = span;
            }
            /// Provide the encoded TOML the error applies to
            pub fn set_input(&mut self, input: Option<&str>) {
                self.input = input.map(|s| s.into());
            }
        }
        impl serde_core::de::Error for Error {
            fn custom<T>(msg: T) -> Self
            where
                T: core::fmt::Display,
            {
                Self::custom(msg.to_string(), None)
            }
        }
        fn render_literal(literal: &str) -> String {
            match literal {
                "\n" => "newline".to_owned(),
                "`" => "'`'".to_owned(),
                s if s.chars().all(|c| c.is_ascii_control()) => {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("`{0}`", s.escape_debug()))
                    })
                }
                s => {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(format_args!("`{0}`", s))
                    })
                }
            }
        }
        /// Displays a TOML parse error
        ///
        /// # Example
        ///
        /// TOML parse error at line 1, column 10
        ///   |
        /// 1 | 00:32:00.a999999
        ///   |          ^
        /// Unexpected `a`
        /// Expected `digit`
        /// While parsing a Time
        /// While parsing a Date-Time
        impl core::fmt::Display for Error {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut context = false;
                if let (Some(input), Some(span)) = (&self.input, self.span()) {
                    context = true;
                    let (line, column) = translate_position(
                        input.as_bytes(),
                        span.start,
                    );
                    let line_num = line + 1;
                    let col_num = column + 1;
                    let gutter = line_num.to_string().len();
                    let content = input
                        .split('\n')
                        .nth(line)
                        .expect("valid line number");
                    let highlight_len = span.end - span.start;
                    let highlight_len = highlight_len
                        .min(content.len().saturating_sub(column));
                    f.write_fmt(
                        format_args!(
                            "TOML parse error at line {0}, column {1}\n",
                            line_num,
                            col_num,
                        ),
                    )?;
                    for _ in 0..=gutter {
                        f.write_fmt(format_args!(" "))?;
                    }
                    f.write_fmt(format_args!("|\n"))?;
                    f.write_fmt(format_args!("{0} | ", line_num))?;
                    f.write_fmt(format_args!("{0}\n", content))?;
                    for _ in 0..=gutter {
                        f.write_fmt(format_args!(" "))?;
                    }
                    f.write_fmt(format_args!("|"))?;
                    for _ in 0..=column {
                        f.write_fmt(format_args!(" "))?;
                    }
                    f.write_fmt(format_args!("^"))?;
                    for _ in 1..highlight_len {
                        f.write_fmt(format_args!("^"))?;
                    }
                    f.write_fmt(format_args!("\n"))?;
                }
                f.write_fmt(format_args!("{0}\n", self.message))?;
                if !context && !self.keys.is_empty() {
                    f.write_fmt(format_args!("in `{0}`\n", self.keys.join(".")))?;
                }
                Ok(())
            }
        }
        impl core::error::Error for Error {}
        fn translate_position(input: &[u8], index: usize) -> (usize, usize) {
            if input.is_empty() {
                return (0, index);
            }
            let safe_index = index.min(input.len() - 1);
            let column_offset = index - safe_index;
            let index = safe_index;
            let nl = input[0..index]
                .iter()
                .rev()
                .enumerate()
                .find(|(_, b)| **b == b'\n')
                .map(|(nl, _)| index - nl - 1);
            let line_start = match nl {
                Some(nl) => nl + 1,
                None => 0,
            };
            let line = input[0..line_start].iter().filter(|b| **b == b'\n').count();
            let column = core::str::from_utf8(&input[line_start..=index])
                .map(|s| s.chars().count() - 1)
                .unwrap_or_else(|_| index - line_start);
            let column = column + column_offset;
            (line, column)
        }
        pub(crate) struct TomlSink<'i, S> {
            source: toml_parser::Source<'i>,
            input: Option<alloc::sync::Arc<str>>,
            sink: S,
        }
        impl<'i, S: Default> TomlSink<'i, S> {
            pub(crate) fn new(source: toml_parser::Source<'i>) -> Self {
                Self {
                    source,
                    input: None,
                    sink: Default::default(),
                }
            }
            pub(crate) fn into_inner(self) -> S {
                self.sink
            }
        }
        impl<'i> toml_parser::ErrorSink for TomlSink<'i, Option<Error>> {
            fn report_error(&mut self, error: toml_parser::ParseError) {
                if self.sink.is_none() {
                    let input = self
                        .input
                        .get_or_insert_with(|| alloc::sync::Arc::from(
                            self.source.input(),
                        ));
                    let error = Error::new(input.clone(), error);
                    self.sink = Some(error);
                }
            }
        }
        impl<'i> toml_parser::ErrorSink for TomlSink<'i, Vec<Error>> {
            fn report_error(&mut self, error: toml_parser::ParseError) {
                let input = self
                    .input
                    .get_or_insert_with(|| alloc::sync::Arc::from(self.source.input()));
                let error = Error::new(input.clone(), error);
                self.sink.push(error);
            }
        }
        mod test_translate_position {
            use super::*;
            extern crate test;
            #[rustc_test_marker = "de::error::test_translate_position::empty"]
            #[doc(hidden)]
            pub const empty: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::empty",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 249usize,
                    start_col: 8usize,
                    end_line: 249usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(empty()),
                ),
            };
            fn empty() {
                let input = b"";
                let index = 0;
                let position = translate_position(&input[..], index);
                match (&position, &(0, 0)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::start"]
            #[doc(hidden)]
            pub const start: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::start",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 257usize,
                    start_col: 8usize,
                    end_line: 257usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(start()),
                ),
            };
            fn start() {
                let input = b"Hello";
                let index = 0;
                let position = translate_position(&input[..], index);
                match (&position, &(0, 0)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::end"]
            #[doc(hidden)]
            pub const end: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::end",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 265usize,
                    start_col: 8usize,
                    end_line: 265usize,
                    end_col: 11usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(end()),
                ),
            };
            fn end() {
                let input = b"Hello";
                let index = input.len() - 1;
                let position = translate_position(&input[..], index);
                match (&position, &(0, input.len() - 1)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::after"]
            #[doc(hidden)]
            pub const after: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::after",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 273usize,
                    start_col: 8usize,
                    end_line: 273usize,
                    end_col: 13usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(after()),
                ),
            };
            fn after() {
                let input = b"Hello";
                let index = input.len();
                let position = translate_position(&input[..], index);
                match (&position, &(0, input.len())) {
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
            #[rustc_test_marker = "de::error::test_translate_position::first_line"]
            #[doc(hidden)]
            pub const first_line: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::first_line",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 281usize,
                    start_col: 8usize,
                    end_line: 281usize,
                    end_col: 18usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(first_line()),
                ),
            };
            fn first_line() {
                let input = b"Hello\nWorld\n";
                let index = 2;
                let position = translate_position(&input[..], index);
                match (&position, &(0, 2)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::end_of_line"]
            #[doc(hidden)]
            pub const end_of_line: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::end_of_line",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 289usize,
                    start_col: 8usize,
                    end_line: 289usize,
                    end_col: 19usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(end_of_line()),
                ),
            };
            fn end_of_line() {
                let input = b"Hello\nWorld\n";
                let index = 5;
                let position = translate_position(&input[..], index);
                match (&position, &(0, 5)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::start_of_second_line"]
            #[doc(hidden)]
            pub const start_of_second_line: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::start_of_second_line",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 297usize,
                    start_col: 8usize,
                    end_line: 297usize,
                    end_col: 28usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(start_of_second_line()),
                ),
            };
            fn start_of_second_line() {
                let input = b"Hello\nWorld\n";
                let index = 6;
                let position = translate_position(&input[..], index);
                match (&position, &(1, 0)) {
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
            #[rustc_test_marker = "de::error::test_translate_position::second_line"]
            #[doc(hidden)]
            pub const second_line: test::TestDescAndFn = test::TestDescAndFn {
                desc: test::TestDesc {
                    name: test::StaticTestName(
                        "de::error::test_translate_position::second_line",
                    ),
                    ignore: false,
                    ignore_message: ::core::option::Option::None,
                    source_file: "crates/toml/src/de/error.rs",
                    start_line: 305usize,
                    start_col: 8usize,
                    end_line: 305usize,
                    end_col: 19usize,
                    compile_fail: false,
                    no_run: false,
                    should_panic: test::ShouldPanic::No,
                    test_type: test::TestType::UnitTest,
                },
                testfn: test::StaticTestFn(
                    #[coverage(off)]
                    || test::assert_test_result(second_line()),
                ),
            };
            fn second_line() {
                let input = b"Hello\nWorld\n";
                let index = 8;
                let position = translate_position(&input[..], index);
                match (&position, &(1, 2)) {
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
    }
    mod parser {
        #![allow(clippy::type_complexity)]
        use serde_spanned::Spanned;
        use toml_parser::parser::RecursionGuard;
        use toml_parser::parser::ValidateWhitespace;
        pub use dearray::DeArray;
        pub use detable::DeTable;
        pub use devalue::DeFloat;
        pub use devalue::DeInteger;
        pub use devalue::DeString;
        pub use devalue::DeValue;
        use crate::alloc_prelude::*;
        pub(crate) mod array {
            use serde_spanned::Spanned;
            use crate::de::parser::inline_table::on_inline_table;
            use crate::de::parser::value::on_scalar;
            use crate::de::{DeArray, DeValue};
            use crate::de::parser::prelude::*;
            /// ```abnf
            /// ;; Array
            ///
            /// array = array-open array-values array-close
            /// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
            /// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
            /// ```
            pub(crate) fn on_array<'i>(
                open_event: &toml_parser::parser::Event,
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> Spanned<DeValue<'i>> {
                let mut result = DeArray::new();
                let mut close_span = open_event.span();
                let mut state = State::default();
                state.open(open_event);
                while let Some(event) = input.next_token() {
                    close_span = event.span();
                    match event.kind() {
                        EventKind::StdTableOpen
                        | EventKind::ArrayTableOpen
                        | EventKind::InlineTableClose
                        | EventKind::SimpleKey
                        | EventKind::KeySep
                        | EventKind::KeyValSep
                        | EventKind::StdTableClose
                        | EventKind::ArrayTableClose => {
                            break;
                        }
                        EventKind::Error => {
                            continue;
                        }
                        EventKind::InlineTableOpen => {
                            let value = on_inline_table(event, input, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::ArrayOpen => {
                            let value = on_array(event, input, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::Scalar => {
                            let value = on_scalar(event, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::ValueSep => {
                            state.finish_value(event, &mut result);
                            state.sep_value(event);
                        }
                        EventKind::Whitespace
                        | EventKind::Comment
                        | EventKind::Newline => {
                            state.whitespace(event);
                        }
                        EventKind::ArrayClose => {
                            state.finish_value(event, &mut result);
                            state.close(open_event, event, &mut result);
                            break;
                        }
                    }
                }
                let span = open_event.span().start()..close_span.end();
                Spanned::new(span, DeValue::Array(result))
            }
            struct State<'i> {
                current_value: Option<Spanned<DeValue<'i>>>,
                trailing_start: Option<usize>,
            }
            #[automatically_derived]
            impl<'i> ::core::default::Default for State<'i> {
                #[inline]
                fn default() -> State<'i> {
                    State {
                        current_value: ::core::default::Default::default(),
                        trailing_start: ::core::default::Default::default(),
                    }
                }
            }
            impl<'i> State<'i> {
                fn open(&mut self, _open_event: &toml_parser::parser::Event) {}
                fn whitespace(&mut self, _event: &toml_parser::parser::Event) {}
                fn capture_value(
                    &mut self,
                    _event: &toml_parser::parser::Event,
                    value: Spanned<DeValue<'i>>,
                ) {
                    self.trailing_start = None;
                    self.current_value = Some(value);
                }
                fn finish_value(
                    &mut self,
                    _event: &toml_parser::parser::Event,
                    result: &mut DeArray<'i>,
                ) {
                    if let Some(value) = self.current_value.take() {
                        result.push(value);
                    }
                }
                fn sep_value(&mut self, event: &toml_parser::parser::Event) {
                    self.trailing_start = Some(event.span().end());
                }
                fn close(
                    &mut self,
                    _open_event: &toml_parser::parser::Event,
                    _close_event: &toml_parser::parser::Event,
                    _result: &mut DeArray<'i>,
                ) {}
            }
        }
        pub(crate) mod dearray {
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeValue;
            /// Type representing a TOML array, payload of the `DeValue::Array` variant
            pub struct DeArray<'i> {
                items: Vec<Spanned<DeValue<'i>>>,
                array_of_tables: bool,
            }
            #[automatically_derived]
            impl<'i> ::core::clone::Clone for DeArray<'i> {
                #[inline]
                fn clone(&self) -> DeArray<'i> {
                    DeArray {
                        items: ::core::clone::Clone::clone(&self.items),
                        array_of_tables: ::core::clone::Clone::clone(
                            &self.array_of_tables,
                        ),
                    }
                }
            }
            impl<'i> DeArray<'i> {
                /// Constructs a new, empty `DeArray`.
                ///
                /// This will not allocate until elements are pushed onto it.
                pub const fn new() -> Self {
                    Self {
                        items: Vec::new(),
                        array_of_tables: false,
                    }
                }
                /// Appends an element to the back of a collection.
                ///
                /// # Panics
                ///
                /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
                pub fn push(&mut self, value: Spanned<DeValue<'i>>) {
                    self.items.push(value);
                }
            }
            impl DeArray<'_> {
                pub(crate) fn is_array_of_tables(&self) -> bool {
                    self.array_of_tables
                }
                pub(crate) fn set_array_of_tables(&mut self, yes: bool) {
                    self.array_of_tables = yes;
                }
            }
            impl<'i> core::ops::Deref for DeArray<'i> {
                type Target = [Spanned<DeValue<'i>>];
                #[inline]
                fn deref(&self) -> &[Spanned<DeValue<'i>>] {
                    self.items.as_slice()
                }
            }
            impl<'i> core::ops::DerefMut for DeArray<'i> {
                #[inline]
                fn deref_mut(&mut self) -> &mut [Spanned<DeValue<'i>>] {
                    self.items.as_mut_slice()
                }
            }
            impl<'i> AsRef<[Spanned<DeValue<'i>>]> for DeArray<'i> {
                fn as_ref(&self) -> &[Spanned<DeValue<'i>>] {
                    &self.items
                }
            }
            impl<'i> AsMut<[Spanned<DeValue<'i>>]> for DeArray<'i> {
                fn as_mut(&mut self) -> &mut [Spanned<DeValue<'i>>] {
                    &mut self.items
                }
            }
            impl<'i> core::borrow::Borrow<[Spanned<DeValue<'i>>]> for DeArray<'i> {
                fn borrow(&self) -> &[Spanned<DeValue<'i>>] {
                    &self.items[..]
                }
            }
            impl<'i> core::borrow::BorrowMut<[Spanned<DeValue<'i>>]> for DeArray<'i> {
                fn borrow_mut(&mut self) -> &mut [Spanned<DeValue<'i>>] {
                    &mut self.items[..]
                }
            }
            impl<
                'i,
                I: core::slice::SliceIndex<[Spanned<DeValue<'i>>]>,
            > core::ops::Index<I> for DeArray<'i> {
                type Output = I::Output;
                #[inline]
                fn index(&self, index: I) -> &Self::Output {
                    self.items.index(index)
                }
            }
            impl<'a, 'i> IntoIterator for &'a DeArray<'i> {
                type Item = &'a Spanned<DeValue<'i>>;
                type IntoIter = core::slice::Iter<'a, Spanned<DeValue<'i>>>;
                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
            impl<'i> IntoIterator for DeArray<'i> {
                type Item = Spanned<DeValue<'i>>;
                type IntoIter = alloc::vec::IntoIter<Spanned<DeValue<'i>>>;
                #[inline]
                fn into_iter(self) -> Self::IntoIter {
                    self.items.into_iter()
                }
            }
            impl<'i> FromIterator<Spanned<DeValue<'i>>> for DeArray<'i> {
                #[inline]
                #[track_caller]
                fn from_iter<I: IntoIterator<Item = Spanned<DeValue<'i>>>>(
                    iter: I,
                ) -> Self {
                    Self {
                        items: iter.into_iter().collect(),
                        array_of_tables: false,
                    }
                }
            }
            impl Default for DeArray<'static> {
                #[inline]
                fn default() -> Self {
                    Self {
                        items: Default::default(),
                        array_of_tables: false,
                    }
                }
            }
            impl core::fmt::Debug for DeArray<'_> {
                #[inline]
                fn fmt(
                    &self,
                    formatter: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    self.items.fmt(formatter)
                }
            }
        }
        pub(crate) mod detable {
            use alloc::borrow::Cow;
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeString;
            use crate::de::DeValue;
            use crate::map::Map;
            /// Type representing a TOML table, payload of the `Value::Table` variant.
            ///
            /// By default it entries are stored in
            /// [lexicographic order](https://doc.rust-lang.org/std/primitive.str.html#impl-Ord-for-str)
            /// of the keys. Enable the `preserve_order` feature to store entries in the order they appear in
            /// the source file.
            pub type DeTable<'i> = Map<Spanned<DeString<'i>>, Spanned<DeValue<'i>>>;
            impl<'i> DeTable<'i> {
                /// Parse a TOML document
                pub fn parse(input: &'i str) -> Result<Spanned<Self>, crate::de::Error> {
                    let source = toml_parser::Source::new(input);
                    let mut errors = crate::de::error::TomlSink::<
                        Option<_>,
                    >::new(source);
                    let value = crate::de::parser::parse_document(source, &mut errors);
                    if let Some(err) = errors.into_inner() {
                        Err(err)
                    } else {
                        Ok(value)
                    }
                }
                /// Parse a TOML document, with best effort recovery on error
                pub fn parse_recoverable(
                    input: &'i str,
                ) -> (Spanned<Self>, Vec<crate::de::Error>) {
                    let source = toml_parser::Source::new(input);
                    let mut errors = crate::de::error::TomlSink::<Vec<_>>::new(source);
                    let value = crate::de::parser::parse_document(source, &mut errors);
                    (value, errors.into_inner())
                }
                /// Ensure no data is borrowed
                pub fn make_owned(&mut self) {
                    self.mut_entries(|k, v| {
                        let owned = core::mem::take(k.get_mut());
                        *k.get_mut() = Cow::Owned(owned.into_owned());
                        v.get_mut().make_owned();
                    });
                }
            }
        }
        pub(crate) mod devalue {
            //! Definition of a TOML [value][DeValue] for deserialization
            use alloc::borrow::Cow;
            use core::mem::discriminant;
            use core::ops;
            use serde_spanned::Spanned;
            use toml_datetime::Datetime;
            use crate::alloc_prelude::*;
            use crate::de::DeArray;
            use crate::de::DeTable;
            /// Type representing a TOML string, payload of the `DeValue::String` variant
            pub type DeString<'i> = Cow<'i, str>;
            /// Represents a TOML integer
            pub struct DeInteger<'i> {
                pub(crate) inner: DeString<'i>,
                pub(crate) radix: u32,
            }
            #[automatically_derived]
            impl<'i> ::core::clone::Clone for DeInteger<'i> {
                #[inline]
                fn clone(&self) -> DeInteger<'i> {
                    DeInteger {
                        inner: ::core::clone::Clone::clone(&self.inner),
                        radix: ::core::clone::Clone::clone(&self.radix),
                    }
                }
            }
            #[automatically_derived]
            impl<'i> ::core::fmt::Debug for DeInteger<'i> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "DeInteger",
                        "inner",
                        &self.inner,
                        "radix",
                        &&self.radix,
                    )
                }
            }
            impl DeInteger<'_> {
                pub(crate) fn to_u64(&self) -> Option<u64> {
                    u64::from_str_radix(self.inner.as_ref(), self.radix).ok()
                }
                pub(crate) fn to_i64(&self) -> Option<i64> {
                    i64::from_str_radix(self.inner.as_ref(), self.radix).ok()
                }
                pub(crate) fn to_u128(&self) -> Option<u128> {
                    u128::from_str_radix(self.inner.as_ref(), self.radix).ok()
                }
                pub(crate) fn to_i128(&self) -> Option<i128> {
                    i128::from_str_radix(self.inner.as_ref(), self.radix).ok()
                }
                /// [`from_str_radix`][i64::from_str_radix]-compatible representation of an integer
                ///
                /// Requires [`DeInteger::radix`] to interpret
                ///
                /// See [`Display`][std::fmt::Display] for a representation that includes the radix
                pub fn as_str(&self) -> &str {
                    self.inner.as_ref()
                }
                /// Numeric base of [`DeInteger::as_str`]
                pub fn radix(&self) -> u32 {
                    self.radix
                }
            }
            impl Default for DeInteger<'_> {
                fn default() -> Self {
                    Self {
                        inner: DeString::Borrowed("0"),
                        radix: 10,
                    }
                }
            }
            impl core::fmt::Display for DeInteger<'_> {
                fn fmt(
                    &self,
                    formatter: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    match self.radix {
                        2 => "0b".fmt(formatter)?,
                        8 => "0o".fmt(formatter)?,
                        10 => {}
                        16 => "0x".fmt(formatter)?,
                        _ => {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!(
                                        "we should only ever have 2, 8, 10, and 16 radix, not {0}",
                                        self.radix,
                                    ),
                                ),
                            );
                        }
                    }
                    self.as_str().fmt(formatter)?;
                    Ok(())
                }
            }
            /// Represents a TOML integer
            pub struct DeFloat<'i> {
                pub(crate) inner: DeString<'i>,
            }
            #[automatically_derived]
            impl<'i> ::core::clone::Clone for DeFloat<'i> {
                #[inline]
                fn clone(&self) -> DeFloat<'i> {
                    DeFloat {
                        inner: ::core::clone::Clone::clone(&self.inner),
                    }
                }
            }
            #[automatically_derived]
            impl<'i> ::core::fmt::Debug for DeFloat<'i> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "DeFloat",
                        "inner",
                        &&self.inner,
                    )
                }
            }
            impl DeFloat<'_> {
                pub(crate) fn to_f64(&self) -> Option<f64> {
                    let f: f64 = self.inner.as_ref().parse().ok()?;
                    if f.is_infinite() && !self.as_str().contains("inf") {
                        None
                    } else {
                        Some(f)
                    }
                }
                /// [`FromStr`][std::str::FromStr]-compatible representation of a float
                pub fn as_str(&self) -> &str {
                    self.inner.as_ref()
                }
            }
            impl Default for DeFloat<'_> {
                fn default() -> Self {
                    Self {
                        inner: DeString::Borrowed("0.0"),
                    }
                }
            }
            impl core::fmt::Display for DeFloat<'_> {
                fn fmt(
                    &self,
                    formatter: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    self.as_str().fmt(formatter)?;
                    Ok(())
                }
            }
            /// Representation of a TOML value.
            pub enum DeValue<'i> {
                /// Represents a TOML string
                String(DeString<'i>),
                /// Represents a TOML integer
                Integer(DeInteger<'i>),
                /// Represents a TOML float
                Float(DeFloat<'i>),
                /// Represents a TOML boolean
                Boolean(bool),
                /// Represents a TOML datetime
                Datetime(Datetime),
                /// Represents a TOML array
                Array(DeArray<'i>),
                /// Represents a TOML table
                Table(DeTable<'i>),
            }
            #[automatically_derived]
            impl<'i> ::core::clone::Clone for DeValue<'i> {
                #[inline]
                fn clone(&self) -> DeValue<'i> {
                    match self {
                        DeValue::String(__self_0) => {
                            DeValue::String(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Integer(__self_0) => {
                            DeValue::Integer(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Float(__self_0) => {
                            DeValue::Float(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Boolean(__self_0) => {
                            DeValue::Boolean(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Datetime(__self_0) => {
                            DeValue::Datetime(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Array(__self_0) => {
                            DeValue::Array(::core::clone::Clone::clone(__self_0))
                        }
                        DeValue::Table(__self_0) => {
                            DeValue::Table(::core::clone::Clone::clone(__self_0))
                        }
                    }
                }
            }
            #[automatically_derived]
            impl<'i> ::core::fmt::Debug for DeValue<'i> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    match self {
                        DeValue::String(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "String",
                                &__self_0,
                            )
                        }
                        DeValue::Integer(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Integer",
                                &__self_0,
                            )
                        }
                        DeValue::Float(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Float",
                                &__self_0,
                            )
                        }
                        DeValue::Boolean(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Boolean",
                                &__self_0,
                            )
                        }
                        DeValue::Datetime(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Datetime",
                                &__self_0,
                            )
                        }
                        DeValue::Array(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Array",
                                &__self_0,
                            )
                        }
                        DeValue::Table(__self_0) => {
                            ::core::fmt::Formatter::debug_tuple_field1_finish(
                                f,
                                "Table",
                                &__self_0,
                            )
                        }
                    }
                }
            }
            impl<'i> DeValue<'i> {
                /// Parse a TOML value
                pub fn parse(input: &'i str) -> Result<Spanned<Self>, crate::de::Error> {
                    let source = toml_parser::Source::new(input);
                    let mut errors = crate::de::error::TomlSink::<
                        Option<_>,
                    >::new(source);
                    let value = crate::de::parser::parse_value(source, &mut errors);
                    if let Some(err) = errors.into_inner() {
                        Err(err)
                    } else {
                        Ok(value)
                    }
                }
                /// Parse a TOML value, with best effort recovery on error
                pub fn parse_recoverable(
                    input: &'i str,
                ) -> (Spanned<Self>, Vec<crate::de::Error>) {
                    let source = toml_parser::Source::new(input);
                    let mut errors = crate::de::error::TomlSink::<Vec<_>>::new(source);
                    let value = crate::de::parser::parse_value(source, &mut errors);
                    (value, errors.into_inner())
                }
                /// Ensure no data is borrowed
                pub fn make_owned(&mut self) {
                    match self {
                        DeValue::String(v) => {
                            let owned = core::mem::take(v);
                            *v = Cow::Owned(owned.into_owned());
                        }
                        DeValue::Integer(..)
                        | DeValue::Float(..)
                        | DeValue::Boolean(..)
                        | DeValue::Datetime(..) => {}
                        DeValue::Array(v) => {
                            for e in v.iter_mut() {
                                e.get_mut().make_owned();
                            }
                        }
                        DeValue::Table(v) => v.make_owned(),
                    }
                }
                /// Index into a TOML array or map. A string index can be used to access a
                /// value in a map, and a usize index can be used to access an element of an
                /// array.
                ///
                /// Returns `None` if the type of `self` does not match the type of the
                /// index, for example if the index is a string and `self` is an array or a
                /// number. Also returns `None` if the given key does not exist in the map
                /// or the given index is not within the bounds of the array.
                pub fn get<I: Index>(&self, index: I) -> Option<&Spanned<Self>> {
                    index.index(self)
                }
                /// Extracts the integer value if it is an integer.
                pub fn as_integer(&self) -> Option<&DeInteger<'i>> {
                    match self {
                        DeValue::Integer(i) => Some(i),
                        _ => None,
                    }
                }
                /// Tests whether this value is an integer.
                pub fn is_integer(&self) -> bool {
                    self.as_integer().is_some()
                }
                /// Extracts the float value if it is a float.
                pub fn as_float(&self) -> Option<&DeFloat<'i>> {
                    match self {
                        DeValue::Float(f) => Some(f),
                        _ => None,
                    }
                }
                /// Tests whether this value is a float.
                pub fn is_float(&self) -> bool {
                    self.as_float().is_some()
                }
                /// Extracts the boolean value if it is a boolean.
                pub fn as_bool(&self) -> Option<bool> {
                    match *self {
                        DeValue::Boolean(b) => Some(b),
                        _ => None,
                    }
                }
                /// Tests whether this value is a boolean.
                pub fn is_bool(&self) -> bool {
                    self.as_bool().is_some()
                }
                /// Extracts the string of this value if it is a string.
                pub fn as_str(&self) -> Option<&str> {
                    match *self {
                        DeValue::String(ref s) => Some(&**s),
                        _ => None,
                    }
                }
                /// Tests if this value is a string.
                pub fn is_str(&self) -> bool {
                    self.as_str().is_some()
                }
                /// Extracts the datetime value if it is a datetime.
                ///
                /// Note that a parsed TOML value will only contain ISO 8601 dates. An
                /// example date is:
                ///
                /// ```notrust
                /// 1979-05-27T07:32:00Z
                /// ```
                pub fn as_datetime(&self) -> Option<&Datetime> {
                    match *self {
                        DeValue::Datetime(ref s) => Some(s),
                        _ => None,
                    }
                }
                /// Tests whether this value is a datetime.
                pub fn is_datetime(&self) -> bool {
                    self.as_datetime().is_some()
                }
                /// Extracts the array value if it is an array.
                pub fn as_array(&self) -> Option<&DeArray<'i>> {
                    match *self {
                        DeValue::Array(ref s) => Some(s),
                        _ => None,
                    }
                }
                pub(crate) fn as_array_mut(&mut self) -> Option<&mut DeArray<'i>> {
                    match self {
                        DeValue::Array(s) => Some(s),
                        _ => None,
                    }
                }
                /// Tests whether this value is an array.
                pub fn is_array(&self) -> bool {
                    self.as_array().is_some()
                }
                /// Extracts the table value if it is a table.
                pub fn as_table(&self) -> Option<&DeTable<'i>> {
                    match *self {
                        DeValue::Table(ref s) => Some(s),
                        _ => None,
                    }
                }
                pub(crate) fn as_table_mut(&mut self) -> Option<&mut DeTable<'i>> {
                    match self {
                        DeValue::Table(s) => Some(s),
                        _ => None,
                    }
                }
                /// Tests whether this value is a table.
                pub fn is_table(&self) -> bool {
                    self.as_table().is_some()
                }
                /// Tests whether this and another value have the same type.
                pub fn same_type(&self, other: &DeValue<'_>) -> bool {
                    discriminant(self) == discriminant(other)
                }
                /// Returns a human-readable representation of the type of this value.
                pub fn type_str(&self) -> &'static str {
                    match *self {
                        DeValue::String(..) => "string",
                        DeValue::Integer(..) => "integer",
                        DeValue::Float(..) => "float",
                        DeValue::Boolean(..) => "boolean",
                        DeValue::Datetime(..) => "datetime",
                        DeValue::Array(..) => "array",
                        DeValue::Table(..) => "table",
                    }
                }
            }
            impl<I> ops::Index<I> for DeValue<'_>
            where
                I: Index,
            {
                type Output = Spanned<Self>;
                fn index(&self, index: I) -> &Spanned<Self> {
                    self.get(index).expect("index not found")
                }
            }
            /// Types that can be used to index a `toml::Value`
            ///
            /// Currently this is implemented for `usize` to index arrays and `str` to index
            /// tables.
            ///
            /// This trait is sealed and not intended for implementation outside of the
            /// `toml` crate.
            pub trait Index: Sealed {
                #[doc(hidden)]
                fn index<'r, 'i>(
                    &self,
                    val: &'r DeValue<'i>,
                ) -> Option<&'r Spanned<DeValue<'i>>>;
            }
            /// An implementation detail that should not be implemented, this will change in
            /// the future and break code otherwise.
            #[doc(hidden)]
            pub trait Sealed {}
            impl Sealed for usize {}
            impl Sealed for str {}
            impl Sealed for String {}
            impl<T: Sealed + ?Sized> Sealed for &T {}
            impl Index for usize {
                fn index<'r, 'i>(
                    &self,
                    val: &'r DeValue<'i>,
                ) -> Option<&'r Spanned<DeValue<'i>>> {
                    match *val {
                        DeValue::Array(ref a) => a.get(*self),
                        _ => None,
                    }
                }
            }
            impl Index for str {
                fn index<'r, 'i>(
                    &self,
                    val: &'r DeValue<'i>,
                ) -> Option<&'r Spanned<DeValue<'i>>> {
                    match *val {
                        DeValue::Table(ref a) => a.get(self),
                        _ => None,
                    }
                }
            }
            impl Index for String {
                fn index<'r, 'i>(
                    &self,
                    val: &'r DeValue<'i>,
                ) -> Option<&'r Spanned<DeValue<'i>>> {
                    self[..].index(val)
                }
            }
            impl<T> Index for &T
            where
                T: Index + ?Sized,
            {
                fn index<'r, 'i>(
                    &self,
                    val: &'r DeValue<'i>,
                ) -> Option<&'r Spanned<DeValue<'i>>> {
                    (**self).index(val)
                }
            }
        }
        pub(crate) mod document {
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeString;
            use crate::de::DeValue;
            use crate::de::parser::key::on_key;
            use crate::de::parser::prelude::*;
            use crate::de::parser::value::value;
            use crate::de::{DeArray, DeTable};
            use crate::map::Entry;
            /// ```abnf
            /// ;; TOML
            ///
            /// toml = expression *( newline expression )
            ///
            /// expression = ( ( ws comment ) /
            ///                ( ws keyval ws [ comment ] ) /
            ///                ( ws table ws [ comment ] ) /
            ///                  ws )
            /// ```
            pub(crate) fn document<'i>(
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> Spanned<DeTable<'i>> {
                let mut state = State::default();
                while let Some(event) = input.next_token() {
                    match event.kind() {
                        EventKind::InlineTableOpen
                        | EventKind::InlineTableClose
                        | EventKind::ArrayOpen
                        | EventKind::ArrayClose
                        | EventKind::Scalar
                        | EventKind::ValueSep
                        | EventKind::Error
                        | EventKind::KeySep
                        | EventKind::KeyValSep
                        | EventKind::StdTableClose
                        | EventKind::ArrayTableClose => {
                            continue;
                        }
                        EventKind::StdTableOpen | EventKind::ArrayTableOpen => {
                            state.finish_table(errors);
                            let header = on_table(event, input, source, errors);
                            state.start_table(header, errors);
                        }
                        EventKind::SimpleKey => {
                            let (path, key) = on_key(event, input, source, errors);
                            let Some(key) = key else {
                                break;
                            };
                            let Some(next_event) = input.next_token() else {
                                break;
                            };
                            let keyval_event = if next_event.kind()
                                == EventKind::Whitespace
                            {
                                let Some(next_event) = input.next_token() else {
                                    break;
                                };
                                next_event
                            } else {
                                next_event
                            };
                            if keyval_event.kind() != EventKind::KeyValSep {
                                break;
                            }
                            if input
                                .first()
                                .map(|e| e.kind() == EventKind::Whitespace)
                                .unwrap_or(false)
                            {
                                let _ = input.next_token();
                            }
                            let value = value(input, source, errors);
                            state.capture_key_value(path, key, value, errors);
                        }
                        EventKind::Whitespace
                        | EventKind::Comment
                        | EventKind::Newline => {
                            state.capture_trailing(event);
                        }
                    }
                }
                state.finish_table(errors);
                let span = Default::default();
                Spanned::new(span, state.root)
            }
            /// ```abnf
            /// ;; Standard Table
            ///
            /// std-table = std-table-open key *( table-key-sep key) std-table-close
            ///
            /// ;; Array Table
            ///
            /// array-table = array-table-open key *( table-key-sep key) array-table-close
            /// ```
            fn on_table<'i>(
                open_event: &toml_parser::parser::Event,
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> TableHeader<'i> {
                let is_array = open_event.kind() == EventKind::ArrayTableOpen;
                let mut current_path = None;
                let mut current_key = None;
                let mut current_span = open_event.span();
                let mut current_prefix = None;
                let mut current_suffix = None;
                while let Some(event) = input.next_token() {
                    match event.kind() {
                        EventKind::InlineTableOpen
                        | EventKind::InlineTableClose
                        | EventKind::ArrayOpen
                        | EventKind::ArrayClose
                        | EventKind::Scalar
                        | EventKind::ValueSep
                        | EventKind::Error
                        | EventKind::KeySep
                        | EventKind::KeyValSep
                        | EventKind::StdTableOpen
                        | EventKind::ArrayTableOpen
                        | EventKind::Comment
                        | EventKind::Newline => {
                            continue;
                        }
                        EventKind::ArrayTableClose | EventKind::StdTableClose => {
                            current_span = current_span.append(event.span());
                            break;
                        }
                        EventKind::SimpleKey => {
                            current_prefix.get_or_insert_with(|| event.span().before());
                            let (path, key) = on_key(event, input, source, errors);
                            current_path = Some(path);
                            current_key = key;
                            current_suffix.get_or_insert_with(|| event.span().after());
                        }
                        EventKind::Whitespace => {
                            if current_key.is_some() {
                                current_suffix = Some(event.span());
                            } else {
                                current_prefix = Some(event.span());
                            }
                        }
                    }
                }
                TableHeader {
                    path: current_path.unwrap_or_default(),
                    key: current_key,
                    span: current_span,
                    is_array,
                }
            }
            struct TableHeader<'i> {
                path: Vec<Spanned<DeString<'i>>>,
                key: Option<Spanned<DeString<'i>>>,
                span: toml_parser::Span,
                is_array: bool,
            }
            struct State<'i> {
                root: DeTable<'i>,
                current_table: DeTable<'i>,
                current_header: Option<TableHeader<'i>>,
                current_position: usize,
            }
            #[automatically_derived]
            impl<'i> ::core::default::Default for State<'i> {
                #[inline]
                fn default() -> State<'i> {
                    State {
                        root: ::core::default::Default::default(),
                        current_table: ::core::default::Default::default(),
                        current_header: ::core::default::Default::default(),
                        current_position: ::core::default::Default::default(),
                    }
                }
            }
            impl<'i> State<'i> {
                fn capture_trailing(&mut self, _event: &toml_parser::parser::Event) {}
                fn capture_key_value(
                    &mut self,
                    path: Vec<Spanned<DeString<'i>>>,
                    key: Spanned<DeString<'i>>,
                    value: Spanned<DeValue<'i>>,
                    errors: &mut dyn ErrorSink,
                ) {
                    let dotted = !path.is_empty();
                    let Some(parent_table) = descend_path(
                        &mut self.current_table,
                        &path,
                        dotted,
                        errors,
                    ) else {
                        return;
                    };
                    let mixed_table_types = dotted && !parent_table.is_implicit();
                    if mixed_table_types {
                        let key_span = get_key_span(&key);
                        errors
                            .report_error(
                                ParseError::new("duplicate key").with_unexpected(key_span),
                            );
                        return;
                    }
                    let key_span = get_key_span(&key);
                    match parent_table.entry(key) {
                        Entry::Vacant(o) => {
                            o.insert(value);
                        }
                        Entry::Occupied(existing) => {
                            let old_span = get_key_span(existing.key());
                            errors
                                .report_error(
                                    ParseError::new("duplicate key")
                                        .with_unexpected(key_span)
                                        .with_context(old_span),
                                );
                        }
                    }
                }
                fn finish_table(&mut self, errors: &mut dyn ErrorSink) {
                    let prev_table = core::mem::take(&mut self.current_table);
                    if let Some(header) = self.current_header.take() {
                        let Some(key) = &header.key else {
                            return;
                        };
                        let header_span = header.span.start()..header.span.end();
                        let prev_table = Spanned::new(
                            header_span.clone(),
                            DeValue::Table(prev_table),
                        );
                        let parent_key = &header.path;
                        let dotted = false;
                        let Some(parent_table) = descend_path(
                            &mut self.root,
                            parent_key,
                            dotted,
                            errors,
                        ) else {
                            return;
                        };
                        if header.is_array {
                            let entry = parent_table
                                .entry(key.clone())
                                .or_insert_with(|| {
                                    let mut array = DeArray::new();
                                    array.set_array_of_tables(true);
                                    Spanned::new(header_span, DeValue::Array(array))
                                });
                            let Some(array) = entry
                                .as_mut()
                                .as_array_mut()
                                .filter(|a| a.is_array_of_tables()) else {
                                let key_span = get_key_span(key);
                                let old_span = entry.span();
                                let old_span = toml_parser::Span::new_unchecked(
                                    old_span.start,
                                    old_span.end,
                                );
                                errors
                                    .report_error(
                                        ParseError::new("duplicate key")
                                            .with_unexpected(key_span)
                                            .with_context(old_span),
                                    );
                                return;
                            };
                            array.push(prev_table);
                        } else {
                            let existing = parent_table.insert(key.clone(), prev_table);
                            if true {
                                if !existing.is_none() {
                                    ::core::panicking::panic(
                                        "assertion failed: existing.is_none()",
                                    )
                                }
                            }
                        }
                    } else {
                        self.root = prev_table;
                    }
                }
                fn start_table(
                    &mut self,
                    header: TableHeader<'i>,
                    errors: &mut dyn ErrorSink,
                ) {
                    if !header.is_array {
                        let root = &mut self.root;
                        if let (Some(parent_table), Some(key)) = (
                            descend_path(root, &header.path, false, errors),
                            &header.key,
                        ) {
                            if let Some((old_key, old_value)) = parent_table
                                .remove_entry(key)
                            {
                                match old_value.into_inner() {
                                    DeValue::Table(t) if t.is_implicit() && !t.is_dotted() => {
                                        self.current_table = t;
                                    }
                                    old_value => {
                                        let old_span = get_key_span(&old_key);
                                        let key_span = get_key_span(key);
                                        errors
                                            .report_error(
                                                ParseError::new("duplicate key")
                                                    .with_unexpected(key_span)
                                                    .with_context(old_span),
                                            );
                                        if let DeValue::Table(t) = old_value {
                                            self.current_table = t;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    self.current_position += 1;
                    self.current_table.set_implicit(false);
                    self.current_table.set_dotted(false);
                    self.current_header = Some(header);
                }
            }
            fn descend_path<'t, 'i>(
                mut table: &'t mut DeTable<'i>,
                path: &[Spanned<DeString<'i>>],
                dotted: bool,
                errors: &mut dyn ErrorSink,
            ) -> Option<&'t mut DeTable<'i>> {
                for key in path.iter() {
                    table = match table.entry(key.clone()) {
                        Entry::Vacant(entry) => {
                            let mut new_table = DeTable::new();
                            new_table.set_implicit(true);
                            new_table.set_dotted(dotted);
                            let value = DeValue::Table(new_table);
                            let value = Spanned::new(key.span(), value);
                            let value = entry.insert(value);
                            value.as_mut().as_table_mut().unwrap()
                        }
                        Entry::Occupied(entry) => {
                            let spanned = entry.into_mut();
                            let old_span = spanned.span();
                            match spanned.as_mut() {
                                DeValue::Array(array) => {
                                    if !array.is_array_of_tables() {
                                        let old_span = toml_parser::Span::new_unchecked(
                                            old_span.start,
                                            old_span.end,
                                        );
                                        let key_span = get_key_span(key);
                                        errors
                                            .report_error(
                                                ParseError::new(
                                                        "cannot extend value of type array with a dotted key",
                                                    )
                                                    .with_unexpected(key_span)
                                                    .with_context(old_span),
                                            );
                                        return None;
                                    }
                                    if true {
                                        if !!array.is_empty() {
                                            ::core::panicking::panic(
                                                "assertion failed: !array.is_empty()",
                                            )
                                        }
                                    }
                                    let index = array.len() - 1;
                                    let last_child = array.get_mut(index).unwrap();
                                    match last_child.as_mut() {
                                        DeValue::Table(table) => table,
                                        existing => {
                                            let old_span = toml_parser::Span::new_unchecked(
                                                old_span.start,
                                                old_span.end,
                                            );
                                            let key_span = get_key_span(key);
                                            errors
                                                .report_error(
                                                    ParseError::new(
                                                            ::alloc::__export::must_use({
                                                                ::alloc::fmt::format(
                                                                    format_args!(
                                                                        "cannot extend value of type {0} with a dotted key",
                                                                        existing.type_str(),
                                                                    ),
                                                                )
                                                            }),
                                                        )
                                                        .with_unexpected(key_span)
                                                        .with_context(old_span),
                                                );
                                            return None;
                                        }
                                    }
                                }
                                DeValue::Table(sweet_child_of_mine) => {
                                    if sweet_child_of_mine.is_inline() {
                                        let key_span = get_key_span(key);
                                        errors
                                            .report_error(
                                                ParseError::new(
                                                        "cannot extend value of type inline table with a dotted key",
                                                    )
                                                    .with_unexpected(key_span),
                                            );
                                        return None;
                                    }
                                    if dotted && sweet_child_of_mine.is_implicit() {
                                        sweet_child_of_mine.set_dotted(true);
                                    }
                                    let mixed_table_types = dotted
                                        && !sweet_child_of_mine.is_implicit();
                                    if mixed_table_types {
                                        let key_span = get_key_span(key);
                                        errors
                                            .report_error(
                                                ParseError::new("duplicate key").with_unexpected(key_span),
                                            );
                                        return None;
                                    }
                                    sweet_child_of_mine
                                }
                                existing => {
                                    let old_span = toml_parser::Span::new_unchecked(
                                        old_span.start,
                                        old_span.end,
                                    );
                                    let key_span = get_key_span(key);
                                    errors
                                        .report_error(
                                            ParseError::new(
                                                    ::alloc::__export::must_use({
                                                        ::alloc::fmt::format(
                                                            format_args!(
                                                                "cannot extend value of type {0} with a dotted key",
                                                                existing.type_str(),
                                                            ),
                                                        )
                                                    }),
                                                )
                                                .with_unexpected(key_span)
                                                .with_context(old_span),
                                        );
                                    return None;
                                }
                            }
                        }
                    };
                }
                Some(table)
            }
            fn get_key_span(key: &Spanned<DeString<'_>>) -> toml_parser::Span {
                let key_span = key.span();
                toml_parser::Span::new_unchecked(key_span.start, key_span.end)
            }
        }
        pub(crate) mod inline_table {
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeString;
            use crate::de::DeTable;
            use crate::de::DeValue;
            use crate::de::parser::array::on_array;
            use crate::de::parser::key::on_key;
            use crate::de::parser::prelude::*;
            use crate::de::parser::value::on_scalar;
            use crate::map::Entry;
            /// ```abnf
            /// ;; Inline Table
            ///
            /// inline-table = inline-table-open [ inline-table-keyvals ] ws-comment-newline inline-table-close
            /// ```
            pub(crate) fn on_inline_table<'i>(
                open_event: &toml_parser::parser::Event,
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> Spanned<DeValue<'i>> {
                let mut result = DeTable::new();
                result.set_inline(true);
                let mut close_span = open_event.span();
                let mut state = State::default();
                while let Some(event) = input.next_token() {
                    close_span = event.span();
                    match event.kind() {
                        EventKind::StdTableOpen
                        | EventKind::ArrayTableOpen
                        | EventKind::StdTableClose
                        | EventKind::ArrayClose
                        | EventKind::ArrayTableClose
                        | EventKind::KeySep => {
                            break;
                        }
                        EventKind::Error => {
                            continue;
                        }
                        EventKind::SimpleKey => {
                            let (path, key) = on_key(event, input, source, errors);
                            state.capture_key(event, path, key);
                        }
                        EventKind::KeyValSep => {
                            state.finish_key(event);
                        }
                        EventKind::InlineTableOpen => {
                            let value = on_inline_table(event, input, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::ArrayOpen => {
                            let value = on_array(event, input, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::Scalar => {
                            let value = on_scalar(event, source, errors);
                            state.capture_value(event, value);
                        }
                        EventKind::ValueSep => {
                            state.finish_value(event, &mut result, errors);
                        }
                        EventKind::Whitespace
                        | EventKind::Comment
                        | EventKind::Newline => {
                            state.whitespace(event);
                        }
                        EventKind::InlineTableClose => {
                            state.finish_value(event, &mut result, errors);
                            state.close(open_event, event, &mut result);
                            break;
                        }
                    }
                }
                let span = open_event.span().start()..close_span.end();
                Spanned::new(span, DeValue::Table(result))
            }
            struct State<'i> {
                current_key: Option<(Vec<Spanned<DeString<'i>>>, Spanned<DeString<'i>>)>,
                seen_keyval_sep: bool,
                current_value: Option<Spanned<DeValue<'i>>>,
            }
            #[automatically_derived]
            impl<'i> ::core::default::Default for State<'i> {
                #[inline]
                fn default() -> State<'i> {
                    State {
                        current_key: ::core::default::Default::default(),
                        seen_keyval_sep: ::core::default::Default::default(),
                        current_value: ::core::default::Default::default(),
                    }
                }
            }
            impl<'i> State<'i> {
                fn whitespace(&mut self, _event: &toml_parser::parser::Event) {}
                fn capture_key(
                    &mut self,
                    _event: &toml_parser::parser::Event,
                    path: Vec<Spanned<DeString<'i>>>,
                    key: Option<Spanned<DeString<'i>>>,
                ) {
                    if let Some(key) = key {
                        self.current_key = Some((path, key));
                    }
                }
                fn finish_key(&mut self, _event: &toml_parser::parser::Event) {
                    self.seen_keyval_sep = true;
                }
                fn capture_value(
                    &mut self,
                    _event: &toml_parser::parser::Event,
                    value: Spanned<DeValue<'i>>,
                ) {
                    self.current_value = Some(value);
                }
                fn finish_value(
                    &mut self,
                    _event: &toml_parser::parser::Event,
                    result: &mut DeTable<'i>,
                    errors: &mut dyn ErrorSink,
                ) {
                    self.seen_keyval_sep = false;
                    if let (Some((path, key)), Some(value)) = (
                        self.current_key.take(),
                        self.current_value.take(),
                    ) {
                        let Some(table) = descend_path(result, &path, true, errors) else {
                            return;
                        };
                        let mixed_table_types = table.is_dotted() == path.is_empty();
                        if mixed_table_types {
                            let key_span = get_key_span(&key);
                            errors
                                .report_error(
                                    ParseError::new("duplicate key").with_unexpected(key_span),
                                );
                        } else {
                            let key_span = get_key_span(&key);
                            match table.entry(key) {
                                Entry::Vacant(o) => {
                                    o.insert(value);
                                }
                                Entry::Occupied(o) => {
                                    let old_span = get_key_span(o.key());
                                    errors
                                        .report_error(
                                            ParseError::new("duplicate key")
                                                .with_unexpected(key_span)
                                                .with_context(old_span),
                                        );
                                }
                            }
                        }
                    }
                }
                fn close(
                    &mut self,
                    _open_event: &toml_parser::parser::Event,
                    _close_event: &toml_parser::parser::Event,
                    _result: &mut DeTable<'i>,
                ) {}
            }
            fn descend_path<'a, 'i>(
                mut table: &'a mut DeTable<'i>,
                path: &'a [Spanned<DeString<'i>>],
                dotted: bool,
                errors: &mut dyn ErrorSink,
            ) -> Option<&'a mut DeTable<'i>> {
                for key in path.iter() {
                    table = match table.entry(key.clone()) {
                        Entry::Vacant(entry) => {
                            let mut new_table = DeTable::new();
                            new_table.set_implicit(true);
                            new_table.set_dotted(dotted);
                            new_table.set_inline(true);
                            let value = DeValue::Table(new_table);
                            let value = Spanned::new(key.span(), value);
                            let value = entry.insert(value);
                            value.as_mut().as_table_mut().unwrap()
                        }
                        Entry::Occupied(entry) => {
                            let spanned = entry.into_mut();
                            let old_span = spanned.span();
                            match spanned.as_mut() {
                                DeValue::Table(sweet_child_of_mine) => {
                                    let mixed_table_types = dotted
                                        && !sweet_child_of_mine.is_implicit();
                                    if mixed_table_types {
                                        let key_span = get_key_span(key);
                                        errors
                                            .report_error(
                                                ParseError::new("duplicate key").with_unexpected(key_span),
                                            );
                                        return None;
                                    }
                                    sweet_child_of_mine
                                }
                                existing => {
                                    let old_span = toml_parser::Span::new_unchecked(
                                        old_span.start,
                                        old_span.end,
                                    );
                                    let key_span = get_key_span(key);
                                    errors
                                        .report_error(
                                            ParseError::new(
                                                    ::alloc::__export::must_use({
                                                        ::alloc::fmt::format(
                                                            format_args!(
                                                                "cannot extend value of type {0} with a dotted key",
                                                                existing.type_str(),
                                                            ),
                                                        )
                                                    }),
                                                )
                                                .with_unexpected(key_span)
                                                .with_context(old_span),
                                        );
                                    return None;
                                }
                            }
                        }
                    };
                }
                Some(table)
            }
            fn get_key_span(key: &Spanned<DeString<'_>>) -> toml_parser::Span {
                let key_span = key.span();
                toml_parser::Span::new_unchecked(key_span.start, key_span.end)
            }
        }
        pub(crate) mod key {
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeString;
            use crate::de::parser::prelude::*;
            /// ```abnf
            /// key = simple-key / dotted-key
            /// dotted-key = simple-key 1*( dot-sep simple-key )
            /// ```
            pub(crate) fn on_key<'i>(
                key_event: &toml_parser::parser::Event,
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> (Vec<Spanned<DeString<'i>>>, Option<Spanned<DeString<'i>>>) {
                let mut result_path = Vec::new();
                let mut result_key = None;
                let mut state = State::new(key_event);
                if more_key(input) {
                    while let Some(event) = input.next_token() {
                        match event.kind() {
                            EventKind::StdTableOpen
                            | EventKind::ArrayTableOpen
                            | EventKind::InlineTableOpen
                            | EventKind::InlineTableClose
                            | EventKind::ArrayOpen
                            | EventKind::ArrayClose
                            | EventKind::Scalar
                            | EventKind::ValueSep
                            | EventKind::Comment
                            | EventKind::Newline
                            | EventKind::KeyValSep
                            | EventKind::StdTableClose
                            | EventKind::ArrayTableClose
                            | EventKind::Error => {
                                continue;
                            }
                            EventKind::SimpleKey => {
                                state.current_key = Some(*event);
                                if !more_key(input) {
                                    break;
                                }
                            }
                            EventKind::Whitespace => {
                                state.whitespace(event);
                            }
                            EventKind::KeySep => {
                                state
                                    .close_key(
                                        &mut result_path,
                                        &mut result_key,
                                        source,
                                        errors,
                                    );
                            }
                        }
                    }
                }
                state.close_key(&mut result_path, &mut result_key, source, errors);
                if super::LIMIT <= result_path.len() as u32 {
                    errors.report_error(ParseError::new("recursion limit"));
                    return (Vec::new(), None);
                }
                (result_path, result_key)
            }
            fn more_key(input: &Input<'_>) -> bool {
                let first = input.get(0).map(|e| e.kind());
                let second = input.get(1).map(|e| e.kind());
                if first == Some(EventKind::KeySep) {
                    true
                } else if first == Some(EventKind::Whitespace)
                    && second == Some(EventKind::KeySep)
                {
                    true
                } else {
                    false
                }
            }
            struct State {
                current_key: Option<toml_parser::parser::Event>,
            }
            impl State {
                fn new(key_event: &toml_parser::parser::Event) -> Self {
                    Self {
                        current_key: Some(*key_event),
                    }
                }
                fn whitespace(&mut self, _event: &toml_parser::parser::Event) {}
                fn close_key<'i>(
                    &mut self,
                    result_path: &mut Vec<Spanned<DeString<'i>>>,
                    result_key: &mut Option<Spanned<DeString<'i>>>,
                    source: toml_parser::Source<'i>,
                    errors: &mut dyn ErrorSink,
                ) {
                    let Some(key) = self.current_key.take() else {
                        return;
                    };
                    let key_span = key.span();
                    let key_span = key_span.start()..key_span.end();
                    let raw = source.get(key).unwrap();
                    let mut decoded = alloc::borrow::Cow::Borrowed("");
                    raw.decode_key(&mut decoded, errors);
                    let key = Spanned::new(key_span, decoded);
                    if let Some(last_key) = result_key.replace(key) {
                        result_path.push(last_key);
                    }
                }
            }
        }
        pub(crate) mod value {
            use serde_spanned::Spanned;
            use crate::alloc_prelude::*;
            use crate::de::DeFloat;
            use crate::de::DeInteger;
            use crate::de::DeValue;
            use crate::de::parser::array::on_array;
            use crate::de::parser::inline_table::on_inline_table;
            use crate::de::parser::prelude::*;
            /// ```abnf
            /// val = string / boolean / array / inline-table / date-time / float / integer
            /// ```
            pub(crate) fn value<'i>(
                input: &mut Input<'_>,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> Spanned<DeValue<'i>> {
                if let Some(event) = input.next_token() {
                    match event.kind() {
                        EventKind::StdTableOpen
                        | EventKind::ArrayTableOpen
                        | EventKind::InlineTableClose
                        | EventKind::ArrayClose
                        | EventKind::ValueSep
                        | EventKind::Comment
                        | EventKind::Newline
                        | EventKind::Error
                        | EventKind::SimpleKey
                        | EventKind::KeySep
                        | EventKind::KeyValSep
                        | EventKind::StdTableClose
                        | EventKind::ArrayTableClose => {}
                        EventKind::Whitespace => {}
                        EventKind::InlineTableOpen => {
                            return on_inline_table(event, input, source, errors);
                        }
                        EventKind::ArrayOpen => {
                            return on_array(event, input, source, errors);
                        }
                        EventKind::Scalar => {
                            return on_scalar(event, source, errors);
                        }
                    }
                }
                Spanned::new(0..0, DeValue::Integer(Default::default()))
            }
            pub(crate) fn on_scalar<'i>(
                event: &toml_parser::parser::Event,
                source: toml_parser::Source<'i>,
                errors: &mut dyn ErrorSink,
            ) -> Spanned<DeValue<'i>> {
                let value_span = event.span();
                let value_span = value_span.start()..value_span.end();
                let raw = source.get(event).unwrap();
                let mut decoded = alloc::borrow::Cow::Borrowed("");
                let kind = raw.decode_scalar(&mut decoded, errors);
                match kind {
                    toml_parser::decoder::ScalarKind::String => {
                        Spanned::new(value_span, DeValue::String(decoded))
                    }
                    toml_parser::decoder::ScalarKind::Boolean(value) => {
                        Spanned::new(value_span, DeValue::Boolean(value))
                    }
                    toml_parser::decoder::ScalarKind::DateTime => {
                        let value = match decoded.parse::<toml_datetime::Datetime>() {
                            Ok(value) => value,
                            Err(err) => {
                                errors
                                    .report_error(
                                        ParseError::new(err.to_string())
                                            .with_unexpected(event.span()),
                                    );
                                toml_datetime::Datetime {
                                    date: None,
                                    time: None,
                                    offset: None,
                                }
                            }
                        };
                        Spanned::new(value_span, DeValue::Datetime(value))
                    }
                    toml_parser::decoder::ScalarKind::Float => {
                        Spanned::new(
                            value_span,
                            DeValue::Float(DeFloat { inner: decoded }),
                        )
                    }
                    toml_parser::decoder::ScalarKind::Integer(radix) => {
                        Spanned::new(
                            value_span,
                            DeValue::Integer(DeInteger {
                                inner: decoded,
                                radix: radix.value(),
                            }),
                        )
                    }
                }
            }
        }
        pub(crate) fn parse_document<'i>(
            source: toml_parser::Source<'i>,
            errors: &mut dyn prelude::ErrorSink,
        ) -> Spanned<DeTable<'i>> {
            let tokens = source.lex().into_vec();
            let mut events = Vec::with_capacity(tokens.len());
            let mut receiver = ValidateWhitespace::new(&mut events, source);
            let mut receiver = RecursionGuard::new(&mut receiver, LIMIT);
            let receiver = &mut receiver;
            toml_parser::parser::parse_document(&tokens, receiver, errors);
            let mut input = prelude::Input::new(&events);
            let doc = document::document(&mut input, source, errors);
            doc
        }
        pub(crate) fn parse_value<'i>(
            source: toml_parser::Source<'i>,
            errors: &mut dyn prelude::ErrorSink,
        ) -> Spanned<DeValue<'i>> {
            let tokens = source.lex().into_vec();
            let mut events = Vec::with_capacity(tokens.len());
            let mut receiver = ValidateWhitespace::new(&mut events, source);
            let mut receiver = RecursionGuard::new(&mut receiver, LIMIT);
            let receiver = &mut receiver;
            toml_parser::parser::parse_value(&tokens, receiver, errors);
            let mut input = prelude::Input::new(&events);
            let value = value::value(&mut input, source, errors);
            value
        }
        const LIMIT: u32 = 80;
        pub(crate) mod prelude {
            pub(crate) use toml_parser::ErrorSink;
            pub(crate) use toml_parser::ParseError;
            pub(crate) use toml_parser::parser::EventKind;
            pub(crate) use winnow::stream::Stream as _;
            pub(crate) type Input<'i> = winnow::stream::TokenSlice<
                'i,
                toml_parser::parser::Event,
            >;
        }
    }
    pub use deserializer::Deserializer;
    pub use deserializer::ValueDeserializer;
    pub use parser::DeArray;
    pub use parser::DeFloat;
    pub use parser::DeInteger;
    pub use parser::DeString;
    pub use parser::DeTable;
    pub use parser::DeValue;
    pub use error::Error;
    use crate::alloc_prelude::*;
    /// Deserializes a string into a type.
    ///
    /// This function will attempt to interpret `s` as a TOML document and
    /// deserialize `T` from the document.
    ///
    /// To deserializes TOML values, instead of documents, see [`ValueDeserializer`].
    ///
    /// # Examples
    ///
    /// ```
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Config {
    ///     title: String,
    ///     owner: Owner,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Owner {
    ///     name: String,
    /// }
    ///
    /// let config: Config = toml::from_str(r#"
    ///     title = 'TOML Example'
    ///
    ///     [owner]
    ///     name = 'Lisa'
    /// "#).unwrap();
    ///
    /// assert_eq!(config.title, "TOML Example");
    /// assert_eq!(config.owner.name, "Lisa");
    /// ```
    pub fn from_str<'de, T>(s: &'de str) -> Result<T, Error>
    where
        T: serde_core::de::Deserialize<'de>,
    {
        T::deserialize(Deserializer::parse(s)?)
    }
    /// Deserializes bytes into a type.
    ///
    /// This function will attempt to interpret `s` as a TOML document and
    /// deserialize `T` from the document.
    ///
    /// To deserializes TOML values, instead of documents, see [`ValueDeserializer`].
    pub fn from_slice<'de, T>(s: &'de [u8]) -> Result<T, Error>
    where
        T: serde_core::de::Deserialize<'de>,
    {
        let s = core::str::from_utf8(s).map_err(|e| Error::custom(e.to_string(), None))?;
        from_str(s)
    }
}
pub mod ser {
    //! Serializing Rust structures into TOML.
    //!
    //! This module contains all the Serde support for serializing Rust structures
    //! into TOML documents (as strings). Note that some top-level functions here
    //! are also provided at the top of the crate.
    mod document {
        //! Serializing Rust structures into TOML.
        //!
        //! This module contains all the Serde support for serializing Rust structures
        //! into TOML documents (as strings). Note that some top-level functions here
        //! are also provided at the top of the crate.
        mod array {
            use core::fmt::Write as _;
            use toml_writer::TomlWrite as _;
            use super::Buffer;
            use super::Error;
            use super::Table;
            use super::style::Style;
            use super::value::ValueSerializer;
            #[doc(hidden)]
            pub struct SerializeDocumentTupleVariant<'d> {
                buf: &'d mut Buffer,
                table: Table,
                seen_value: bool,
                style: Style,
            }
            impl<'d> SerializeDocumentTupleVariant<'d> {
                pub(crate) fn tuple(
                    buf: &'d mut Buffer,
                    mut table: Table,
                    variant: &'static str,
                    _len: usize,
                    style: Style,
                ) -> Result<Self, Error> {
                    let dst = table.body_mut();
                    dst.key(variant)?;
                    dst.space()?;
                    dst.keyval_sep()?;
                    dst.space()?;
                    dst.open_array()?;
                    Ok(Self {
                        buf,
                        table,
                        seen_value: false,
                        style,
                    })
                }
            }
            impl<'d> serde_core::ser::SerializeTupleVariant
            for SerializeDocumentTupleVariant<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let dst = self.table.body_mut();
                    if self.style.multiline_array {
                        dst.newline()?;
                        dst.write_fmt(format_args!("    "))?;
                    } else {
                        if self.seen_value {
                            dst.val_sep()?;
                            dst.space()?;
                        }
                    }
                    self.seen_value = true;
                    value.serialize(ValueSerializer::with_style(dst, self.style))?;
                    if self.style.multiline_array {
                        dst.val_sep()?;
                    }
                    Ok(())
                }
                fn end(mut self) -> Result<Self::Ok, Self::Error> {
                    let dst = self.table.body_mut();
                    if self.style.multiline_array && self.seen_value {
                        dst.newline()?;
                    }
                    dst.close_array()?;
                    dst.newline()?;
                    self.buf.push(self.table);
                    Ok(self.buf)
                }
            }
        }
        mod array_of_tables {
            use super::Buffer;
            use super::Error;
            use super::Serializer;
            use super::Table;
            use super::style::Style;
            use crate::alloc_prelude::*;
            pub(crate) struct ArrayOfTablesSerializer<'d> {
                buf: &'d mut Buffer,
                parent: Table,
                key: String,
                style: Style,
            }
            impl<'d> ArrayOfTablesSerializer<'d> {
                /// Creates a new serializer which will emit TOML into the buffer provided.
                ///
                /// The serializer can then be used to serialize a type after which the data
                /// will be present in `dst`.
                pub(crate) fn new(
                    buf: &'d mut Buffer,
                    parent: Table,
                    key: String,
                    style: Style,
                ) -> Self {
                    Self { buf, parent, key, style }
                }
            }
            impl<'d> serde_core::ser::Serializer for ArrayOfTablesSerializer<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                type SerializeSeq = SerializeArrayOfTablesSerializer<'d>;
                type SerializeTuple = SerializeArrayOfTablesSerializer<'d>;
                type SerializeTupleStruct = SerializeArrayOfTablesSerializer<'d>;
                type SerializeTupleVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Self::Error,
                >;
                type SerializeMap = serde_core::ser::Impossible<Self::Ok, Self::Error>;
                type SerializeStruct = serde_core::ser::Impossible<
                    Self::Ok,
                    Self::Error,
                >;
                type SerializeStructVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Self::Error,
                >;
                fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("bool")))
                }
                fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("i8")))
                }
                fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("i16")))
                }
                fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("i32")))
                }
                fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("i64")))
                }
                fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("u8")))
                }
                fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("u16")))
                }
                fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("u32")))
                }
                fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("u64")))
                }
                fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("f32")))
                }
                fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("f64")))
                }
                fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("char")))
                }
                fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("str")))
                }
                fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("bytes")))
                }
                fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_none())
                }
                fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    v.serialize(self)
                }
                fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some("unit")))
                }
                fn serialize_unit_struct(
                    self,
                    name: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some(name)))
                }
                fn serialize_unit_variant(
                    self,
                    name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    Err(Error::unsupported_type(Some(name)))
                }
                fn serialize_newtype_struct<T>(
                    self,
                    _name: &'static str,
                    v: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    v.serialize(self)
                }
                fn serialize_newtype_variant<T>(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    variant: &'static str,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Err(Error::unsupported_type(Some(variant)))
                }
                fn serialize_seq(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeSeq, Self::Error> {
                    Ok(
                        SerializeArrayOfTablesSerializer::seq(
                            self.buf,
                            self.parent,
                            self.key,
                            self.style,
                        ),
                    )
                }
                fn serialize_tuple(
                    self,
                    len: usize,
                ) -> Result<Self::SerializeTuple, Self::Error> {
                    self.serialize_seq(Some(len))
                }
                fn serialize_tuple_struct(
                    self,
                    _name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                    self.serialize_seq(Some(len))
                }
                fn serialize_tuple_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                    Err(Error::unsupported_type(Some(variant)))
                }
                fn serialize_map(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeMap, Self::Error> {
                    Err(Error::unsupported_type(Some("map")))
                }
                fn serialize_struct(
                    self,
                    name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStruct, Self::Error> {
                    Err(Error::unsupported_type(Some(name)))
                }
                fn serialize_struct_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStructVariant, Self::Error> {
                    Err(Error::unsupported_type(Some(variant)))
                }
            }
            #[doc(hidden)]
            pub(crate) struct SerializeArrayOfTablesSerializer<'d> {
                buf: &'d mut Buffer,
                parent: Table,
                key: String,
                style: Style,
            }
            impl<'d> SerializeArrayOfTablesSerializer<'d> {
                pub(crate) fn seq(
                    buf: &'d mut Buffer,
                    parent: Table,
                    key: String,
                    style: Style,
                ) -> Self {
                    Self { buf, parent, key, style }
                }
                fn end(self) -> Result<&'d mut Buffer, Error> {
                    Ok(self.buf)
                }
            }
            impl<'d> serde_core::ser::SerializeSeq
            for SerializeArrayOfTablesSerializer<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let child = self
                        .buf
                        .element_table(&mut self.parent, self.key.clone());
                    let value_serializer = Serializer::with_table(
                        self.buf,
                        child,
                        self.style,
                    );
                    value.serialize(value_serializer)?;
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl<'d> serde_core::ser::SerializeTuple
            for SerializeArrayOfTablesSerializer<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeSeq::serialize_element(self, value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    serde_core::ser::SerializeSeq::end(self)
                }
            }
            impl<'d> serde_core::ser::SerializeTupleStruct
            for SerializeArrayOfTablesSerializer<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeSeq::serialize_element(self, value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    serde_core::ser::SerializeSeq::end(self)
                }
            }
        }
        mod buffer {
            use toml_writer::TomlWrite as _;
            use crate::alloc_prelude::*;
            /// TOML Document serialization buffer
            pub struct Buffer {
                tables: Vec<Option<Table>>,
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Buffer {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Buffer",
                        "tables",
                        &&self.tables,
                    )
                }
            }
            #[automatically_derived]
            impl ::core::default::Default for Buffer {
                #[inline]
                fn default() -> Buffer {
                    Buffer {
                        tables: ::core::default::Default::default(),
                    }
                }
            }
            impl Buffer {
                /// Initialize a new serialization buffer
                pub fn new() -> Self {
                    Default::default()
                }
                /// Reset the buffer for serializing another document
                pub fn clear(&mut self) {
                    self.tables.clear();
                }
                pub(crate) fn root_table(&mut self) -> Table {
                    self.new_table(None)
                }
                pub(crate) fn child_table(
                    &mut self,
                    parent: &mut Table,
                    key: String,
                ) -> Table {
                    parent.has_children = true;
                    let mut key_path = parent.key.clone();
                    key_path.get_or_insert_with(Vec::new).push(key);
                    self.new_table(key_path)
                }
                pub(crate) fn element_table(
                    &mut self,
                    parent: &mut Table,
                    key: String,
                ) -> Table {
                    let mut table = self.child_table(parent, key);
                    table.array = true;
                    table
                }
                pub(crate) fn new_table(&mut self, key: Option<Vec<String>>) -> Table {
                    let pos = self.tables.len();
                    let table = Table {
                        key,
                        body: String::new(),
                        has_children: false,
                        pos,
                        array: false,
                    };
                    self.tables.push(None);
                    table
                }
                pub(crate) fn push(&mut self, table: Table) {
                    let pos = table.pos;
                    self.tables[pos] = Some(table);
                }
            }
            impl core::fmt::Display for Buffer {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    let mut tables = self
                        .tables
                        .iter()
                        .filter_map(|t| t.as_ref())
                        .filter(|t| required_table(t));
                    if let Some(table) = tables.next() {
                        table.fmt(f)?;
                    }
                    for table in tables {
                        f.newline()?;
                        table.fmt(f)?;
                    }
                    Ok(())
                }
            }
            fn required_table(table: &Table) -> bool {
                if table.key.is_none() {
                    !table.body.is_empty()
                } else {
                    table.array || !table.body.is_empty() || !table.has_children
                }
            }
            pub(crate) struct Table {
                key: Option<Vec<String>>,
                body: String,
                has_children: bool,
                array: bool,
                pos: usize,
            }
            #[automatically_derived]
            impl ::core::clone::Clone for Table {
                #[inline]
                fn clone(&self) -> Table {
                    Table {
                        key: ::core::clone::Clone::clone(&self.key),
                        body: ::core::clone::Clone::clone(&self.body),
                        has_children: ::core::clone::Clone::clone(&self.has_children),
                        array: ::core::clone::Clone::clone(&self.array),
                        pos: ::core::clone::Clone::clone(&self.pos),
                    }
                }
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for Table {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_struct_field5_finish(
                        f,
                        "Table",
                        "key",
                        &self.key,
                        "body",
                        &self.body,
                        "has_children",
                        &self.has_children,
                        "array",
                        &self.array,
                        "pos",
                        &&self.pos,
                    )
                }
            }
            impl Table {
                pub(crate) fn body_mut(&mut self) -> &mut String {
                    &mut self.body
                }
                pub(crate) fn has_children(&mut self, yes: bool) {
                    self.has_children = yes;
                }
            }
            impl core::fmt::Display for Table {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    if let Some(key) = &self.key {
                        if self.array {
                            f.open_array_of_tables_header()?;
                        } else {
                            f.open_table_header()?;
                        }
                        let mut key = key.iter();
                        if let Some(key) = key.next() {
                            f.write_fmt(format_args!("{0}", key))?;
                        }
                        for key in key {
                            f.key_sep()?;
                            f.write_fmt(format_args!("{0}", key))?;
                        }
                        if self.array {
                            f.close_array_of_tables_header()?;
                        } else {
                            f.close_table_header()?;
                        }
                        f.newline()?;
                    }
                    self.body.fmt(f)?;
                    Ok(())
                }
            }
        }
        mod map {
            use core::fmt::Write as _;
            use toml_writer::TomlWrite as _;
            use super::Buffer;
            use super::Error;
            use super::SerializationStrategy;
            use super::Serializer;
            use super::Table;
            use super::array_of_tables::ArrayOfTablesSerializer;
            use super::style::Style;
            use super::value::KeySerializer;
            use super::value::ValueSerializer;
            use crate::alloc_prelude::*;
            #[doc(hidden)]
            pub struct SerializeDocumentTable<'d> {
                buf: &'d mut Buffer,
                table: Table,
                key: Option<String>,
                style: Style,
            }
            impl<'d> SerializeDocumentTable<'d> {
                pub(crate) fn map(
                    buf: &'d mut Buffer,
                    table: Table,
                    style: Style,
                ) -> Result<Self, Error> {
                    Ok(Self {
                        buf,
                        table,
                        key: None,
                        style,
                    })
                }
                fn end(self) -> Result<&'d mut Buffer, Error> {
                    self.buf.push(self.table);
                    Ok(self.buf)
                }
            }
            impl<'d> serde_core::ser::SerializeMap for SerializeDocumentTable<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_key<T>(&mut self, input: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let mut encoded_key = String::new();
                    input
                        .serialize(KeySerializer {
                            dst: &mut encoded_key,
                        })?;
                    self.key = Some(encoded_key);
                    Ok(())
                }
                fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let encoded_key = self
                        .key
                        .take()
                        .expect("always called after `serialize_key`");
                    match SerializationStrategy::from(value) {
                        SerializationStrategy::Value => {
                            let dst = self.table.body_mut();
                            dst.write_fmt(format_args!("{0}", encoded_key))?;
                            dst.space()?;
                            dst.keyval_sep()?;
                            dst.space()?;
                            let value_serializer = ValueSerializer::with_style(
                                dst,
                                self.style,
                            );
                            let dst = value.serialize(value_serializer)?;
                            dst.newline()?;
                        }
                        SerializationStrategy::ArrayOfTables => {
                            self.table.has_children(true);
                            let value_serializer = ArrayOfTablesSerializer::new(
                                self.buf,
                                self.table.clone(),
                                encoded_key,
                                self.style,
                            );
                            value.serialize(value_serializer)?;
                        }
                        SerializationStrategy::Table
                        | SerializationStrategy::Unknown => {
                            let child = self
                                .buf
                                .child_table(&mut self.table, encoded_key);
                            let value_serializer = Serializer::with_table(
                                self.buf,
                                child,
                                self.style,
                            );
                            value.serialize(value_serializer)?;
                        }
                        SerializationStrategy::Skip => {}
                    }
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl<'d> serde_core::ser::SerializeStruct for SerializeDocumentTable<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    match SerializationStrategy::from(value) {
                        SerializationStrategy::Value => {
                            let dst = self.table.body_mut();
                            dst.key(key)?;
                            dst.space()?;
                            dst.keyval_sep()?;
                            dst.space()?;
                            let value_serializer = ValueSerializer::with_style(
                                dst,
                                self.style,
                            );
                            let dst = value.serialize(value_serializer)?;
                            dst.newline()?;
                        }
                        SerializationStrategy::ArrayOfTables => {
                            self.table.has_children(true);
                            let value_serializer = ArrayOfTablesSerializer::new(
                                self.buf,
                                self.table.clone(),
                                key.to_owned(),
                                self.style,
                            );
                            value.serialize(value_serializer)?;
                        }
                        SerializationStrategy::Table
                        | SerializationStrategy::Unknown => {
                            let child = self
                                .buf
                                .child_table(&mut self.table, key.to_owned());
                            let value_serializer = Serializer::with_table(
                                self.buf,
                                child,
                                self.style,
                            );
                            value.serialize(value_serializer)?;
                        }
                        SerializationStrategy::Skip => {}
                    }
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl<'d> serde_core::ser::SerializeStructVariant
            for SerializeDocumentTable<'d> {
                type Ok = &'d mut Buffer;
                type Error = Error;
                #[inline]
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeStruct::serialize_field(self, key, value)
                }
                #[inline]
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
        }
        mod strategy {
            pub(crate) enum SerializationStrategy {
                Value,
                Table,
                ArrayOfTables,
                Skip,
                Unknown,
            }
            #[automatically_derived]
            impl ::core::marker::Copy for SerializationStrategy {}
            #[automatically_derived]
            #[doc(hidden)]
            unsafe impl ::core::clone::TrivialClone for SerializationStrategy {}
            #[automatically_derived]
            impl ::core::clone::Clone for SerializationStrategy {
                #[inline]
                fn clone(&self) -> SerializationStrategy {
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for SerializationStrategy {}
            #[automatically_derived]
            impl ::core::cmp::PartialEq for SerializationStrategy {
                #[inline]
                fn eq(&self, other: &SerializationStrategy) -> bool {
                    let __self_discr = ::core::intrinsics::discriminant_value(self);
                    let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                    __self_discr == __arg1_discr
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Eq for SerializationStrategy {
                #[inline]
                #[doc(hidden)]
                #[coverage(off)]
                fn assert_receiver_is_total_eq(&self) {}
            }
            #[automatically_derived]
            impl ::core::fmt::Debug for SerializationStrategy {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(
                        f,
                        match self {
                            SerializationStrategy::Value => "Value",
                            SerializationStrategy::Table => "Table",
                            SerializationStrategy::ArrayOfTables => "ArrayOfTables",
                            SerializationStrategy::Skip => "Skip",
                            SerializationStrategy::Unknown => "Unknown",
                        },
                    )
                }
            }
            impl<T> From<&T> for SerializationStrategy
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                fn from(value: &T) -> Self {
                    value.serialize(WalkValue).unwrap_err()
                }
            }
            impl serde_core::ser::Error for SerializationStrategy {
                fn custom<T>(_msg: T) -> Self
                where
                    T: core::fmt::Display,
                {
                    Self::Unknown
                }
            }
            impl core::fmt::Display for SerializationStrategy {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    "error".fmt(f)
                }
            }
            impl core::error::Error for SerializationStrategy {}
            struct WalkValue;
            impl serde_core::ser::Serializer for WalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                type SerializeSeq = ArrayWalkValue;
                type SerializeTuple = ArrayWalkValue;
                type SerializeTupleStruct = ArrayWalkValue;
                type SerializeTupleVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Self::Error,
                >;
                type SerializeMap = serde_core::ser::Impossible<Self::Ok, Self::Error>;
                type SerializeStruct = StructWalkValue;
                type SerializeStructVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Self::Error,
                >;
                fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Skip)
                }
                fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    v.serialize(self)
                }
                fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_unit_struct(
                    self,
                    _name: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_unit_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
                fn serialize_newtype_struct<T>(
                    self,
                    _name: &'static str,
                    v: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    v.serialize(self)
                }
                fn serialize_newtype_variant<T>(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Err(SerializationStrategy::Table)
                }
                fn serialize_seq(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeSeq, Self::Error> {
                    Ok(ArrayWalkValue::new())
                }
                fn serialize_tuple(
                    self,
                    len: usize,
                ) -> Result<Self::SerializeTuple, Self::Error> {
                    self.serialize_seq(Some(len))
                }
                fn serialize_tuple_struct(
                    self,
                    _name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                    self.serialize_seq(Some(len))
                }
                fn serialize_tuple_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                    Err(SerializationStrategy::Table)
                }
                fn serialize_map(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeMap, Self::Error> {
                    Err(SerializationStrategy::Table)
                }
                fn serialize_struct(
                    self,
                    name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStruct, Self::Error> {
                    if toml_datetime::ser::is_datetime(name) {
                        Ok(StructWalkValue)
                    } else {
                        Err(SerializationStrategy::Table)
                    }
                }
                fn serialize_struct_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStructVariant, Self::Error> {
                    Err(SerializationStrategy::Table)
                }
            }
            #[doc(hidden)]
            pub(crate) struct ArrayWalkValue {
                is_empty: bool,
            }
            impl ArrayWalkValue {
                fn new() -> Self {
                    Self { is_empty: true }
                }
                fn serialize_element<T>(
                    &mut self,
                    value: &T,
                ) -> Result<(), SerializationStrategy>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    self.is_empty = false;
                    match SerializationStrategy::from(value) {
                        SerializationStrategy::Value
                        | SerializationStrategy::ArrayOfTables
                        | SerializationStrategy::Unknown
                        | SerializationStrategy::Skip => {
                            Err(SerializationStrategy::Value)
                        }
                        SerializationStrategy::Table => Ok(()),
                    }
                }
                fn end(
                    self,
                ) -> Result<core::convert::Infallible, SerializationStrategy> {
                    if self.is_empty {
                        Err(SerializationStrategy::Value)
                    } else {
                        Err(SerializationStrategy::ArrayOfTables)
                    }
                }
            }
            impl serde_core::ser::SerializeSeq for ArrayWalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    self.serialize_element(value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl serde_core::ser::SerializeTuple for ArrayWalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    self.serialize_element(value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl serde_core::ser::SerializeTupleStruct for ArrayWalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    self.serialize_element(value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            pub(crate) struct StructWalkValue;
            impl serde_core::ser::SerializeMap for StructWalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                fn serialize_key<T>(&mut self, _input: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Ok(())
                }
                fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
            }
            impl serde_core::ser::SerializeStruct for StructWalkValue {
                type Ok = core::convert::Infallible;
                type Error = SerializationStrategy;
                fn serialize_field<T>(
                    &mut self,
                    _key: &'static str,
                    _value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    Err(SerializationStrategy::Value)
                }
            }
        }
        use toml_writer::TomlWrite as _;
        use super::Error;
        use super::style;
        use super::value;
        use crate::alloc_prelude::*;
        use buffer::Table;
        use strategy::SerializationStrategy;
        pub use buffer::Buffer;
        /// Serialization for TOML documents.
        ///
        /// This structure implements serialization support for TOML to serialize an
        /// arbitrary type to TOML. Note that the TOML format does not support all
        /// datatypes in Rust, such as enums, tuples, and tuple structs. These types
        /// will generate an error when serialized.
        ///
        /// Currently a serializer always writes its output to an in-memory `String`,
        /// which is passed in when creating the serializer itself.
        ///
        /// To serialize TOML values, instead of documents, see
        /// [`ValueSerializer`][super::value::ValueSerializer].
        pub struct Serializer<'d> {
            buf: &'d mut Buffer,
            style: style::Style,
            table: Table,
        }
        impl<'d> Serializer<'d> {
            /// Creates a new serializer which will emit TOML into the buffer provided.
            ///
            /// The serializer can then be used to serialize a type after which the data
            /// will be present in `buf`.
            pub fn new(buf: &'d mut Buffer) -> Self {
                let table = buf.root_table();
                Self {
                    buf,
                    style: Default::default(),
                    table,
                }
            }
            /// Apply a default "pretty" policy to the document
            ///
            /// For greater customization, instead serialize to a
            /// [`toml_edit::DocumentMut`](https://docs.rs/toml_edit/latest/toml_edit/struct.DocumentMut.html).
            pub fn pretty(buf: &'d mut Buffer) -> Self {
                let mut ser = Serializer::new(buf);
                ser.style.multiline_array = true;
                ser
            }
            pub(crate) fn with_table(
                buf: &'d mut Buffer,
                table: Table,
                style: style::Style,
            ) -> Self {
                Self { buf, style, table }
            }
            fn end(self) -> Result<&'d mut Buffer, Error> {
                self.buf.push(self.table);
                Ok(self.buf)
            }
        }
        impl<'d> serde_core::ser::Serializer for Serializer<'d> {
            type Ok = &'d mut Buffer;
            type Error = Error;
            type SerializeSeq = serde_core::ser::Impossible<Self::Ok, Self::Error>;
            type SerializeTuple = serde_core::ser::Impossible<Self::Ok, Self::Error>;
            type SerializeTupleStruct = serde_core::ser::Impossible<
                Self::Ok,
                Self::Error,
            >;
            type SerializeTupleVariant = array::SerializeDocumentTupleVariant<'d>;
            type SerializeMap = map::SerializeDocumentTable<'d>;
            type SerializeStruct = map::SerializeDocumentTable<'d>;
            type SerializeStructVariant = map::SerializeDocumentTable<'d>;
            fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("bool")))
            }
            fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("i8")))
            }
            fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("i16")))
            }
            fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("i32")))
            }
            fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("i64")))
            }
            fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("u8")))
            }
            fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("u16")))
            }
            fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("u32")))
            }
            fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("u64")))
            }
            fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("f32")))
            }
            fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("f64")))
            }
            fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("char")))
            }
            fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("str")))
            }
            fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("bytes")))
            }
            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_none())
            }
            fn serialize_some<T>(self, v: &T) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                v.serialize(self)
            }
            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("unit")))
            }
            fn serialize_unit_struct(
                self,
                name: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some(name)))
            }
            fn serialize_unit_variant(
                self,
                name: &'static str,
                _variant_index: u32,
                _variant: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some(name)))
            }
            fn serialize_newtype_struct<T>(
                self,
                _name: &'static str,
                v: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                v.serialize(self)
            }
            fn serialize_newtype_variant<T>(
                mut self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                match SerializationStrategy::from(value) {
                    SerializationStrategy::Value
                    | SerializationStrategy::ArrayOfTables => {
                        let dst = self.table.body_mut();
                        dst.key(variant)?;
                        dst.space()?;
                        dst.keyval_sep()?;
                        dst.space()?;
                        let value_serializer = value::ValueSerializer::with_style(
                            dst,
                            self.style,
                        );
                        let dst = value.serialize(value_serializer)?;
                        dst.newline()?;
                    }
                    SerializationStrategy::Table | SerializationStrategy::Unknown => {
                        let child = self
                            .buf
                            .child_table(&mut self.table, variant.to_owned());
                        let value_serializer = Serializer::with_table(
                            self.buf,
                            child,
                            self.style,
                        );
                        value.serialize(value_serializer)?;
                    }
                    SerializationStrategy::Skip => {}
                }
                self.end()
            }
            fn serialize_seq(
                self,
                _len: Option<usize>,
            ) -> Result<Self::SerializeSeq, Self::Error> {
                Err(Error::unsupported_type(Some("array")))
            }
            fn serialize_tuple(
                self,
                len: usize,
            ) -> Result<Self::SerializeTuple, Self::Error> {
                self.serialize_seq(Some(len))
            }
            fn serialize_tuple_struct(
                self,
                _name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                self.serialize_seq(Some(len))
            }
            fn serialize_tuple_variant(
                self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                array::SerializeDocumentTupleVariant::tuple(
                    self.buf,
                    self.table,
                    variant,
                    len,
                    self.style,
                )
            }
            fn serialize_map(
                self,
                _len: Option<usize>,
            ) -> Result<Self::SerializeMap, Self::Error> {
                map::SerializeDocumentTable::map(self.buf, self.table, self.style)
            }
            fn serialize_struct(
                self,
                _name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                self.serialize_map(Some(len))
            }
            fn serialize_struct_variant(
                mut self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                let child = self.buf.child_table(&mut self.table, variant.to_owned());
                self.buf.push(self.table);
                map::SerializeDocumentTable::map(self.buf, child, self.style)
            }
        }
    }
    mod error {
        use crate::alloc_prelude::*;
        /// Errors that can occur when serializing a type.
        pub struct Error {
            pub(crate) inner: ErrorInner,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Error {
            #[inline]
            fn clone(&self) -> Error {
                Error {
                    inner: ::core::clone::Clone::clone(&self.inner),
                }
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Error {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                self.inner == other.inner
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<ErrorInner>;
            }
        }
        impl Error {
            pub(crate) fn new(inner: impl core::fmt::Display) -> Self {
                Self {
                    inner: ErrorInner::Custom(inner.to_string()),
                }
            }
            pub(crate) fn unsupported_type(t: Option<&'static str>) -> Self {
                Self {
                    inner: ErrorInner::UnsupportedType(t),
                }
            }
            pub(crate) fn unsupported_none() -> Self {
                Self {
                    inner: ErrorInner::UnsupportedNone,
                }
            }
            pub(crate) fn key_not_string() -> Self {
                Self {
                    inner: ErrorInner::KeyNotString,
                }
            }
            pub(crate) fn date_invalid() -> Self {
                Self {
                    inner: ErrorInner::DateInvalid,
                }
            }
        }
        impl From<core::fmt::Error> for Error {
            fn from(_: core::fmt::Error) -> Self {
                Self::new("an error occurred when writing a value")
            }
        }
        impl serde_core::ser::Error for Error {
            fn custom<T>(msg: T) -> Self
            where
                T: core::fmt::Display,
            {
                Self::new(msg)
            }
        }
        impl core::fmt::Display for Error {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }
        impl core::fmt::Debug for Error {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.inner.fmt(f)
            }
        }
        impl core::error::Error for Error {}
        /// Errors that can occur when deserializing a type.
        #[non_exhaustive]
        pub(crate) enum ErrorInner {
            /// Type could not be serialized to TOML
            UnsupportedType(Option<&'static str>),
            /// `None` could not be serialized to TOML
            UnsupportedNone,
            /// Key was not convertible to `String` for serializing to TOML
            KeyNotString,
            /// A serialized date was invalid
            DateInvalid,
            /// Other serialization error
            Custom(String),
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for ErrorInner {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    ErrorInner::UnsupportedType(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "UnsupportedType",
                            &__self_0,
                        )
                    }
                    ErrorInner::UnsupportedNone => {
                        ::core::fmt::Formatter::write_str(f, "UnsupportedNone")
                    }
                    ErrorInner::KeyNotString => {
                        ::core::fmt::Formatter::write_str(f, "KeyNotString")
                    }
                    ErrorInner::DateInvalid => {
                        ::core::fmt::Formatter::write_str(f, "DateInvalid")
                    }
                    ErrorInner::Custom(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Custom",
                            &__self_0,
                        )
                    }
                }
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for ErrorInner {
            #[inline]
            fn clone(&self) -> ErrorInner {
                match self {
                    ErrorInner::UnsupportedType(__self_0) => {
                        ErrorInner::UnsupportedType(
                            ::core::clone::Clone::clone(__self_0),
                        )
                    }
                    ErrorInner::UnsupportedNone => ErrorInner::UnsupportedNone,
                    ErrorInner::KeyNotString => ErrorInner::KeyNotString,
                    ErrorInner::DateInvalid => ErrorInner::DateInvalid,
                    ErrorInner::Custom(__self_0) => {
                        ErrorInner::Custom(::core::clone::Clone::clone(__self_0))
                    }
                }
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for ErrorInner {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for ErrorInner {
            #[inline]
            fn eq(&self, other: &ErrorInner) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
                    && match (self, other) {
                        (
                            ErrorInner::UnsupportedType(__self_0),
                            ErrorInner::UnsupportedType(__arg1_0),
                        ) => __self_0 == __arg1_0,
                        (ErrorInner::Custom(__self_0), ErrorInner::Custom(__arg1_0)) => {
                            __self_0 == __arg1_0
                        }
                        _ => true,
                    }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for ErrorInner {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<Option<&'static str>>;
                let _: ::core::cmp::AssertParamIsEq<String>;
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for ErrorInner {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_discr, state);
                match self {
                    ErrorInner::UnsupportedType(__self_0) => {
                        ::core::hash::Hash::hash(__self_0, state)
                    }
                    ErrorInner::Custom(__self_0) => {
                        ::core::hash::Hash::hash(__self_0, state)
                    }
                    _ => {}
                }
            }
        }
        impl core::fmt::Display for ErrorInner {
            fn fmt(
                &self,
                formatter: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result {
                match self {
                    Self::UnsupportedType(Some(t)) => {
                        formatter.write_fmt(format_args!("unsupported {0} type", t))
                    }
                    Self::UnsupportedType(None) => {
                        formatter.write_fmt(format_args!("unsupported rust type"))
                    }
                    Self::UnsupportedNone => "unsupported None value".fmt(formatter),
                    Self::KeyNotString => "map key was not a string".fmt(formatter),
                    Self::DateInvalid => "a serialized date was invalid".fmt(formatter),
                    Self::Custom(s) => s.fmt(formatter),
                }
            }
        }
    }
    mod style {
        pub(crate) struct Style {
            pub(crate) multiline_array: bool,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Style {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Style {}
        #[automatically_derived]
        impl ::core::clone::Clone for Style {
            #[inline]
            fn clone(&self) -> Style {
                let _: ::core::clone::AssertParamIsClone<bool>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::default::Default for Style {
            #[inline]
            fn default() -> Style {
                Style {
                    multiline_array: ::core::default::Default::default(),
                }
            }
        }
    }
    mod value {
        mod array {
            use core::fmt::Write as _;
            use toml_writer::TomlWrite as _;
            use super::Error;
            use super::Style;
            use crate::alloc_prelude::*;
            #[doc(hidden)]
            pub struct SerializeValueArray<'d> {
                dst: &'d mut String,
                seen_value: bool,
                style: Style,
                len: Option<usize>,
            }
            impl<'d> SerializeValueArray<'d> {
                pub(crate) fn seq(
                    dst: &'d mut String,
                    style: Style,
                    len: Option<usize>,
                ) -> Result<Self, Error> {
                    dst.open_array()?;
                    Ok(Self {
                        dst,
                        seen_value: false,
                        style,
                        len,
                    })
                }
                fn end(self) -> Result<&'d mut String, Error> {
                    if self.multiline_array() && self.seen_value {
                        self.dst.newline()?;
                    }
                    self.dst.close_array()?;
                    Ok(self.dst)
                }
                fn multiline_array(&self) -> bool {
                    self.style.multiline_array && 2 <= self.len.unwrap_or(usize::MAX)
                }
            }
            impl<'d> serde_core::ser::SerializeSeq for SerializeValueArray<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    if self.multiline_array() {
                        self.dst.newline()?;
                        self.dst.write_fmt(format_args!("    "))?;
                    } else {
                        if self.seen_value {
                            self.dst.val_sep()?;
                            self.dst.space()?;
                        }
                    }
                    self.seen_value = true;
                    value
                        .serialize(
                            super::ValueSerializer::with_style(self.dst, self.style),
                        )?;
                    if self.multiline_array() {
                        self.dst.val_sep()?;
                    }
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl<'d> serde_core::ser::SerializeTuple for SerializeValueArray<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeSeq::serialize_element(self, value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    serde_core::ser::SerializeSeq::end(self)
                }
            }
            impl<'d> serde_core::ser::SerializeTupleStruct for SerializeValueArray<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeSeq::serialize_element(self, value)
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    serde_core::ser::SerializeSeq::end(self)
                }
            }
            pub struct SerializeTupleVariant<'d> {
                inner: SerializeValueArray<'d>,
            }
            impl<'d> SerializeTupleVariant<'d> {
                pub(crate) fn tuple(
                    dst: &'d mut String,
                    variant: &'static str,
                    len: usize,
                    style: Style,
                ) -> Result<Self, Error> {
                    dst.open_inline_table()?;
                    dst.space()?;
                    dst.key(variant)?;
                    dst.space()?;
                    dst.keyval_sep()?;
                    dst.space()?;
                    Ok(Self {
                        inner: SerializeValueArray::seq(dst, style, Some(len))?,
                    })
                }
            }
            impl<'d> serde_core::ser::SerializeTupleVariant
            for SerializeTupleVariant<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeSeq::serialize_element(
                        &mut self.inner,
                        value,
                    )
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    let dst = self.inner.end()?;
                    dst.space()?;
                    dst.close_inline_table()?;
                    Ok(dst)
                }
            }
        }
        mod key {
            use toml_writer::TomlWrite as _;
            use super::Error;
            use crate::alloc_prelude::*;
            pub(crate) struct KeySerializer<'d> {
                pub(crate) dst: &'d mut String,
            }
            impl serde_core::ser::Serializer for KeySerializer<'_> {
                type Ok = ();
                type Error = Error;
                type SerializeSeq = serde_core::ser::Impossible<Self::Ok, Error>;
                type SerializeTuple = serde_core::ser::Impossible<Self::Ok, Error>;
                type SerializeTupleStruct = serde_core::ser::Impossible<Self::Ok, Error>;
                type SerializeTupleVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Error,
                >;
                type SerializeMap = serde_core::ser::Impossible<Self::Ok, Error>;
                type SerializeStruct = serde_core::ser::Impossible<Self::Ok, Error>;
                type SerializeStructVariant = serde_core::ser::Impossible<
                    Self::Ok,
                    Error,
                >;
                fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(v.to_string())?;
                    Ok(())
                }
                fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
                    let mut b = [0; 4];
                    let result = v.encode_utf8(&mut b);
                    self.dst.key(&*result)?;
                    Ok(())
                }
                fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(value)?;
                    Ok(())
                }
                fn serialize_bytes(
                    self,
                    _value: &[u8],
                ) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Err(Error::key_not_string())
                }
                fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_unit_struct(
                    self,
                    _name: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_unit_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    variant: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    self.dst.key(variant)?;
                    Ok(())
                }
                fn serialize_newtype_struct<T>(
                    self,
                    _name: &'static str,
                    value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    value.serialize(self)
                }
                fn serialize_newtype_variant<T>(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    Err(Error::key_not_string())
                }
                fn serialize_seq(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeSeq, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_tuple(
                    self,
                    _len: usize,
                ) -> Result<Self::SerializeTuple, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_tuple_struct(
                    self,
                    _name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_tuple_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_map(
                    self,
                    _len: Option<usize>,
                ) -> Result<Self::SerializeMap, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_struct(
                    self,
                    _name: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStruct, Self::Error> {
                    Err(Error::key_not_string())
                }
                fn serialize_struct_variant(
                    self,
                    _name: &'static str,
                    _variant_index: u32,
                    _variant: &'static str,
                    _len: usize,
                ) -> Result<Self::SerializeStructVariant, Self::Error> {
                    Err(Error::key_not_string())
                }
            }
        }
        mod map {
            use core::fmt::Write as _;
            use toml_writer::TomlWrite as _;
            use super::Error;
            use super::Style;
            use super::ValueSerializer;
            use super::array::SerializeTupleVariant;
            use super::array::SerializeValueArray;
            use super::key::KeySerializer;
            use crate::alloc_prelude::*;
            #[doc(hidden)]
            #[allow(clippy::large_enum_variant)]
            pub enum SerializeMap<'d> {
                Datetime(SerializeDatetime<'d>),
                Table(SerializeTable<'d>),
            }
            impl<'d> SerializeMap<'d> {
                pub(crate) fn map(
                    dst: &'d mut String,
                    style: Style,
                ) -> Result<Self, Error> {
                    Ok(Self::Table(SerializeTable::map(dst, style)?))
                }
                pub(crate) fn struct_(
                    name: &'static str,
                    dst: &'d mut String,
                    style: Style,
                ) -> Result<Self, Error> {
                    if toml_datetime::ser::is_datetime(name) {
                        Ok(Self::Datetime(SerializeDatetime::new(dst)))
                    } else {
                        Ok(Self::map(dst, style)?)
                    }
                }
            }
            impl<'d> serde_core::ser::SerializeMap for SerializeMap<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_key<T>(&mut self, input: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    match self {
                        Self::Datetime(s) => s.serialize_key(input),
                        Self::Table(s) => s.serialize_key(input),
                    }
                }
                fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    match self {
                        Self::Datetime(s) => s.serialize_value(value),
                        Self::Table(s) => s.serialize_value(value),
                    }
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    match self {
                        Self::Datetime(s) => s.end(),
                        Self::Table(s) => s.end(),
                    }
                }
            }
            impl<'d> serde_core::ser::SerializeStruct for SerializeMap<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    match self {
                        Self::Datetime(s) => s.serialize_field(key, value),
                        Self::Table(s) => s.serialize_field(key, value),
                    }
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    match self {
                        Self::Datetime(s) => s.end(),
                        Self::Table(s) => s.end(),
                    }
                }
            }
            #[doc(hidden)]
            pub struct SerializeDatetime<'d> {
                dst: &'d mut String,
                inner: toml_datetime::ser::DatetimeSerializer,
            }
            impl<'d> SerializeDatetime<'d> {
                pub(crate) fn new(dst: &'d mut String) -> Self {
                    Self {
                        dst,
                        inner: toml_datetime::ser::DatetimeSerializer::new(),
                    }
                }
            }
            impl<'d> serde_core::ser::SerializeMap for SerializeDatetime<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_key<T>(&mut self, _input: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "internal error: entered unreachable code: {0}",
                                format_args!(
                                    "datetimes should only be serialized as structs, not maps",
                                ),
                            ),
                        );
                    }
                }
                fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "internal error: entered unreachable code: {0}",
                                format_args!(
                                    "datetimes should only be serialized as structs, not maps",
                                ),
                            ),
                        );
                    }
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "internal error: entered unreachable code: {0}",
                                format_args!(
                                    "datetimes should only be serialized as structs, not maps",
                                ),
                            ),
                        );
                    }
                }
            }
            impl<'d> serde_core::ser::SerializeStruct for SerializeDatetime<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    self.inner.serialize_field(key, value).map_err(dt_err)?;
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    let value = self.inner.end().map_err(dt_err)?;
                    self.dst.write_fmt(format_args!("{0}", value))?;
                    Ok(self.dst)
                }
            }
            fn dt_err(err: toml_datetime::ser::SerializerError) -> Error {
                match err {
                    toml_datetime::ser::SerializerError::InvalidFormat(err) => {
                        Error::new(err)
                    }
                    _ => Error::date_invalid(),
                }
            }
            #[doc(hidden)]
            pub struct SerializeTable<'d> {
                dst: &'d mut String,
                seen_value: bool,
                key: Option<String>,
                style: Style,
            }
            impl<'d> SerializeTable<'d> {
                pub(crate) fn map(
                    dst: &'d mut String,
                    style: Style,
                ) -> Result<Self, Error> {
                    dst.open_inline_table()?;
                    Ok(Self {
                        dst,
                        seen_value: false,
                        key: None,
                        style,
                    })
                }
                pub(crate) fn end(self) -> Result<&'d mut String, Error> {
                    if self.seen_value {
                        self.dst.space()?;
                    }
                    self.dst.close_inline_table()?;
                    Ok(self.dst)
                }
            }
            impl<'d> serde_core::ser::SerializeMap for SerializeTable<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_key<T>(&mut self, input: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let mut encoded_key = String::new();
                    input
                        .serialize(KeySerializer {
                            dst: &mut encoded_key,
                        })?;
                    self.key = Some(encoded_key);
                    Ok(())
                }
                fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let encoded_key = self
                        .key
                        .take()
                        .expect("always called after `serialize_key`");
                    let mut encoded_value = String::new();
                    let mut is_none = false;
                    let value_serializer = MapValueSerializer::new(
                        &mut encoded_value,
                        &mut is_none,
                        self.style,
                    );
                    let res = value.serialize(value_serializer);
                    match res {
                        Ok(_) => {
                            use core::fmt::Write as _;
                            if self.seen_value {
                                self.dst.val_sep()?;
                            }
                            self.seen_value = true;
                            self.dst.space()?;
                            self.dst.write_fmt(format_args!("{0}", encoded_key))?;
                            self.dst.space()?;
                            self.dst.keyval_sep()?;
                            self.dst.space()?;
                            self.dst.write_fmt(format_args!("{0}", encoded_value))?;
                        }
                        Err(e) => {
                            if !(e == Error::unsupported_none() && is_none) {
                                return Err(e);
                            }
                        }
                    }
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            impl<'d> serde_core::ser::SerializeStruct for SerializeTable<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    let mut encoded_value = String::new();
                    let mut is_none = false;
                    let value_serializer = MapValueSerializer::new(
                        &mut encoded_value,
                        &mut is_none,
                        self.style,
                    );
                    let res = value.serialize(value_serializer);
                    match res {
                        Ok(_) => {
                            use core::fmt::Write as _;
                            if self.seen_value {
                                self.dst.val_sep()?;
                            }
                            self.seen_value = true;
                            self.dst.space()?;
                            self.dst.key(key)?;
                            self.dst.space()?;
                            self.dst.keyval_sep()?;
                            self.dst.space()?;
                            self.dst.write_fmt(format_args!("{0}", encoded_value))?;
                        }
                        Err(e) => {
                            if !(e == Error::unsupported_none() && is_none) {
                                return Err(e);
                            }
                        }
                    }
                    Ok(())
                }
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    self.end()
                }
            }
            pub(crate) struct MapValueSerializer<'d> {
                dst: &'d mut String,
                is_none: &'d mut bool,
                style: Style,
            }
            impl<'d> MapValueSerializer<'d> {
                pub(crate) fn new(
                    dst: &'d mut String,
                    is_none: &'d mut bool,
                    style: Style,
                ) -> Self {
                    Self { dst, is_none, style }
                }
            }
            impl<'d> serde_core::ser::Serializer for MapValueSerializer<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                type SerializeSeq = SerializeValueArray<'d>;
                type SerializeTuple = SerializeValueArray<'d>;
                type SerializeTupleStruct = SerializeValueArray<'d>;
                type SerializeTupleVariant = SerializeTupleVariant<'d>;
                type SerializeMap = SerializeMap<'d>;
                type SerializeStruct = SerializeMap<'d>;
                type SerializeStructVariant = SerializeStructVariant<'d>;
                fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_bool(v)
                }
                fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_i8(v)
                }
                fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_i16(v)
                }
                fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_i32(v)
                }
                fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_i64(v)
                }
                fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_u8(v)
                }
                fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_u16(v)
                }
                fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_u32(v)
                }
                fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_u64(v)
                }
                fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_f32(v)
                }
                fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_f64(v)
                }
                fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_char(v)
                }
                fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_str(v)
                }
                fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_bytes(value)
                }
                fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                    *self.is_none = true;
                    Err(Error::unsupported_none())
                }
                fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_some(value)
                }
                fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_unit()
                }
                fn serialize_unit_struct(
                    self,
                    name: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_unit_struct(name)
                }
                fn serialize_unit_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                ) -> Result<Self::Ok, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_unit_variant(name, variant_index, variant)
                }
                fn serialize_newtype_struct<T>(
                    self,
                    _name: &'static str,
                    value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    value.serialize(self)
                }
                fn serialize_newtype_variant<T>(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    value: &T,
                ) -> Result<Self::Ok, Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_newtype_variant(name, variant_index, variant, value)
                }
                fn serialize_seq(
                    self,
                    len: Option<usize>,
                ) -> Result<Self::SerializeSeq, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_seq(len)
                }
                fn serialize_tuple(
                    self,
                    len: usize,
                ) -> Result<Self::SerializeTuple, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_tuple(len)
                }
                fn serialize_tuple_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_tuple_struct(name, len)
                }
                fn serialize_tuple_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_tuple_variant(name, variant_index, variant, len)
                }
                fn serialize_map(
                    self,
                    len: Option<usize>,
                ) -> Result<Self::SerializeMap, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style).serialize_map(len)
                }
                fn serialize_struct(
                    self,
                    name: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStruct, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_struct(name, len)
                }
                fn serialize_struct_variant(
                    self,
                    name: &'static str,
                    variant_index: u32,
                    variant: &'static str,
                    len: usize,
                ) -> Result<Self::SerializeStructVariant, Self::Error> {
                    ValueSerializer::with_style(self.dst, self.style)
                        .serialize_struct_variant(name, variant_index, variant, len)
                }
            }
            pub struct SerializeStructVariant<'d> {
                inner: SerializeTable<'d>,
            }
            impl<'d> SerializeStructVariant<'d> {
                pub(crate) fn struct_(
                    dst: &'d mut String,
                    variant: &'static str,
                    _len: usize,
                    style: Style,
                ) -> Result<Self, Error> {
                    dst.open_inline_table()?;
                    dst.space()?;
                    dst.key(variant)?;
                    dst.space()?;
                    dst.keyval_sep()?;
                    dst.space()?;
                    Ok(Self {
                        inner: SerializeTable::map(dst, style)?,
                    })
                }
            }
            impl<'d> serde_core::ser::SerializeStructVariant
            for SerializeStructVariant<'d> {
                type Ok = &'d mut String;
                type Error = Error;
                #[inline]
                fn serialize_field<T>(
                    &mut self,
                    key: &'static str,
                    value: &T,
                ) -> Result<(), Self::Error>
                where
                    T: serde_core::ser::Serialize + ?Sized,
                {
                    serde_core::ser::SerializeStruct::serialize_field(
                        &mut self.inner,
                        key,
                        value,
                    )
                }
                #[inline]
                fn end(self) -> Result<Self::Ok, Self::Error> {
                    let dst = serde_core::ser::SerializeStruct::end(self.inner)?;
                    dst.space()?;
                    dst.close_inline_table()?;
                    Ok(dst)
                }
            }
        }
        use toml_writer::TomlWrite as _;
        use super::Error;
        use super::style::Style;
        use crate::alloc_prelude::*;
        #[allow(clippy::wildcard_imports)]
        pub(crate) use array::*;
        #[allow(clippy::wildcard_imports)]
        pub(crate) use key::*;
        #[allow(clippy::wildcard_imports)]
        pub(crate) use map::*;
        /// Serialization for TOML [values][crate::Value].
        ///
        /// This structure implements serialization support for TOML to serialize an
        /// arbitrary type to TOML. Note that the TOML format does not support all
        /// datatypes in Rust, such as enums, tuples, and tuple structs. These types
        /// will generate an error when serialized.
        ///
        /// Currently a serializer always writes its output to an in-memory `String`,
        /// which is passed in when creating the serializer itself.
        ///
        /// # Examples
        ///
        /// ```
        /// use serde::Serialize;
        ///
        /// #[derive(Serialize)]
        /// struct Config {
        ///     database: Database,
        /// }
        ///
        /// #[derive(Serialize)]
        /// struct Database {
        ///     ip: String,
        ///     port: Vec<u16>,
        ///     connection_max: u32,
        ///     enabled: bool,
        /// }
        ///
        /// let config = Config {
        ///     database: Database {
        ///         ip: "192.168.1.1".to_string(),
        ///         port: vec![8001, 8002, 8003],
        ///         connection_max: 5000,
        ///         enabled: false,
        ///     },
        /// };
        ///
        /// let mut value = String::new();
        /// serde::Serialize::serialize(
        ///     &config,
        ///     toml::ser::ValueSerializer::new(&mut value)
        /// ).unwrap();
        /// println!("{}", value)
        /// ```
        pub struct ValueSerializer<'d> {
            dst: &'d mut String,
            style: Style,
        }
        impl<'d> ValueSerializer<'d> {
            /// Creates a new serializer which will emit TOML into the buffer provided.
            ///
            /// The serializer can then be used to serialize a type after which the data
            /// will be present in `dst`.
            pub fn new(dst: &'d mut String) -> Self {
                Self {
                    dst,
                    style: Default::default(),
                }
            }
            pub(crate) fn with_style(dst: &'d mut String, style: Style) -> Self {
                Self { dst, style }
            }
        }
        impl<'d> serde_core::ser::Serializer for ValueSerializer<'d> {
            type Ok = &'d mut String;
            type Error = Error;
            type SerializeSeq = SerializeValueArray<'d>;
            type SerializeTuple = SerializeValueArray<'d>;
            type SerializeTupleStruct = SerializeValueArray<'d>;
            type SerializeTupleVariant = SerializeTupleVariant<'d>;
            type SerializeMap = SerializeMap<'d>;
            type SerializeStruct = SerializeMap<'d>;
            type SerializeStructVariant = SerializeStructVariant<'d>;
            fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_f32(self, mut v: f32) -> Result<Self::Ok, Self::Error> {
                if v.is_nan() {
                    v = v.copysign(1.0);
                }
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_f64(self, mut v: f64) -> Result<Self::Ok, Self::Error> {
                if v.is_nan() {
                    v = v.copysign(1.0);
                }
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
                self.dst.value(v)?;
                Ok(self.dst)
            }
            fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
                use serde_core::ser::Serialize;
                value.serialize(self)
            }
            fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_none())
            }
            fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                value.serialize(self)
            }
            fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some("unit")))
            }
            fn serialize_unit_struct(
                self,
                name: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                Err(Error::unsupported_type(Some(name)))
            }
            fn serialize_unit_variant(
                self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
            ) -> Result<Self::Ok, Self::Error> {
                self.serialize_str(variant)
            }
            fn serialize_newtype_struct<T>(
                self,
                _name: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                value.serialize(self)
            }
            fn serialize_newtype_variant<T>(
                self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                value: &T,
            ) -> Result<Self::Ok, Self::Error>
            where
                T: serde_core::ser::Serialize + ?Sized,
            {
                self.dst.open_inline_table()?;
                self.dst.space()?;
                self.dst.key(variant)?;
                self.dst.space()?;
                self.dst.keyval_sep()?;
                self.dst.space()?;
                value.serialize(ValueSerializer::with_style(self.dst, self.style))?;
                self.dst.space()?;
                self.dst.close_inline_table()?;
                Ok(self.dst)
            }
            fn serialize_seq(
                self,
                len: Option<usize>,
            ) -> Result<Self::SerializeSeq, Self::Error> {
                SerializeValueArray::seq(self.dst, self.style, len)
            }
            fn serialize_tuple(
                self,
                len: usize,
            ) -> Result<Self::SerializeTuple, Self::Error> {
                self.serialize_seq(Some(len))
            }
            fn serialize_tuple_struct(
                self,
                _name: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                self.serialize_seq(Some(len))
            }
            fn serialize_tuple_variant(
                self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                SerializeTupleVariant::tuple(self.dst, variant, len, self.style)
            }
            fn serialize_map(
                self,
                _len: Option<usize>,
            ) -> Result<Self::SerializeMap, Self::Error> {
                SerializeMap::map(self.dst, self.style)
            }
            fn serialize_struct(
                self,
                name: &'static str,
                _len: usize,
            ) -> Result<Self::SerializeStruct, Self::Error> {
                SerializeMap::struct_(name, self.dst, self.style)
            }
            fn serialize_struct_variant(
                self,
                _name: &'static str,
                _variant_index: u32,
                variant: &'static str,
                len: usize,
            ) -> Result<Self::SerializeStructVariant, Self::Error> {
                SerializeStructVariant::struct_(self.dst, variant, len, self.style)
            }
        }
    }
    use crate::alloc_prelude::*;
    pub use document::Buffer;
    pub use document::Serializer;
    pub use error::Error;
    pub(crate) use error::ErrorInner;
    pub use value::ValueSerializer;
    /// Serialize the given data structure as a String of TOML.
    ///
    /// Serialization can fail if `T`'s implementation of `Serialize` decides to
    /// fail, if `T` contains a map with non-string keys, or if `T` attempts to
    /// serialize an unsupported datatype such as an enum, tuple, or tuple struct.
    ///
    /// To serialize TOML values, instead of documents, see [`ValueSerializer`].
    ///
    /// # Examples
    ///
    /// ```
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Config {
    ///     database: Database,
    /// }
    ///
    /// #[derive(Serialize)]
    /// struct Database {
    ///     ip: String,
    ///     port: Vec<u16>,
    ///     connection_max: u32,
    ///     enabled: bool,
    /// }
    ///
    /// let config = Config {
    ///     database: Database {
    ///         ip: "192.168.1.1".to_string(),
    ///         port: vec![8001, 8002, 8003],
    ///         connection_max: 5000,
    ///         enabled: false,
    ///     },
    /// };
    ///
    /// let toml = toml::to_string(&config).unwrap();
    /// println!("{}", toml)
    /// ```
    pub fn to_string<T>(value: &T) -> Result<String, Error>
    where
        T: serde_core::ser::Serialize + ?Sized,
    {
        let mut output = Buffer::new();
        let serializer = Serializer::new(&mut output);
        value.serialize(serializer)?;
        Ok(output.to_string())
    }
    /// Serialize the given data structure as a "pretty" String of TOML.
    ///
    /// This is identical to `to_string` except the output string has a more
    /// "pretty" output. See `Serializer::pretty` for more details.
    ///
    /// To serialize TOML values, instead of documents, see [`ValueSerializer`].
    ///
    /// For greater customization, instead serialize to a
    /// [`toml_edit::DocumentMut`](https://docs.rs/toml_edit/latest/toml_edit/struct.DocumentMut.html).
    pub fn to_string_pretty<T>(value: &T) -> Result<String, Error>
    where
        T: serde_core::ser::Serialize + ?Sized,
    {
        let mut output = Buffer::new();
        let serializer = Serializer::pretty(&mut output);
        value.serialize(serializer)?;
        Ok(output.to_string())
    }
}
#[doc(hidden)]
pub mod macros {
    pub use serde_core::de::{Deserialize, IntoDeserializer};
    use crate::alloc_prelude::*;
    use crate::value::{Array, Table, Value};
    pub fn insert_toml(root: &mut Value, path: &[&str], value: Value) {
        *traverse(root, path) = value;
    }
    pub fn push_toml(root: &mut Value, path: &[&str]) {
        let target = traverse(root, path);
        if !target.is_array() {
            *target = Value::Array(Array::new());
        }
        target.as_array_mut().unwrap().push(Value::Table(Table::new()));
    }
    fn traverse<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Value {
        let mut cur = root;
        for &key in path {
            let cur1 = cur;
            let cur2 = if cur1.is_array() {
                cur1.as_array_mut().unwrap().last_mut().unwrap()
            } else {
                cur1
            };
            if !cur2.is_table() {
                *cur2 = Value::Table(Table::new());
            }
            if !cur2.as_table().unwrap().contains_key(key) {
                let empty = Value::Table(Table::new());
                cur2.as_table_mut().unwrap().insert(key.to_owned(), empty);
            }
            cur = cur2.as_table_mut().unwrap().get_mut(key).unwrap();
        }
        cur
    }
}
mod table {
    use serde_core::de;
    use serde_core::ser;
    use crate::Value;
    use crate::alloc_prelude::*;
    use crate::map::Map;
    /// Type representing a TOML table, payload of the `Value::Table` variant.
    ///
    /// By default it entries are stored in
    /// [lexicographic order](https://doc.rust-lang.org/std/primitive.str.html#impl-Ord-for-str)
    /// of the keys. Enable the `preserve_order` feature to store entries in the order they appear in
    /// the source file.
    pub type Table = Map<String, Value>;
    impl Table {
        /// Convert a `T` into `toml::Table`.
        ///
        /// This conversion can fail if `T`'s implementation of `Serialize` decides to
        /// fail, or if `T` contains a map with non-string keys.
        pub fn try_from<T>(value: T) -> Result<Self, crate::ser::Error>
        where
            T: ser::Serialize,
        {
            value.serialize(TableSerializer)
        }
        /// Interpret a `toml::Table` as an instance of type `T`.
        ///
        /// This conversion can fail if the structure of the `Table` does not match the structure
        /// expected by `T`, for example if `T` is a bool which can't be mapped to a `Table`. It can
        /// also fail if the structure is correct but `T`'s implementation of `Deserialize` decides
        /// that something is wrong with the data, for example required struct fields are missing from
        /// the TOML map or some number is too big to fit in the expected primitive type.
        pub fn try_into<'de, T>(self) -> Result<T, crate::de::Error>
        where
            T: de::Deserialize<'de>,
        {
            de::Deserialize::deserialize(self)
        }
    }
    impl core::fmt::Display for Table {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            crate::ser::to_string(self)
                .expect("Unable to represent value as string")
                .fmt(f)
        }
    }
    impl core::str::FromStr for Table {
        type Err = crate::de::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            crate::from_str(s)
        }
    }
    impl ser::Serialize for Table {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            use serde_core::ser::SerializeMap;
            let mut map = serializer.serialize_map(Some(self.len()))?;
            for (k, v) in self {
                map.serialize_key(k)?;
                map.serialize_value(v)?;
            }
            map.end()
        }
    }
    impl<'de> de::Deserialize<'de> for Table {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            struct Visitor;
            impl<'de> de::Visitor<'de> for Visitor {
                type Value = Map<String, Value>;
                fn expecting(
                    &self,
                    formatter: &mut core::fmt::Formatter<'_>,
                ) -> core::fmt::Result {
                    formatter.write_str("a map")
                }
                #[inline]
                fn visit_unit<E>(self) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(Map::new())
                }
                #[inline]
                fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: de::MapAccess<'de>,
                {
                    let mut values = Map::new();
                    while let Some((key, value)) = visitor.next_entry()? {
                        values.insert(key, value);
                    }
                    Ok(values)
                }
            }
            deserializer.deserialize_map(Visitor)
        }
    }
    impl<'de> de::Deserializer<'de> for Table {
        type Error = crate::de::Error;
        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            Value::Table(self).deserialize_any(visitor)
        }
        #[inline]
        fn deserialize_enum<V>(
            self,
            name: &'static str,
            variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            Value::Table(self).deserialize_enum(name, variants, visitor)
        }
        fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            Value::Table(self).deserialize_option(visitor)
        }
        fn deserialize_newtype_struct<V>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, crate::de::Error>
        where
            V: de::Visitor<'de>,
        {
            Value::Table(self).deserialize_newtype_struct(name, visitor)
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
    }
    impl de::IntoDeserializer<'_, crate::de::Error> for Table {
        type Deserializer = Self;
        fn into_deserializer(self) -> Self {
            self
        }
    }
    pub(crate) struct TableSerializer;
    impl ser::Serializer for TableSerializer {
        type Ok = Table;
        type Error = crate::ser::Error;
        type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
        type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
        type SerializeMap = SerializeMap;
        type SerializeStruct = SerializeMap;
        type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;
        fn serialize_bool(self, _value: bool) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_i8(self, _value: i8) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_i16(self, _value: i16) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_i32(self, _value: i32) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_i64(self, _value: i64) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_u8(self, _value: u8) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_u16(self, _value: u16) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_u32(self, _value: u32) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_u64(self, _value: u64) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_f32(self, _value: f32) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_f64(self, _value: f64) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_char(self, _value: char) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_str(self, _value: &str) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_bytes(self, _value: &[u8]) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_unit(self) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_unit_struct(
            self,
            _name: &'static str,
        ) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_unit_variant(
            self,
            name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
        ) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some(name)))
        }
        fn serialize_newtype_struct<T>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Table, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(self)
        }
        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Table, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(crate::value::ValueSerializer)?;
            let mut table = Table::new();
            table.insert(variant.to_owned(), value);
            Ok(table)
        }
        fn serialize_none(self) -> Result<Table, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_none())
        }
        fn serialize_some<T>(self, value: &T) -> Result<Table, crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(self)
        }
        fn serialize_seq(
            self,
            _len: Option<usize>,
        ) -> Result<Self::SerializeSeq, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_tuple(
            self,
            _len: usize,
        ) -> Result<Self::SerializeTuple, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(None))
        }
        fn serialize_tuple_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some(name)))
        }
        fn serialize_tuple_variant(
            self,
            name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some(name)))
        }
        fn serialize_map(
            self,
            _len: Option<usize>,
        ) -> Result<Self::SerializeMap, crate::ser::Error> {
            Ok(SerializeMap::new())
        }
        fn serialize_struct(
            self,
            _name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, crate::ser::Error> {
            self.serialize_map(Some(len))
        }
        fn serialize_struct_variant(
            self,
            name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, crate::ser::Error> {
            Err(crate::ser::Error::unsupported_type(Some(name)))
        }
    }
    pub(crate) struct SerializeMap {
        map: Table,
        next_key: Option<String>,
    }
    impl SerializeMap {
        pub(crate) fn new() -> Self {
            Self {
                map: Table::new(),
                next_key: None,
            }
        }
        pub(crate) fn with_capacity(capacity: usize) -> Self {
            Self {
                map: Table::with_capacity(capacity),
                next_key: None,
            }
        }
    }
    impl ser::SerializeMap for SerializeMap {
        type Ok = Table;
        type Error = crate::ser::Error;
        fn serialize_key<T>(&mut self, key: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            match Value::try_from(key)? {
                Value::String(s) => self.next_key = Some(s),
                _ => return Err(crate::ser::Error::key_not_string()),
            };
            Ok(())
        }
        fn serialize_value<T>(&mut self, value: &T) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let key = self.next_key.take();
            let key = key.expect("serialize_value called before serialize_key");
            match Value::try_from(value) {
                Ok(value) => {
                    self.map.insert(key, value);
                }
                Err(
                    crate::ser::Error { inner: crate::ser::ErrorInner::UnsupportedNone },
                ) => {}
                Err(e) => return Err(e),
            }
            Ok(())
        }
        fn end(self) -> Result<Table, crate::ser::Error> {
            Ok(self.map)
        }
    }
    impl ser::SerializeStruct for SerializeMap {
        type Ok = Table;
        type Error = crate::ser::Error;
        fn serialize_field<T>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), crate::ser::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            ser::SerializeMap::serialize_key(self, key)?;
            ser::SerializeMap::serialize_value(self, value)
        }
        fn end(self) -> Result<Table, crate::ser::Error> {
            ser::SerializeMap::end(self)
        }
    }
}
#[doc(inline)]
pub use crate::de::{Deserializer, from_slice, from_str};
#[doc(inline)]
pub use crate::ser::{Serializer, to_string, to_string_pretty};
#[doc(inline)]
pub use crate::value::Value;
pub use serde_spanned::Spanned;
pub use table::Table;
#[allow(unused_imports)]
use core::str::FromStr;
#[allow(unused_imports)]
use toml_datetime::Datetime;
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &after,
            &empty,
            &end,
            &end_of_line,
            &first_line,
            &second_line,
            &start,
            &start_of_second_line,
        ],
    )
}
