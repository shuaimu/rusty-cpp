#![feature(prelude_import)]
#![no_std]
//! [`IndexMap`] is a hash table where the iteration order of the key-value
//! pairs is independent of the hash values of the keys.
//!
//! [`IndexSet`] is a corresponding hash set using the same implementation and
//! with similar properties.
//!
//! ### Highlights
//!
//! [`IndexMap`] and [`IndexSet`] are drop-in compatible with the std `HashMap`
//! and `HashSet`, but they also have some features of note:
//!
//! - The ordering semantics (see their documentation for details)
//! - Sorting methods and the [`.pop()`][IndexMap::pop] methods.
//! - The [`Equivalent`] trait, which offers more flexible equality definitions
//!   between borrowed and owned versions of keys.
//! - The [`MutableKeys`][map::MutableKeys] trait, which gives opt-in mutable
//!   access to map keys, and [`MutableValues`][set::MutableValues] for sets.
//!
//! ### Feature Flags
//!
//! To reduce the amount of compiled code in the crate by default, certain
//! features are gated behind [feature flags]. These allow you to opt in to (or
//! out of) functionality. Below is a list of the features available in this
//! crate.
//!
//! * `std`: Enables features which require the Rust standard library. For more
//!   information see the section on [`no_std`].
//! * `rayon`: Enables parallel iteration and other parallel methods.
//! * `serde`: Adds implementations for [`Serialize`] and [`Deserialize`]
//!   to [`IndexMap`] and [`IndexSet`]. Alternative implementations for
//!   (de)serializing [`IndexMap`] as an ordered sequence are available in the
//!   [`map::serde_seq`] module.
//! * `arbitrary`: Adds implementations for the [`arbitrary::Arbitrary`] trait
//!   to [`IndexMap`] and [`IndexSet`].
//! * `quickcheck`: Adds implementations for the [`quickcheck::Arbitrary`] trait
//!   to [`IndexMap`] and [`IndexSet`].
//! * `borsh` (**deprecated**): Adds implementations for [`BorshSerialize`] and
//!   [`BorshDeserialize`] to [`IndexMap`] and [`IndexSet`]. Due to a cyclic
//!   dependency that arose between [`borsh`] and `indexmap`, `borsh v1.5.6`
//!   added an `indexmap` feature that should be used instead of enabling the
//!   feature here.
//!
//! _Note: only the `std` feature is enabled by default._
//!
//! [feature flags]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section
//! [`no_std`]: #no-standard-library-targets
//! [`Serialize`]: `::serde_core::Serialize`
//! [`Deserialize`]: `::serde_core::Deserialize`
//! [`BorshSerialize`]: `::borsh::BorshSerialize`
//! [`BorshDeserialize`]: `::borsh::BorshDeserialize`
//! [`borsh`]: `::borsh`
//! [`arbitrary::Arbitrary`]: `::arbitrary::Arbitrary`
//! [`quickcheck::Arbitrary`]: `::quickcheck::Arbitrary`
//!
//! ### Alternate Hashers
//!
//! [`IndexMap`] and [`IndexSet`] have a default hasher type
//! [`S = RandomState`][std::hash::RandomState],
//! just like the standard `HashMap` and `HashSet`, which is resistant to
//! HashDoS attacks but not the most performant. Type aliases can make it easier
//! to use alternate hashers:
//!
//! ```
//! use fnv::FnvBuildHasher;
//! use indexmap::{IndexMap, IndexSet};
//!
//! type FnvIndexMap<K, V> = IndexMap<K, V, FnvBuildHasher>;
//! type FnvIndexSet<T> = IndexSet<T, FnvBuildHasher>;
//!
//! let std: IndexSet<i32> = (0..100).collect();
//! let fnv: FnvIndexSet<i32> = (0..100).collect();
//! assert_eq!(std, fnv);
//! ```
//!
//! ### Rust Version
//!
//! This version of indexmap requires Rust 1.82 or later.
//!
//! The indexmap 2.x release series will use a carefully considered version
//! upgrade policy, where in a later 2.x version, we will raise the minimum
//! required Rust version.
//!
//! ## No Standard Library Targets
//!
//! This crate supports being built without `std`, requiring `alloc` instead.
//! This is chosen by disabling the default "std" cargo feature, by adding
//! `default-features = false` to your dependency specification.
//!
//! - Creating maps and sets using [`new`][IndexMap::new] and
//!   [`with_capacity`][IndexMap::with_capacity] is unavailable without `std`.
//!   Use methods [`IndexMap::default`], [`with_hasher`][IndexMap::with_hasher],
//!   [`with_capacity_and_hasher`][IndexMap::with_capacity_and_hasher] instead.
//!   A no-std compatible hasher will be needed as well, for example
//!   from the crate `twox-hash`.
//! - Macros [`indexmap!`] and [`indexset!`] are unavailable without `std`. Use
//!   the macros [`indexmap_with_default!`] and [`indexset_with_default!`] instead.
extern crate core;
#[prelude_import]
use core::prelude::rust_2021::*;
extern crate alloc;
#[macro_use]
extern crate std;
mod arbitrary {}
mod inner {
    //! This is the core implementation that doesn't depend on the hasher at all.
    //!
    //! The methods of `Core` don't use any Hash properties of K.
    //!
    //! It's cleaner to separate them out, then the compiler checks that we are not
    //! using Hash at all in these methods.
    //!
    //! However, we should probably not let this show in the public API or docs.
    mod entry {
        use super::{equivalent, get_hash, Bucket, Core};
        use crate::map::{Entry, IndexedEntry};
        use crate::HashValue;
        use core::cmp::Ordering;
        use core::mem;
        impl<'a, K, V> Entry<'a, K, V> {
            pub(crate) fn new(map: &'a mut Core<K, V>, hash: HashValue, key: K) -> Self
            where
                K: Eq,
            {
                let eq = equivalent(&key, &map.entries);
                match map.indices.find_entry(hash.get(), eq) {
                    Ok(entry) => {
                        Entry::Occupied(OccupiedEntry {
                            bucket: entry.bucket_index(),
                            index: *entry.get(),
                            map,
                        })
                    }
                    Err(_) => Entry::Vacant(VacantEntry { map, hash, key }),
                }
            }
        }
        /// A view into an occupied entry in an [`IndexMap`][crate::IndexMap].
        /// It is part of the [`Entry`] enum.
        pub struct OccupiedEntry<'a, K, V> {
            map: &'a mut Core<K, V>,
            index: usize,
            bucket: usize,
        }
        impl<'a, K, V> OccupiedEntry<'a, K, V> {
            /// Constructor for `RawEntryMut::from_hash`
            pub(crate) fn from_hash<F>(
                map: &'a mut Core<K, V>,
                hash: HashValue,
                mut is_match: F,
            ) -> Result<Self, &'a mut Core<K, V>>
            where
                F: FnMut(&K) -> bool,
            {
                let entries = &*map.entries;
                let eq = move |&i: &usize| is_match(&entries[i].key);
                match map.indices.find_entry(hash.get(), eq) {
                    Ok(entry) => {
                        Ok(OccupiedEntry {
                            bucket: entry.bucket_index(),
                            index: *entry.get(),
                            map,
                        })
                    }
                    Err(_) => Err(map),
                }
            }
            pub(crate) fn into_core(self) -> &'a mut Core<K, V> {
                self.map
            }
            pub(crate) fn get_bucket(&self) -> &Bucket<K, V> {
                &self.map.entries[self.index]
            }
            pub(crate) fn get_bucket_mut(&mut self) -> &mut Bucket<K, V> {
                &mut self.map.entries[self.index]
            }
            pub(crate) fn into_bucket(self) -> &'a mut Bucket<K, V> {
                &mut self.map.entries[self.index]
            }
            /// Return the index of the key-value pair
            #[inline]
            pub fn index(&self) -> usize {
                self.index
            }
            /// Gets a reference to the entry's key in the map.
            ///
            /// Note that this is not the key that was used to find the entry. There may be an observable
            /// difference if the key type has any distinguishing features outside of `Hash` and `Eq`, like
            /// extra fields or the memory address of an allocation.
            pub fn key(&self) -> &K {
                &self.get_bucket().key
            }
            /// Gets a reference to the entry's value in the map.
            pub fn get(&self) -> &V {
                &self.get_bucket().value
            }
            /// Gets a mutable reference to the entry's value in the map.
            ///
            /// If you need a reference which may outlive the destruction of the
            /// [`Entry`] value, see [`into_mut`][Self::into_mut].
            pub fn get_mut(&mut self) -> &mut V {
                &mut self.get_bucket_mut().value
            }
            /// Converts into a mutable reference to the entry's value in the map,
            /// with a lifetime bound to the map itself.
            pub fn into_mut(self) -> &'a mut V {
                &mut self.into_bucket().value
            }
            /// Sets the value of the entry to `value`, and returns the entry's old value.
            pub fn insert(&mut self, value: V) -> V {
                mem::replace(self.get_mut(), value)
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// **NOTE:** This is equivalent to [`.swap_remove()`][Self::swap_remove], replacing this
            /// entry's position with the last element, and it is deprecated in favor of calling that
            /// explicitly. If you need to preserve the relative order of the keys in the map, use
            /// [`.shift_remove()`][Self::shift_remove] instead.
            #[deprecated(
                note = "`remove` disrupts the map order -- \
        use `swap_remove` or `shift_remove` for explicit behavior."
            )]
            pub fn remove(self) -> V {
                self.swap_remove()
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove(self) -> V {
                self.swap_remove_entry().1
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove(self) -> V {
                self.shift_remove_entry().1
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// **NOTE:** This is equivalent to [`.swap_remove_entry()`][Self::swap_remove_entry],
            /// replacing this entry's position with the last element, and it is deprecated in favor of
            /// calling that explicitly. If you need to preserve the relative order of the keys in the map,
            /// use [`.shift_remove_entry()`][Self::shift_remove_entry] instead.
            #[deprecated(
                note = "`remove_entry` disrupts the map order -- \
        use `swap_remove_entry` or `shift_remove_entry` for explicit behavior."
            )]
            pub fn remove_entry(self) -> (K, V) {
                self.swap_remove_entry()
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove_entry(mut self) -> (K, V) {
                self.remove_index();
                self.map.swap_remove_finish(self.index)
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove_entry(mut self) -> (K, V) {
                self.remove_index();
                self.map.shift_remove_finish(self.index)
            }
            fn remove_index(&mut self) {
                let entry = self.map.indices.get_bucket_entry(self.bucket).unwrap();
                if true {
                    match (&*entry.get(), &self.index) {
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
                entry.remove();
            }
            /// Moves the position of the entry to a new index
            /// by shifting all other entries in-between.
            ///
            /// This is equivalent to [`IndexMap::move_index`][`crate::IndexMap::move_index`]
            /// coming `from` the current [`.index()`][Self::index].
            ///
            /// * If `self.index() < to`, the other pairs will shift down while the targeted pair moves up.
            /// * If `self.index() > to`, the other pairs will shift up while the targeted pair moves down.
            ///
            /// ***Panics*** if `to` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn move_index(self, to: usize) {
                if self.index != to {
                    let _ = self.map.entries[to];
                    self.map.move_index_inner(self.index, to);
                    self.update_index(to);
                }
            }
            /// Swaps the position of entry with another.
            ///
            /// This is equivalent to [`IndexMap::swap_indices`][`crate::IndexMap::swap_indices`]
            /// with the current [`.index()`][Self::index] as one of the two being swapped.
            ///
            /// ***Panics*** if the `other` index is out of bounds.
            ///
            /// Computes in **O(1)** time (average).
            #[track_caller]
            pub fn swap_indices(self, other: usize) {
                if self.index != other {
                    let hash = self.map.entries[other].hash;
                    let other_mut = self
                        .map
                        .indices
                        .find_mut(hash.get(), move |&i| i == other);
                    *other_mut.expect("index not found") = self.index;
                    self.map.entries.swap(self.index, other);
                    self.update_index(other);
                }
            }
            fn update_index(self, to: usize) {
                let index = self.map.indices.get_bucket_mut(self.bucket).unwrap();
                if true {
                    match (&*index, &self.index) {
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
                *index = to;
            }
        }
        impl<'a, K, V> From<IndexedEntry<'a, K, V>> for OccupiedEntry<'a, K, V> {
            fn from(other: IndexedEntry<'a, K, V>) -> Self {
                let index = other.index();
                let map = other.into_core();
                let hash = map.entries[index].hash;
                let bucket = map
                    .indices
                    .find_bucket_index(hash.get(), move |&i| i == index)
                    .expect("index not found");
                Self { map, index, bucket }
            }
        }
        /// A view into a vacant entry in an [`IndexMap`][crate::IndexMap].
        /// It is part of the [`Entry`] enum.
        pub struct VacantEntry<'a, K, V> {
            map: &'a mut Core<K, V>,
            hash: HashValue,
            key: K,
        }
        impl<'a, K, V> VacantEntry<'a, K, V> {
            /// Return the index where a key-value pair may be inserted.
            pub fn index(&self) -> usize {
                self.map.indices.len()
            }
            /// Gets a reference to the key that was used to find the entry.
            pub fn key(&self) -> &K {
                &self.key
            }
            pub(crate) fn key_mut(&mut self) -> &mut K {
                &mut self.key
            }
            /// Takes ownership of the key, leaving the entry vacant.
            pub fn into_key(self) -> K {
                self.key
            }
            /// Inserts the entry's key and the given value into the map, and returns a mutable reference
            /// to the value.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn insert(self, value: V) -> &'a mut V {
                let Self { map, hash, key } = self;
                map.insert_unique(hash, key, value).value_mut()
            }
            /// Inserts the entry's key and the given value into the map, and returns an `OccupiedEntry`.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
                let Self { map, hash, key } = self;
                let index = map.indices.len();
                if true {
                    match (&index, &map.entries.len()) {
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
                let bucket = map
                    .indices
                    .insert_unique(hash.get(), index, get_hash(&map.entries))
                    .bucket_index();
                map.push_entry(hash, key, value);
                OccupiedEntry {
                    map,
                    index,
                    bucket,
                }
            }
            /// Inserts the entry's key and the given value into the map at its ordered
            /// position among sorted keys, and returns the new index and a mutable
            /// reference to the value.
            ///
            /// If the existing keys are **not** already sorted, then the insertion
            /// index is unspecified (like [`slice::binary_search`]), but the key-value
            /// pair is inserted at that position regardless.
            ///
            /// Computes in **O(n)** time (average).
            pub fn insert_sorted(self, value: V) -> (usize, &'a mut V)
            where
                K: Ord,
            {
                let slice = crate::map::Slice::from_slice(&self.map.entries);
                let i = slice.binary_search_keys(&self.key).unwrap_err();
                (i, self.shift_insert(i, value))
            }
            /// Inserts the entry's key and the given value into the map at its ordered
            /// position among keys sorted by `cmp`, and returns the new index and a
            /// mutable reference to the value.
            ///
            /// If the existing keys are **not** already sorted, then the insertion
            /// index is unspecified (like [`slice::binary_search`]), but the key-value
            /// pair is inserted at that position regardless.
            ///
            /// Computes in **O(n)** time (average).
            pub fn insert_sorted_by<F>(self, value: V, mut cmp: F) -> (usize, &'a mut V)
            where
                F: FnMut(&K, &V, &K, &V) -> Ordering,
            {
                let slice = crate::map::Slice::from_slice(&self.map.entries);
                let (Ok(i) | Err(i)) = slice
                    .binary_search_by(|k, v| cmp(k, v, &self.key, &value));
                (i, self.shift_insert(i, value))
            }
            /// Inserts the entry's key and the given value into the map at its ordered
            /// position using a sort-key extraction function, and returns the new index
            /// and a mutable reference to the value.
            ///
            /// If the existing keys are **not** already sorted, then the insertion
            /// index is unspecified (like [`slice::binary_search`]), but the key-value
            /// pair is inserted at that position regardless.
            ///
            /// Computes in **O(n)** time (average).
            pub fn insert_sorted_by_key<B, F>(
                self,
                value: V,
                mut sort_key: F,
            ) -> (usize, &'a mut V)
            where
                B: Ord,
                F: FnMut(&K, &V) -> B,
            {
                let search_key = sort_key(&self.key, &value);
                let slice = crate::map::Slice::from_slice(&self.map.entries);
                let (Ok(i) | Err(i)) = slice.binary_search_by_key(&search_key, sort_key);
                (i, self.shift_insert(i, value))
            }
            /// Inserts the entry's key and the given value into the map at the given index,
            /// shifting others to the right, and returns a mutable reference to the value.
            ///
            /// ***Panics*** if `index` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn shift_insert(self, index: usize, value: V) -> &'a mut V {
                self.map
                    .shift_insert_unique(index, self.hash, self.key, value)
                    .value_mut()
            }
            /// Replaces the key at the given index with this entry's key, returning the
            /// old key and an `OccupiedEntry` for that index.
            ///
            /// ***Panics*** if `index` is out of bounds.
            ///
            /// Computes in **O(1)** time (average).
            #[track_caller]
            pub fn replace_index(self, index: usize) -> (K, OccupiedEntry<'a, K, V>) {
                let Self { map, hash, key } = self;
                let old_hash = map.entries[index].hash;
                map.indices
                    .find_entry(old_hash.get(), move |&i| i == index)
                    .expect("index not found")
                    .remove();
                let bucket = map
                    .indices
                    .insert_unique(hash.get(), index, get_hash(&map.entries))
                    .bucket_index();
                let entry = &mut map.entries[index];
                entry.hash = hash;
                let old_key = mem::replace(&mut entry.key, key);
                (
                    old_key,
                    OccupiedEntry {
                        map,
                        index,
                        bucket,
                    },
                )
            }
        }
    }
    mod extract {
        #![allow(unsafe_code)]
        use super::{Bucket, Core};
        use crate::util::simplify_range;
        use core::ops::RangeBounds;
        impl<K, V> Core<K, V> {
            #[track_caller]
            pub(crate) fn extract<R>(&mut self, range: R) -> ExtractCore<'_, K, V>
            where
                R: RangeBounds<usize>,
            {
                let range = simplify_range(range, self.entries.len());
                match (&self.entries.len(), &self.indices.len()) {
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
                unsafe {
                    self.entries.set_len(range.start);
                }
                ExtractCore {
                    map: self,
                    new_len: range.start,
                    current: range.start,
                    end: range.end,
                }
            }
        }
        pub(crate) struct ExtractCore<'a, K, V> {
            map: &'a mut Core<K, V>,
            new_len: usize,
            current: usize,
            end: usize,
        }
        impl<K, V> Drop for ExtractCore<'_, K, V> {
            fn drop(&mut self) {
                let old_len = self.map.indices.len();
                let mut new_len = self.new_len;
                if true {
                    if !(new_len <= self.current) {
                        ::core::panicking::panic(
                            "assertion failed: new_len <= self.current",
                        )
                    }
                }
                if true {
                    if !(self.current <= self.end) {
                        ::core::panicking::panic(
                            "assertion failed: self.current <= self.end",
                        )
                    }
                }
                if true {
                    if !(self.current <= old_len) {
                        ::core::panicking::panic(
                            "assertion failed: self.current <= old_len",
                        )
                    }
                }
                if true {
                    if !(old_len <= self.map.entries.capacity()) {
                        ::core::panicking::panic(
                            "assertion failed: old_len <= self.map.entries.capacity()",
                        )
                    }
                }
                unsafe {
                    if new_len == self.current {
                        new_len = old_len;
                    } else if self.current < old_len {
                        let tail_len = old_len - self.current;
                        let base = self.map.entries.as_mut_ptr();
                        let src = base.add(self.current);
                        let dest = base.add(new_len);
                        src.copy_to(dest, tail_len);
                        new_len += tail_len;
                    }
                    self.map.entries.set_len(new_len);
                }
                if new_len != old_len {
                    self.map.rebuild_hash_table();
                }
            }
        }
        impl<K, V> ExtractCore<'_, K, V> {
            pub(crate) fn extract_if<F>(&mut self, mut pred: F) -> Option<Bucket<K, V>>
            where
                F: FnMut(&mut Bucket<K, V>) -> bool,
            {
                if true {
                    if !(self.end <= self.map.entries.capacity()) {
                        ::core::panicking::panic(
                            "assertion failed: self.end <= self.map.entries.capacity()",
                        )
                    }
                }
                let base = self.map.entries.as_mut_ptr();
                while self.current < self.end {
                    unsafe {
                        let item = base.add(self.current);
                        if pred(&mut *item) {
                            self.current += 1;
                            return Some(item.read());
                        } else {
                            if self.new_len != self.current {
                                if true {
                                    if !(self.new_len < self.current) {
                                        ::core::panicking::panic(
                                            "assertion failed: self.new_len < self.current",
                                        )
                                    }
                                }
                                let dest = base.add(self.new_len);
                                item.copy_to_nonoverlapping(dest, 1);
                            }
                            self.current += 1;
                            self.new_len += 1;
                        }
                    }
                }
                None
            }
            pub(crate) fn remaining(&self) -> usize {
                self.end - self.current
            }
        }
    }
    use alloc::vec::{self, Vec};
    use core::mem;
    use core::ops::RangeBounds;
    use hashbrown::hash_table;
    use crate::util::simplify_range;
    use crate::{Bucket, Equivalent, HashValue, TryReserveError};
    type Indices = hash_table::HashTable<usize>;
    type Entries<K, V> = Vec<Bucket<K, V>>;
    pub use entry::{OccupiedEntry, VacantEntry};
    pub(crate) use extract::ExtractCore;
    /// Core of the map that does not depend on S
    pub(crate) struct Core<K, V> {
        /// indices mapping from the entry hash to its index.
        indices: Indices,
        /// entries is a dense vec maintaining entry order.
        entries: Entries<K, V>,
    }
    #[inline(always)]
    fn get_hash<K, V>(
        entries: &[Bucket<K, V>],
    ) -> impl Fn(&usize) -> u64 + use<'_, K, V> {
        move |&i| entries[i].hash.get()
    }
    #[inline]
    fn equivalent<'a, K, V, Q: ?Sized + Equivalent<K>>(
        key: &'a Q,
        entries: &'a [Bucket<K, V>],
    ) -> impl Fn(&usize) -> bool + use<'a, K, V, Q> {
        move |&i| Q::equivalent(key, &entries[i].key)
    }
    #[inline]
    fn erase_index(table: &mut Indices, hash: HashValue, index: usize) {
        if let Ok(entry) = table.find_entry(hash.get(), move |&i| i == index) {
            entry.remove();
        } else if true {
            {
                ::core::panicking::panic_fmt(format_args!("index not found"));
            };
        }
    }
    #[inline]
    fn update_index(table: &mut Indices, hash: HashValue, old: usize, new: usize) {
        let index = table
            .find_mut(hash.get(), move |&i| i == old)
            .expect("index not found");
        *index = new;
    }
    /// Inserts many entries into the indices table without reallocating,
    /// and without regard for duplication.
    ///
    /// ***Panics*** if there is not sufficient capacity already.
    fn insert_bulk_no_grow<K, V>(indices: &mut Indices, entries: &[Bucket<K, V>]) {
        if !(indices.capacity() - indices.len() >= entries.len()) {
            ::core::panicking::panic(
                "assertion failed: indices.capacity() - indices.len() >= entries.len()",
            )
        }
        for entry in entries {
            indices
                .insert_unique(
                    entry.hash.get(),
                    indices.len(),
                    |_| ::core::panicking::panic(
                        "internal error: entered unreachable code",
                    ),
                );
        }
    }
    impl<K, V> Clone for Core<K, V>
    where
        K: Clone,
        V: Clone,
    {
        fn clone(&self) -> Self {
            let mut new = Self::new();
            new.clone_from(self);
            new
        }
        fn clone_from(&mut self, other: &Self) {
            self.indices.clone_from(&other.indices);
            if self.entries.capacity() < other.entries.len() {
                let additional = other.entries.len() - self.entries.len();
                self.reserve_entries(additional);
            }
            self.entries.clone_from(&other.entries);
        }
    }
    impl<K, V> Core<K, V> {
        /// The maximum capacity before the `entries` allocation would exceed `isize::MAX`.
        const MAX_ENTRIES_CAPACITY: usize = (isize::MAX as usize)
            / size_of::<Bucket<K, V>>();
        #[inline]
        pub(crate) const fn new() -> Self {
            Core {
                indices: Indices::new(),
                entries: Vec::new(),
            }
        }
        #[inline]
        pub(crate) fn with_capacity(n: usize) -> Self {
            Core {
                indices: Indices::with_capacity(n),
                entries: Vec::with_capacity(n),
            }
        }
        #[inline]
        pub(crate) fn into_entries(self) -> Entries<K, V> {
            self.entries
        }
        #[inline]
        pub(crate) fn as_entries(&self) -> &[Bucket<K, V>] {
            &self.entries
        }
        #[inline]
        pub(crate) fn as_entries_mut(&mut self) -> &mut [Bucket<K, V>] {
            &mut self.entries
        }
        pub(crate) fn with_entries<F>(&mut self, f: F)
        where
            F: FnOnce(&mut [Bucket<K, V>]),
        {
            f(&mut self.entries);
            self.rebuild_hash_table();
        }
        #[inline]
        pub(crate) fn len(&self) -> usize {
            if true {
                match (&self.entries.len(), &self.indices.len()) {
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
            self.indices.len()
        }
        #[inline]
        pub(crate) fn capacity(&self) -> usize {
            Ord::min(self.indices.capacity(), self.entries.capacity())
        }
        pub(crate) fn clear(&mut self) {
            self.indices.clear();
            self.entries.clear();
        }
        pub(crate) fn truncate(&mut self, len: usize) {
            if len < self.len() {
                self.erase_indices(len, self.entries.len());
                self.entries.truncate(len);
            }
        }
        #[track_caller]
        pub(crate) fn drain<R>(&mut self, range: R) -> vec::Drain<'_, Bucket<K, V>>
        where
            R: RangeBounds<usize>,
        {
            let range = simplify_range(range, self.entries.len());
            self.erase_indices(range.start, range.end);
            self.entries.drain(range)
        }
        #[track_caller]
        pub(crate) fn split_off(&mut self, at: usize) -> Self {
            let len = self.entries.len();
            if !(at <= len) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "index out of bounds: the len is {0} but the index is {1}. Expected index <= len",
                            len,
                            at,
                        ),
                    );
                }
            }
            self.erase_indices(at, self.entries.len());
            let entries = self.entries.split_off(at);
            let mut indices = Indices::with_capacity(entries.len());
            insert_bulk_no_grow(&mut indices, &entries);
            Self { indices, entries }
        }
        #[track_caller]
        pub(crate) fn split_splice<R>(
            &mut self,
            range: R,
        ) -> (Self, vec::IntoIter<Bucket<K, V>>)
        where
            R: RangeBounds<usize>,
        {
            let range = simplify_range(range, self.len());
            self.erase_indices(range.start, self.entries.len());
            let entries = self.entries.split_off(range.end);
            let drained = self.entries.split_off(range.start);
            let mut indices = Indices::with_capacity(entries.len());
            insert_bulk_no_grow(&mut indices, &entries);
            (Self { indices, entries }, drained.into_iter())
        }
        /// Append from another map without checking whether items already exist.
        pub(crate) fn append_unchecked(&mut self, other: &mut Self) {
            self.reserve(other.len());
            insert_bulk_no_grow(&mut self.indices, &other.entries);
            self.entries.append(&mut other.entries);
            other.indices.clear();
        }
        /// Reserve capacity for `additional` more key-value pairs.
        pub(crate) fn reserve(&mut self, additional: usize) {
            self.indices.reserve(additional, get_hash(&self.entries));
            if additional > self.entries.capacity() - self.entries.len() {
                self.reserve_entries(additional);
            }
        }
        /// Reserve capacity for `additional` more key-value pairs, without over-allocating.
        pub(crate) fn reserve_exact(&mut self, additional: usize) {
            self.indices.reserve(additional, get_hash(&self.entries));
            self.entries.reserve_exact(additional);
        }
        /// Try to reserve capacity for `additional` more key-value pairs.
        pub(crate) fn try_reserve(
            &mut self,
            additional: usize,
        ) -> Result<(), TryReserveError> {
            self.indices
                .try_reserve(additional, get_hash(&self.entries))
                .map_err(TryReserveError::from_hashbrown)?;
            if additional > self.entries.capacity() - self.entries.len() {
                self.try_reserve_entries(additional)
            } else {
                Ok(())
            }
        }
        /// Try to reserve entries capacity, rounded up to match the indices
        fn try_reserve_entries(
            &mut self,
            additional: usize,
        ) -> Result<(), TryReserveError> {
            let new_capacity = Ord::min(
                self.indices.capacity(),
                Self::MAX_ENTRIES_CAPACITY,
            );
            let try_add = new_capacity - self.entries.len();
            if try_add > additional && self.entries.try_reserve_exact(try_add).is_ok() {
                return Ok(());
            }
            self.entries
                .try_reserve_exact(additional)
                .map_err(TryReserveError::from_alloc)
        }
        /// Try to reserve capacity for `additional` more key-value pairs, without over-allocating.
        pub(crate) fn try_reserve_exact(
            &mut self,
            additional: usize,
        ) -> Result<(), TryReserveError> {
            self.indices
                .try_reserve(additional, get_hash(&self.entries))
                .map_err(TryReserveError::from_hashbrown)?;
            self.entries
                .try_reserve_exact(additional)
                .map_err(TryReserveError::from_alloc)
        }
        /// Shrink the capacity of the map with a lower bound
        pub(crate) fn shrink_to(&mut self, min_capacity: usize) {
            self.indices.shrink_to(min_capacity, get_hash(&self.entries));
            self.entries.shrink_to(min_capacity);
        }
        /// Remove the last key-value pair
        pub(crate) fn pop(&mut self) -> Option<(K, V)> {
            if let Some(entry) = self.entries.pop() {
                let last = self.entries.len();
                erase_index(&mut self.indices, entry.hash, last);
                Some((entry.key, entry.value))
            } else {
                None
            }
        }
        /// Return the index in `entries` where an equivalent key can be found
        pub(crate) fn get_index_of<Q>(&self, hash: HashValue, key: &Q) -> Option<usize>
        where
            Q: ?Sized + Equivalent<K>,
        {
            let eq = equivalent(key, &self.entries);
            self.indices.find(hash.get(), eq).copied()
        }
        /// Return the index in `entries` where an equivalent key can be found
        pub(crate) fn get_index_of_raw<F>(
            &self,
            hash: HashValue,
            mut is_match: F,
        ) -> Option<usize>
        where
            F: FnMut(&K) -> bool,
        {
            let eq = move |&i: &usize| is_match(&self.entries[i].key);
            self.indices.find(hash.get(), eq).copied()
        }
        /// Append a key-value pair to `entries`,
        /// *without* checking whether it already exists.
        fn push_entry(&mut self, hash: HashValue, key: K, value: V) {
            if self.entries.len() == self.entries.capacity() {
                self.reserve_entries(1);
            }
            self.entries.push(Bucket { hash, key, value });
        }
        pub(crate) fn insert_full(
            &mut self,
            hash: HashValue,
            key: K,
            value: V,
        ) -> (usize, Option<V>)
        where
            K: Eq,
        {
            let eq = equivalent(&key, &self.entries);
            let hasher = get_hash(&self.entries);
            match self.indices.entry(hash.get(), eq, hasher) {
                hash_table::Entry::Occupied(entry) => {
                    let i = *entry.get();
                    (i, Some(mem::replace(&mut self.entries[i].value, value)))
                }
                hash_table::Entry::Vacant(entry) => {
                    let i = self.entries.len();
                    entry.insert(i);
                    self.push_entry(hash, key, value);
                    if true {
                        match (&self.indices.len(), &self.entries.len()) {
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
                    (i, None)
                }
            }
        }
        /// Same as `insert_full`, except it also replaces the key
        pub(crate) fn replace_full(
            &mut self,
            hash: HashValue,
            key: K,
            value: V,
        ) -> (usize, Option<(K, V)>)
        where
            K: Eq,
        {
            let eq = equivalent(&key, &self.entries);
            let hasher = get_hash(&self.entries);
            match self.indices.entry(hash.get(), eq, hasher) {
                hash_table::Entry::Occupied(entry) => {
                    let i = *entry.get();
                    let entry = &mut self.entries[i];
                    let kv = (
                        mem::replace(&mut entry.key, key),
                        mem::replace(&mut entry.value, value),
                    );
                    (i, Some(kv))
                }
                hash_table::Entry::Vacant(entry) => {
                    let i = self.entries.len();
                    entry.insert(i);
                    self.push_entry(hash, key, value);
                    if true {
                        match (&self.indices.len(), &self.entries.len()) {
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
                    (i, None)
                }
            }
        }
        /// Remove an entry by shifting all entries that follow it
        pub(crate) fn shift_remove_full<Q>(
            &mut self,
            hash: HashValue,
            key: &Q,
        ) -> Option<(usize, K, V)>
        where
            Q: ?Sized + Equivalent<K>,
        {
            let eq = equivalent(key, &self.entries);
            let (index, _) = self.indices.find_entry(hash.get(), eq).ok()?.remove();
            let (key, value) = self.shift_remove_finish(index);
            Some((index, key, value))
        }
        /// Remove an entry by swapping it with the last
        pub(crate) fn swap_remove_full<Q>(
            &mut self,
            hash: HashValue,
            key: &Q,
        ) -> Option<(usize, K, V)>
        where
            Q: ?Sized + Equivalent<K>,
        {
            let eq = equivalent(key, &self.entries);
            let (index, _) = self.indices.find_entry(hash.get(), eq).ok()?.remove();
            let (key, value) = self.swap_remove_finish(index);
            Some((index, key, value))
        }
        /// Erase `start..end` from `indices`, and shift `end..` indices down to `start..`
        ///
        /// All of these items should still be at their original location in `entries`.
        /// This is used by `drain`, which will let `Vec::drain` do the work on `entries`.
        fn erase_indices(&mut self, start: usize, end: usize) {
            let (init, shifted_entries) = self.entries.split_at(end);
            let (start_entries, erased_entries) = init.split_at(start);
            let erased = erased_entries.len();
            let shifted = shifted_entries.len();
            let half_capacity = self.indices.capacity() / 2;
            if erased == 0 {} else if start + shifted < half_capacity && start < erased {
                self.indices.clear();
                insert_bulk_no_grow(&mut self.indices, start_entries);
                insert_bulk_no_grow(&mut self.indices, shifted_entries);
            } else if erased + shifted < half_capacity {
                for (i, entry) in (start..).zip(erased_entries) {
                    erase_index(&mut self.indices, entry.hash, i);
                }
                for ((new, old), entry) in (start..).zip(end..).zip(shifted_entries) {
                    update_index(&mut self.indices, entry.hash, old, new);
                }
            } else {
                let offset = end - start;
                self.indices
                    .retain(move |i| {
                        if *i >= end {
                            *i -= offset;
                            true
                        } else {
                            *i < start
                        }
                    });
            }
            if true {
                match (&self.indices.len(), &(start + shifted)) {
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
        pub(crate) fn retain_in_order<F>(&mut self, mut keep: F)
        where
            F: FnMut(&mut K, &mut V) -> bool,
        {
            self.entries.retain_mut(|entry| keep(&mut entry.key, &mut entry.value));
            if self.entries.len() < self.indices.len() {
                self.rebuild_hash_table();
            }
        }
        fn rebuild_hash_table(&mut self) {
            self.indices.clear();
            insert_bulk_no_grow(&mut self.indices, &self.entries);
        }
        pub(crate) fn reverse(&mut self) {
            self.entries.reverse();
            let len = self.entries.len();
            for i in &mut self.indices {
                *i = len - *i - 1;
            }
        }
        /// Reserve entries capacity, rounded up to match the indices
        #[inline]
        fn reserve_entries(&mut self, additional: usize) {
            let try_capacity = Ord::min(
                self.indices.capacity(),
                Self::MAX_ENTRIES_CAPACITY,
            );
            let try_add = try_capacity - self.entries.len();
            if try_add > additional && self.entries.try_reserve_exact(try_add).is_ok() {
                return;
            }
            self.entries.reserve_exact(additional);
        }
        /// Insert a key-value pair in `entries`,
        /// *without* checking whether it already exists.
        pub(super) fn insert_unique(
            &mut self,
            hash: HashValue,
            key: K,
            value: V,
        ) -> &mut Bucket<K, V> {
            let i = self.indices.len();
            if true {
                match (&i, &self.entries.len()) {
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
            self.indices.insert_unique(hash.get(), i, get_hash(&self.entries));
            self.push_entry(hash, key, value);
            &mut self.entries[i]
        }
        /// Replaces the key at the given index,
        /// *without* checking whether it already exists.
        #[track_caller]
        pub(crate) fn replace_index_unique(
            &mut self,
            index: usize,
            hash: HashValue,
            key: K,
        ) -> K {
            erase_index(&mut self.indices, self.entries[index].hash, index);
            self.indices.insert_unique(hash.get(), index, get_hash(&self.entries));
            let entry = &mut self.entries[index];
            entry.hash = hash;
            mem::replace(&mut entry.key, key)
        }
        /// Insert a key-value pair in `entries` at a particular index,
        /// *without* checking whether it already exists.
        pub(crate) fn shift_insert_unique(
            &mut self,
            index: usize,
            hash: HashValue,
            key: K,
            value: V,
        ) -> &mut Bucket<K, V> {
            let end = self.indices.len();
            if !(index <= end) {
                ::core::panicking::panic("assertion failed: index <= end")
            }
            self.increment_indices(index, end);
            let entries = &*self.entries;
            self.indices
                .insert_unique(
                    hash.get(),
                    index,
                    move |&i| {
                        if true {
                            match (&i, &index) {
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
                        let i = if i < index { i } else { i - 1 };
                        entries[i].hash.get()
                    },
                );
            if self.entries.len() == self.entries.capacity() {
                self.reserve_entries(1);
            }
            self.entries.insert(index, Bucket { hash, key, value });
            &mut self.entries[index]
        }
        /// Remove an entry by shifting all entries that follow it
        pub(crate) fn shift_remove_index(&mut self, index: usize) -> Option<(K, V)> {
            match self.entries.get(index) {
                Some(entry) => {
                    erase_index(&mut self.indices, entry.hash, index);
                    Some(self.shift_remove_finish(index))
                }
                None => None,
            }
        }
        /// Remove an entry by shifting all entries that follow it
        ///
        /// The index should already be removed from `self.indices`.
        fn shift_remove_finish(&mut self, index: usize) -> (K, V) {
            self.decrement_indices(index + 1, self.entries.len());
            let entry = self.entries.remove(index);
            (entry.key, entry.value)
        }
        /// Remove an entry by swapping it with the last
        pub(crate) fn swap_remove_index(&mut self, index: usize) -> Option<(K, V)> {
            match self.entries.get(index) {
                Some(entry) => {
                    erase_index(&mut self.indices, entry.hash, index);
                    Some(self.swap_remove_finish(index))
                }
                None => None,
            }
        }
        /// Finish removing an entry by swapping it with the last
        ///
        /// The index should already be removed from `self.indices`.
        fn swap_remove_finish(&mut self, index: usize) -> (K, V) {
            let entry = self.entries.swap_remove(index);
            if let Some(entry) = self.entries.get(index) {
                let last = self.entries.len();
                update_index(&mut self.indices, entry.hash, last, index);
            }
            (entry.key, entry.value)
        }
        /// Decrement all indices in the range `start..end`.
        ///
        /// The index `start - 1` should not exist in `self.indices`.
        /// All entries should still be in their original positions.
        fn decrement_indices(&mut self, start: usize, end: usize) {
            let shifted_entries = &self.entries[start..end];
            if shifted_entries.len() > self.indices.capacity() / 2 {
                for i in &mut self.indices {
                    if start <= *i && *i < end {
                        *i -= 1;
                    }
                }
            } else {
                for (i, entry) in (start..end).zip(shifted_entries) {
                    update_index(&mut self.indices, entry.hash, i, i - 1);
                }
            }
        }
        /// Increment all indices in the range `start..end`.
        ///
        /// The index `end` should not exist in `self.indices`.
        /// All entries should still be in their original positions.
        fn increment_indices(&mut self, start: usize, end: usize) {
            let shifted_entries = &self.entries[start..end];
            if shifted_entries.len() > self.indices.capacity() / 2 {
                for i in &mut self.indices {
                    if start <= *i && *i < end {
                        *i += 1;
                    }
                }
            } else {
                for (i, entry) in (start..end).zip(shifted_entries).rev() {
                    update_index(&mut self.indices, entry.hash, i, i + 1);
                }
            }
        }
        #[track_caller]
        pub(super) fn move_index(&mut self, from: usize, to: usize) {
            let from_hash = self.entries[from].hash;
            if from != to {
                let _ = self.entries[to];
                let bucket = self
                    .indices
                    .find_bucket_index(from_hash.get(), move |&i| i == from)
                    .expect("index not found");
                self.move_index_inner(from, to);
                *self.indices.get_bucket_mut(bucket).unwrap() = to;
            }
        }
        fn move_index_inner(&mut self, from: usize, to: usize) {
            if from < to {
                self.decrement_indices(from + 1, to + 1);
                self.entries[from..=to].rotate_left(1);
            } else if to < from {
                self.increment_indices(to, from);
                self.entries[to..=from].rotate_right(1);
            }
        }
        #[track_caller]
        pub(crate) fn swap_indices(&mut self, a: usize, b: usize) {
            if a == b && a < self.entries.len() {
                return;
            }
            match self
                .indices
                .get_disjoint_mut(
                    [self.entries[a].hash.get(), self.entries[b].hash.get()],
                    move |i, &x| if i == 0 { x == a } else { x == b },
                )
            {
                [Some(ref_a), Some(ref_b)] => {
                    mem::swap(ref_a, ref_b);
                    self.entries.swap(a, b);
                }
                _ => {
                    ::core::panicking::panic_fmt(format_args!("indices not found"));
                }
            }
        }
    }
}
#[macro_use]
mod macros {}
mod util {
    use core::ops::{Bound, Range, RangeBounds};
    pub(crate) fn third<A, B, C>(t: (A, B, C)) -> C {
        t.2
    }
    #[track_caller]
    pub(crate) fn simplify_range<R>(range: R, len: usize) -> Range<usize>
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&i) if i <= len => i,
            Bound::Excluded(&i) if i < len => i + 1,
            Bound::Included(i) | Bound::Excluded(i) => {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "range start index {0} out of range for slice of length {1}",
                        i,
                        len,
                    ),
                );
            }
        };
        let end = match range.end_bound() {
            Bound::Unbounded => len,
            Bound::Excluded(&i) if i <= len => i,
            Bound::Included(&i) if i < len => i + 1,
            Bound::Included(i) | Bound::Excluded(i) => {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "range end index {0} out of range for slice of length {1}",
                        i,
                        len,
                    ),
                );
            }
        };
        if start > end {
            {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "range start index {0:?} should be <= range end index {1:?}",
                        range.start_bound(),
                        range.end_bound(),
                    ),
                );
            };
        }
        start..end
    }
    pub(crate) fn try_simplify_range<R>(range: R, len: usize) -> Option<Range<usize>>
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&i) if i <= len => i,
            Bound::Excluded(&i) if i < len => i + 1,
            _ => return None,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => len,
            Bound::Excluded(&i) if i <= len => i,
            Bound::Included(&i) if i < len => i + 1,
            _ => return None,
        };
        if start > end {
            return None;
        }
        Some(start..end)
    }
    pub(crate) fn slice_eq<T, U>(
        left: &[T],
        right: &[U],
        eq: impl Fn(&T, &U) -> bool,
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }
        for i in 0..left.len() {
            if !eq(&left[i], &right[i]) {
                return false;
            }
        }
        true
    }
}
pub mod map {
    //! [`IndexMap`] is a hash table where the iteration order of the key-value
    //! pairs is independent of the hash values of the keys.
    mod entry {
        use crate::inner::{Core, OccupiedEntry, VacantEntry};
        use crate::Bucket;
        use core::{fmt, mem};
        /// Entry for an existing key-value pair in an [`IndexMap`][crate::IndexMap]
        /// or a vacant location to insert one.
        pub enum Entry<'a, K, V> {
            /// Existing slot with equivalent key.
            Occupied(OccupiedEntry<'a, K, V>),
            /// Vacant slot (no equivalent key in the map).
            Vacant(VacantEntry<'a, K, V>),
        }
        impl<'a, K, V> Entry<'a, K, V> {
            /// Return the index where the key-value pair exists or will be inserted.
            pub fn index(&self) -> usize {
                match self {
                    Entry::Occupied(entry) => entry.index(),
                    Entry::Vacant(entry) => entry.index(),
                }
            }
            /// Sets the value of the entry (after inserting if vacant), and returns an `OccupiedEntry`.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
                match self {
                    Entry::Occupied(mut entry) => {
                        entry.insert(value);
                        entry
                    }
                    Entry::Vacant(entry) => entry.insert_entry(value),
                }
            }
            /// Inserts the given default value in the entry if it is vacant and returns a mutable
            /// reference to it. Otherwise a mutable reference to an already existent value is returned.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn or_insert(self, default: V) -> &'a mut V {
                match self {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(default),
                }
            }
            /// Inserts the result of the `call` function in the entry if it is vacant and returns a mutable
            /// reference to it. Otherwise a mutable reference to an already existent value is returned.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn or_insert_with<F>(self, call: F) -> &'a mut V
            where
                F: FnOnce() -> V,
            {
                match self {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(call()),
                }
            }
            /// Inserts the result of the `call` function with a reference to the entry's key if it is
            /// vacant, and returns a mutable reference to the new value. Otherwise a mutable reference to
            /// an already existent value is returned.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn or_insert_with_key<F>(self, call: F) -> &'a mut V
            where
                F: FnOnce(&K) -> V,
            {
                match self {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let value = call(entry.key());
                        entry.insert(value)
                    }
                }
            }
            /// Gets a reference to the entry's key, either within the map if occupied,
            /// or else the new key that was used to find the entry.
            pub fn key(&self) -> &K {
                match *self {
                    Entry::Occupied(ref entry) => entry.key(),
                    Entry::Vacant(ref entry) => entry.key(),
                }
            }
            /// Modifies the entry if it is occupied.
            pub fn and_modify<F>(mut self, f: F) -> Self
            where
                F: FnOnce(&mut V),
            {
                if let Entry::Occupied(entry) = &mut self {
                    f(entry.get_mut());
                }
                self
            }
            /// Inserts a default-constructed value in the entry if it is vacant and returns a mutable
            /// reference to it. Otherwise a mutable reference to an already existent value is returned.
            ///
            /// Computes in **O(1)** time (amortized average).
            pub fn or_default(self) -> &'a mut V
            where
                V: Default,
            {
                match self {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => entry.insert(V::default()),
                }
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut tuple = f.debug_tuple("Entry");
                match self {
                    Entry::Vacant(v) => tuple.field(v),
                    Entry::Occupied(o) => tuple.field(o),
                };
                tuple.finish()
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("OccupiedEntry")
                    .field("key", self.key())
                    .field("value", self.get())
                    .finish()
            }
        }
        impl<K: fmt::Debug, V> fmt::Debug for VacantEntry<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("VacantEntry").field(self.key()).finish()
            }
        }
        /// A view into an occupied entry in an [`IndexMap`][crate::IndexMap] obtained by index.
        ///
        /// This `struct` is created from the [`get_index_entry`][crate::IndexMap::get_index_entry] method.
        pub struct IndexedEntry<'a, K, V> {
            map: &'a mut Core<K, V>,
            index: usize,
        }
        impl<'a, K, V> IndexedEntry<'a, K, V> {
            pub(crate) fn new(map: &'a mut Core<K, V>, index: usize) -> Option<Self> {
                if index < map.len() { Some(Self { map, index }) } else { None }
            }
            /// Return the index of the key-value pair
            #[inline]
            pub fn index(&self) -> usize {
                self.index
            }
            pub(crate) fn into_core(self) -> &'a mut Core<K, V> {
                self.map
            }
            fn get_bucket(&self) -> &Bucket<K, V> {
                &self.map.as_entries()[self.index]
            }
            fn get_bucket_mut(&mut self) -> &mut Bucket<K, V> {
                &mut self.map.as_entries_mut()[self.index]
            }
            fn into_bucket(self) -> &'a mut Bucket<K, V> {
                &mut self.map.as_entries_mut()[self.index]
            }
            /// Gets a reference to the entry's key in the map.
            pub fn key(&self) -> &K {
                &self.get_bucket().key
            }
            pub(super) fn key_mut(&mut self) -> &mut K {
                &mut self.get_bucket_mut().key
            }
            /// Gets a reference to the entry's value in the map.
            pub fn get(&self) -> &V {
                &self.get_bucket().value
            }
            /// Gets a mutable reference to the entry's value in the map.
            ///
            /// If you need a reference which may outlive the destruction of the
            /// `IndexedEntry` value, see [`into_mut`][Self::into_mut].
            pub fn get_mut(&mut self) -> &mut V {
                &mut self.get_bucket_mut().value
            }
            /// Sets the value of the entry to `value`, and returns the entry's old value.
            pub fn insert(&mut self, value: V) -> V {
                mem::replace(self.get_mut(), value)
            }
            /// Converts into a mutable reference to the entry's value in the map,
            /// with a lifetime bound to the map itself.
            pub fn into_mut(self) -> &'a mut V {
                &mut self.into_bucket().value
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove_entry(self) -> (K, V) {
                self.map.swap_remove_index(self.index).unwrap()
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove_entry(self) -> (K, V) {
                self.map.shift_remove_index(self.index).unwrap()
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove(self) -> V {
                self.swap_remove_entry().1
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove(self) -> V {
                self.shift_remove_entry().1
            }
            /// Moves the position of the entry to a new index
            /// by shifting all other entries in-between.
            ///
            /// This is equivalent to [`IndexMap::move_index`][`crate::IndexMap::move_index`]
            /// coming `from` the current [`.index()`][Self::index].
            ///
            /// * If `self.index() < to`, the other pairs will shift down while the targeted pair moves up.
            /// * If `self.index() > to`, the other pairs will shift up while the targeted pair moves down.
            ///
            /// ***Panics*** if `to` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn move_index(self, to: usize) {
                self.map.move_index(self.index, to);
            }
            /// Swaps the position of entry with another.
            ///
            /// This is equivalent to [`IndexMap::swap_indices`][`crate::IndexMap::swap_indices`]
            /// with the current [`.index()`][Self::index] as one of the two being swapped.
            ///
            /// ***Panics*** if the `other` index is out of bounds.
            ///
            /// Computes in **O(1)** time (average).
            #[track_caller]
            pub fn swap_indices(self, other: usize) {
                self.map.swap_indices(self.index, other);
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IndexedEntry<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("IndexedEntry")
                    .field("index", &self.index)
                    .field("key", self.key())
                    .field("value", self.get())
                    .finish()
            }
        }
        impl<'a, K, V> From<OccupiedEntry<'a, K, V>> for IndexedEntry<'a, K, V> {
            fn from(other: OccupiedEntry<'a, K, V>) -> Self {
                Self {
                    index: other.index(),
                    map: other.into_core(),
                }
            }
        }
    }
    mod iter {
        use super::{Bucket, HashValue, IndexMap, Slice};
        use crate::inner::{Core, ExtractCore};
        use alloc::vec::{self, Vec};
        use core::fmt;
        use core::hash::{BuildHasher, Hash};
        use core::iter::FusedIterator;
        use core::mem::MaybeUninit;
        use core::ops::{Index, RangeBounds};
        use core::slice;
        impl<'a, K, V, S> IntoIterator for &'a IndexMap<K, V, S> {
            type Item = (&'a K, &'a V);
            type IntoIter = Iter<'a, K, V>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl<'a, K, V, S> IntoIterator for &'a mut IndexMap<K, V, S> {
            type Item = (&'a K, &'a mut V);
            type IntoIter = IterMut<'a, K, V>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter_mut()
            }
        }
        impl<K, V, S> IntoIterator for IndexMap<K, V, S> {
            type Item = (K, V);
            type IntoIter = IntoIter<K, V>;
            fn into_iter(self) -> Self::IntoIter {
                IntoIter::new(self.into_entries())
            }
        }
        /// An iterator over the entries of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::iter`] method.
        /// See its documentation for more.
        pub struct Iter<'a, K, V> {
            iter: slice::Iter<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> Iter<'a, K, V> {
            pub(super) fn new(entries: &'a [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &'a Slice<K, V> {
                Slice::from_slice(self.iter.as_slice())
            }
        }
        impl<'a, K, V> Iterator for Iter<'a, K, V> {
            type Item = (&'a K, &'a V);
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::refs)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::refs)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::refs).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for Iter<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::refs)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::refs)
            }
        }
        impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for Iter<'_, K, V> {}
        impl<K, V> Clone for Iter<'_, K, V> {
            fn clone(&self) -> Self {
                Iter { iter: self.iter.clone() }
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        impl<K, V> Default for Iter<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter() }
            }
        }
        /// A mutable iterator over the entries of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::iter_mut`] method.
        /// See its documentation for more.
        pub struct IterMut<'a, K, V> {
            iter: slice::IterMut<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> IterMut<'a, K, V> {
            pub(super) fn new(entries: &'a mut [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter_mut() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<K, V> {
                Slice::from_slice(self.iter.as_slice())
            }
            /// Returns a mutable slice of the remaining entries in the iterator.
            ///
            /// To avoid creating `&mut` references that alias, this is forced to consume the iterator.
            pub fn into_slice(self) -> &'a mut Slice<K, V> {
                Slice::from_mut_slice(self.iter.into_slice())
            }
        }
        impl<'a, K, V> Iterator for IterMut<'a, K, V> {
            type Item = (&'a K, &'a mut V);
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::ref_mut)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::ref_mut)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::ref_mut).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for IterMut<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::ref_mut)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::ref_mut)
            }
        }
        impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for IterMut<'_, K, V> {}
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IterMut<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::refs);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for IterMut<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter_mut() }
            }
        }
        /// A mutable iterator over the entries of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`MutableKeys::iter_mut2`][super::MutableKeys::iter_mut2] method.
        /// See its documentation for more.
        pub struct IterMut2<'a, K, V> {
            iter: slice::IterMut<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> IterMut2<'a, K, V> {
            pub(super) fn new(entries: &'a mut [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter_mut() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<K, V> {
                Slice::from_slice(self.iter.as_slice())
            }
            /// Returns a mutable slice of the remaining entries in the iterator.
            ///
            /// To avoid creating `&mut` references that alias, this is forced to consume the iterator.
            pub fn into_slice(self) -> &'a mut Slice<K, V> {
                Slice::from_mut_slice(self.iter.into_slice())
            }
        }
        impl<'a, K, V> Iterator for IterMut2<'a, K, V> {
            type Item = (&'a mut K, &'a mut V);
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::muts)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::muts)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::muts).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for IterMut2<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::muts)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::muts)
            }
        }
        impl<K, V> ExactSizeIterator for IterMut2<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for IterMut2<'_, K, V> {}
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IterMut2<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::refs);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for IterMut2<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter_mut() }
            }
        }
        /// An owning iterator over the entries of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::into_iter`] method
        /// (provided by the [`IntoIterator`] trait). See its documentation for more.
        pub struct IntoIter<K, V> {
            iter: vec::IntoIter<Bucket<K, V>>,
        }
        #[automatically_derived]
        impl<K: ::core::clone::Clone, V: ::core::clone::Clone> ::core::clone::Clone
        for IntoIter<K, V> {
            #[inline]
            fn clone(&self) -> IntoIter<K, V> {
                IntoIter {
                    iter: ::core::clone::Clone::clone(&self.iter),
                }
            }
        }
        impl<K, V> IntoIter<K, V> {
            pub(super) fn new(entries: Vec<Bucket<K, V>>) -> Self {
                Self { iter: entries.into_iter() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<K, V> {
                Slice::from_slice(self.iter.as_slice())
            }
            /// Returns a mutable slice of the remaining entries in the iterator.
            pub fn as_mut_slice(&mut self) -> &mut Slice<K, V> {
                Slice::from_mut_slice(self.iter.as_mut_slice())
            }
        }
        impl<K, V> Iterator for IntoIter<K, V> {
            type Item = (K, V);
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key_value)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key_value)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key_value).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key_value)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key_value)
            }
        }
        impl<K, V> ExactSizeIterator for IntoIter<K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for IntoIter<K, V> {}
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IntoIter<K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::refs);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for IntoIter<K, V> {
            fn default() -> Self {
                Self {
                    iter: Vec::new().into_iter(),
                }
            }
        }
        /// A draining iterator over the entries of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::drain`] method.
        /// See its documentation for more.
        pub struct Drain<'a, K, V> {
            iter: vec::Drain<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> Drain<'a, K, V> {
            pub(super) fn new(iter: vec::Drain<'a, Bucket<K, V>>) -> Self {
                Self { iter }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<K, V> {
                Slice::from_slice(self.iter.as_slice())
            }
        }
        impl<K, V> Iterator for Drain<'_, K, V> {
            type Item = (K, V);
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key_value)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key_value)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key_value).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for Drain<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key_value)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key_value)
            }
        }
        impl<K, V> ExactSizeIterator for Drain<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for Drain<'_, K, V> {}
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Drain<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::refs);
                f.debug_list().entries(iter).finish()
            }
        }
        /// An iterator over the keys of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::keys`] method.
        /// See its documentation for more.
        pub struct Keys<'a, K, V> {
            iter: slice::Iter<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> Keys<'a, K, V> {
            pub(super) fn new(entries: &'a [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter() }
            }
        }
        impl<'a, K, V> Iterator for Keys<'a, K, V> {
            type Item = &'a K;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key_ref)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key_ref)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key_ref).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for Keys<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key_ref)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key_ref)
            }
        }
        impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for Keys<'_, K, V> {}
        impl<K, V> Clone for Keys<'_, K, V> {
            fn clone(&self) -> Self {
                Keys { iter: self.iter.clone() }
            }
        }
        impl<K: fmt::Debug, V> fmt::Debug for Keys<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        impl<K, V> Default for Keys<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter() }
            }
        }
        /// Access [`IndexMap`] keys at indexed positions.
        ///
        /// While [`Index<usize> for IndexMap`][values] accesses a map's values,
        /// indexing through [`IndexMap::keys`] offers an alternative to access a map's
        /// keys instead.
        ///
        /// [values]: IndexMap#impl-Index<usize>-for-IndexMap<K,+V,+S>
        ///
        /// Since `Keys` is also an iterator, consuming items from the iterator will
        /// offset the effective indices. Similarly, if `Keys` is obtained from
        /// [`Slice::keys`], indices will be interpreted relative to the position of
        /// that slice.
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// let mut map = IndexMap::new();
        /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
        ///     map.insert(word.to_lowercase(), word.to_uppercase());
        /// }
        ///
        /// assert_eq!(map[0], "LOREM");
        /// assert_eq!(map.keys()[0], "lorem");
        /// assert_eq!(map[1], "IPSUM");
        /// assert_eq!(map.keys()[1], "ipsum");
        ///
        /// map.reverse();
        /// assert_eq!(map.keys()[0], "amet");
        /// assert_eq!(map.keys()[1], "sit");
        ///
        /// map.sort_keys();
        /// assert_eq!(map.keys()[0], "amet");
        /// assert_eq!(map.keys()[1], "dolor");
        ///
        /// // Advancing the iterator will offset the indexing
        /// let mut keys = map.keys();
        /// assert_eq!(keys[0], "amet");
        /// assert_eq!(keys.next().map(|s| &**s), Some("amet"));
        /// assert_eq!(keys[0], "dolor");
        /// assert_eq!(keys[1], "ipsum");
        ///
        /// // Slices may have an offset as well
        /// let slice = &map[2..];
        /// assert_eq!(slice[0], "IPSUM");
        /// assert_eq!(slice.keys()[0], "ipsum");
        /// ```
        ///
        /// ```should_panic
        /// use indexmap::IndexMap;
        ///
        /// let mut map = IndexMap::new();
        /// map.insert("foo", 1);
        /// println!("{:?}", map.keys()[10]); // panics!
        /// ```
        impl<K, V> Index<usize> for Keys<'_, K, V> {
            type Output = K;
            /// Returns a reference to the key at the supplied `index`.
            ///
            /// ***Panics*** if `index` is out of bounds.
            fn index(&self, index: usize) -> &K {
                &self.iter.as_slice()[index].key
            }
        }
        /// An owning iterator over the keys of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::into_keys`] method.
        /// See its documentation for more.
        pub struct IntoKeys<K, V> {
            iter: vec::IntoIter<Bucket<K, MaybeUninit<V>>>,
        }
        impl<K, V> IntoKeys<K, V> {
            pub(super) fn new(entries: Vec<Bucket<K, V>>) -> Self {
                let entries = entries
                    .into_iter()
                    .map(|Bucket { hash, key, .. }| Bucket {
                        hash,
                        key,
                        value: MaybeUninit::uninit(),
                    })
                    .collect::<Vec<_>>();
                Self { iter: entries.into_iter() }
            }
        }
        impl<K: Clone, V> Clone for IntoKeys<K, V> {
            fn clone(&self) -> Self {
                let entries = self
                    .iter
                    .as_slice()
                    .iter()
                    .map(|Bucket { key, .. }| Bucket {
                        hash: HashValue(0),
                        key: key.clone(),
                        value: MaybeUninit::uninit(),
                    })
                    .collect::<Vec<_>>();
                Self { iter: entries.into_iter() }
            }
        }
        impl<K, V> Iterator for IntoKeys<K, V> {
            type Item = K;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for IntoKeys<K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key)
            }
        }
        impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for IntoKeys<K, V> {}
        impl<K: fmt::Debug, V> fmt::Debug for IntoKeys<K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::key_ref);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for IntoKeys<K, V> {
            fn default() -> Self {
                Self {
                    iter: Vec::new().into_iter(),
                }
            }
        }
        /// An iterator over the values of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::values`] method.
        /// See its documentation for more.
        pub struct Values<'a, K, V> {
            iter: slice::Iter<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> Values<'a, K, V> {
            pub(super) fn new(entries: &'a [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter() }
            }
        }
        impl<'a, K, V> Iterator for Values<'a, K, V> {
            type Item = &'a V;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::value_ref)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::value_ref)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::value_ref).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for Values<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::value_ref)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::value_ref)
            }
        }
        impl<K, V> ExactSizeIterator for Values<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for Values<'_, K, V> {}
        impl<K, V> Clone for Values<'_, K, V> {
            fn clone(&self) -> Self {
                Values { iter: self.iter.clone() }
            }
        }
        impl<K, V: fmt::Debug> fmt::Debug for Values<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        impl<K, V> Default for Values<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter() }
            }
        }
        /// A mutable iterator over the values of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::values_mut`] method.
        /// See its documentation for more.
        pub struct ValuesMut<'a, K, V> {
            iter: slice::IterMut<'a, Bucket<K, V>>,
        }
        impl<'a, K, V> ValuesMut<'a, K, V> {
            pub(super) fn new(entries: &'a mut [Bucket<K, V>]) -> Self {
                Self { iter: entries.iter_mut() }
            }
        }
        impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
            type Item = &'a mut V;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::value_mut)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::value_mut)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::value_mut).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for ValuesMut<'_, K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::value_mut)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::value_mut)
            }
        }
        impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}
        impl<K, V: fmt::Debug> fmt::Debug for ValuesMut<'_, K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::value_ref);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for ValuesMut<'_, K, V> {
            fn default() -> Self {
                Self { iter: [].iter_mut() }
            }
        }
        /// An owning iterator over the values of an [`IndexMap`].
        ///
        /// This `struct` is created by the [`IndexMap::into_values`] method.
        /// See its documentation for more.
        pub struct IntoValues<K, V> {
            iter: vec::IntoIter<Bucket<MaybeUninit<K>, V>>,
        }
        impl<K, V> IntoValues<K, V> {
            pub(super) fn new(entries: Vec<Bucket<K, V>>) -> Self {
                let entries = entries
                    .into_iter()
                    .map(|Bucket { hash, value, .. }| Bucket {
                        hash,
                        key: MaybeUninit::uninit(),
                        value,
                    })
                    .collect::<Vec<_>>();
                Self { iter: entries.into_iter() }
            }
        }
        impl<K, V: Clone> Clone for IntoValues<K, V> {
            fn clone(&self) -> Self {
                let entries = self
                    .iter
                    .as_slice()
                    .iter()
                    .map(|Bucket { value, .. }| Bucket {
                        hash: HashValue(0),
                        key: MaybeUninit::uninit(),
                        value: value.clone(),
                    })
                    .collect::<Vec<_>>();
                Self { iter: entries.into_iter() }
            }
        }
        impl<K, V> Iterator for IntoValues<K, V> {
            type Item = V;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::value)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::value)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::value).collect()
            }
        }
        impl<K, V> DoubleEndedIterator for IntoValues<K, V> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::value)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::value)
            }
        }
        impl<K, V> ExactSizeIterator for IntoValues<K, V> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<K, V> FusedIterator for IntoValues<K, V> {}
        impl<K, V: fmt::Debug> fmt::Debug for IntoValues<K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::value_ref);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<K, V> Default for IntoValues<K, V> {
            fn default() -> Self {
                Self {
                    iter: Vec::new().into_iter(),
                }
            }
        }
        /// A splicing iterator for `IndexMap`.
        ///
        /// This `struct` is created by [`IndexMap::splice()`].
        /// See its documentation for more.
        pub struct Splice<'a, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            map: &'a mut IndexMap<K, V, S>,
            tail: Core<K, V>,
            drain: vec::IntoIter<Bucket<K, V>>,
            replace_with: I,
        }
        impl<'a, I, K, V, S> Splice<'a, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            #[track_caller]
            pub(super) fn new<R>(
                map: &'a mut IndexMap<K, V, S>,
                range: R,
                replace_with: I,
            ) -> Self
            where
                R: RangeBounds<usize>,
            {
                let (tail, drain) = map.core.split_splice(range);
                Self {
                    map,
                    tail,
                    drain,
                    replace_with,
                }
            }
        }
        impl<I, K, V, S> Drop for Splice<'_, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            fn drop(&mut self) {
                let _ = self.drain.nth(usize::MAX);
                while let Some((key, value)) = self.replace_with.next() {
                    let hash = self.map.hash(&key);
                    if let Some(i) = self.tail.get_index_of(hash, &key) {
                        self.tail.as_entries_mut()[i].value = value;
                    } else {
                        self.map.core.insert_full(hash, key, value);
                    }
                }
                self.map.core.append_unchecked(&mut self.tail);
            }
        }
        impl<I, K, V, S> Iterator for Splice<'_, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            type Item = (K, V);
            fn next(&mut self) -> Option<Self::Item> {
                self.drain.next().map(Bucket::key_value)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.drain.size_hint()
            }
        }
        impl<I, K, V, S> DoubleEndedIterator for Splice<'_, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.drain.next_back().map(Bucket::key_value)
            }
        }
        impl<I, K, V, S> ExactSizeIterator for Splice<'_, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {
            fn len(&self) -> usize {
                self.drain.len()
            }
        }
        impl<I, K, V, S> FusedIterator for Splice<'_, I, K, V, S>
        where
            I: Iterator<Item = (K, V)>,
            K: Hash + Eq,
            S: BuildHasher,
        {}
        impl<I, K, V, S> fmt::Debug for Splice<'_, I, K, V, S>
        where
            I: fmt::Debug + Iterator<Item = (K, V)>,
            K: fmt::Debug + Hash + Eq,
            V: fmt::Debug,
            S: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("Splice")
                    .field("drain", &self.drain)
                    .field("replace_with", &self.replace_with)
                    .finish()
            }
        }
        /// An extracting iterator for `IndexMap`.
        ///
        /// This `struct` is created by [`IndexMap::extract_if()`].
        /// See its documentation for more.
        pub struct ExtractIf<'a, K, V, F> {
            inner: ExtractCore<'a, K, V>,
            pred: F,
        }
        impl<K, V, F> ExtractIf<'_, K, V, F> {
            #[track_caller]
            pub(super) fn new<R>(
                core: &mut Core<K, V>,
                range: R,
                pred: F,
            ) -> ExtractIf<'_, K, V, F>
            where
                R: RangeBounds<usize>,
                F: FnMut(&K, &mut V) -> bool,
            {
                ExtractIf {
                    inner: core.extract(range),
                    pred,
                }
            }
        }
        impl<K, V, F> Iterator for ExtractIf<'_, K, V, F>
        where
            F: FnMut(&K, &mut V) -> bool,
        {
            type Item = (K, V);
            fn next(&mut self) -> Option<Self::Item> {
                self.inner
                    .extract_if(|bucket| {
                        let (key, value) = bucket.ref_mut();
                        (self.pred)(key, value)
                    })
                    .map(Bucket::key_value)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (0, Some(self.inner.remaining()))
            }
        }
        impl<K, V, F> FusedIterator for ExtractIf<'_, K, V, F>
        where
            F: FnMut(&K, &mut V) -> bool,
        {}
        impl<K, V, F> fmt::Debug for ExtractIf<'_, K, V, F>
        where
            K: fmt::Debug,
            V: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("ExtractIf").finish_non_exhaustive()
            }
        }
    }
    mod mutable {
        use core::hash::{BuildHasher, Hash};
        use super::{
            Bucket, Entry, Equivalent, IndexMap, IndexedEntry, IterMut2, OccupiedEntry,
            VacantEntry,
        };
        /// Opt-in mutable access to [`IndexMap`] keys.
        ///
        /// These methods expose `&mut K`, mutable references to the key as it is stored
        /// in the map.
        /// You are allowed to modify the keys in the map **if the modification
        /// does not change the key's hash and equality**.
        ///
        /// If keys are modified erroneously, you can no longer look them up.
        /// This is sound (memory safe) but a logical error hazard (just like
        /// implementing `PartialEq`, `Eq`, or `Hash` incorrectly would be).
        ///
        /// `use` this trait to enable its methods for `IndexMap`.
        ///
        /// This trait is sealed and cannot be implemented for types outside this crate.
        #[expect(private_bounds)]
        pub trait MutableKeys: Sealed {
            type Key;
            type Value;
            /// Return item index, mutable reference to key and value
            ///
            /// Computes in **O(1)** time (average).
            fn get_full_mut2<Q>(
                &mut self,
                key: &Q,
            ) -> Option<(usize, &mut Self::Key, &mut Self::Value)>
            where
                Q: ?Sized + Hash + Equivalent<Self::Key>;
            /// Return mutable reference to key and value at an index.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            ///
            /// Computes in **O(1)** time.
            fn get_index_mut2(
                &mut self,
                index: usize,
            ) -> Option<(&mut Self::Key, &mut Self::Value)>;
            /// Return an iterator over the key-value pairs of the map, in their order
            fn iter_mut2(&mut self) -> IterMut2<'_, Self::Key, Self::Value>;
            /// Scan through each key-value pair in the map and keep those where the
            /// closure `keep` returns `true`.
            ///
            /// The elements are visited in order, and remaining elements keep their
            /// order.
            ///
            /// Computes in **O(n)** time (average).
            fn retain2<F>(&mut self, keep: F)
            where
                F: FnMut(&mut Self::Key, &mut Self::Value) -> bool;
        }
        /// Opt-in mutable access to [`IndexMap`] keys.
        ///
        /// See [`MutableKeys`] for more information.
        impl<K, V, S> MutableKeys for IndexMap<K, V, S>
        where
            S: BuildHasher,
        {
            type Key = K;
            type Value = V;
            fn get_full_mut2<Q>(&mut self, key: &Q) -> Option<(usize, &mut K, &mut V)>
            where
                Q: ?Sized + Hash + Equivalent<K>,
            {
                if let Some(i) = self.get_index_of(key) {
                    let entry = &mut self.as_entries_mut()[i];
                    Some((i, &mut entry.key, &mut entry.value))
                } else {
                    None
                }
            }
            fn get_index_mut2(&mut self, index: usize) -> Option<(&mut K, &mut V)> {
                self.as_entries_mut().get_mut(index).map(Bucket::muts)
            }
            fn iter_mut2(&mut self) -> IterMut2<'_, Self::Key, Self::Value> {
                IterMut2::new(self.as_entries_mut())
            }
            fn retain2<F>(&mut self, keep: F)
            where
                F: FnMut(&mut K, &mut V) -> bool,
            {
                self.core.retain_in_order(keep);
            }
        }
        /// Opt-in mutable access to [`Entry`] keys.
        ///
        /// These methods expose `&mut K`, mutable references to the key as it is stored
        /// in the map.
        /// You are allowed to modify the keys in the map **if the modification
        /// does not change the key's hash and equality**.
        ///
        /// If keys are modified erroneously, you can no longer look them up.
        /// This is sound (memory safe) but a logical error hazard (just like
        /// implementing `PartialEq`, `Eq`, or `Hash` incorrectly would be).
        ///
        /// `use` this trait to enable its methods for `Entry`.
        ///
        /// This trait is sealed and cannot be implemented for types outside this crate.
        #[expect(private_bounds)]
        pub trait MutableEntryKey: Sealed {
            type Key;
            /// Gets a mutable reference to the entry's key, either within the map if occupied,
            /// or else the new key that was used to find the entry.
            fn key_mut(&mut self) -> &mut Self::Key;
        }
        /// Opt-in mutable access to [`Entry`] keys.
        ///
        /// See [`MutableEntryKey`] for more information.
        impl<K, V> MutableEntryKey for Entry<'_, K, V> {
            type Key = K;
            fn key_mut(&mut self) -> &mut Self::Key {
                match self {
                    Entry::Occupied(e) => e.key_mut(),
                    Entry::Vacant(e) => e.key_mut(),
                }
            }
        }
        /// Opt-in mutable access to [`OccupiedEntry`] keys.
        ///
        /// See [`MutableEntryKey`] for more information.
        impl<K, V> MutableEntryKey for OccupiedEntry<'_, K, V> {
            type Key = K;
            fn key_mut(&mut self) -> &mut Self::Key {
                &mut self.get_bucket_mut().key
            }
        }
        /// Opt-in mutable access to [`VacantEntry`] keys.
        ///
        /// See [`MutableEntryKey`] for more information.
        impl<K, V> MutableEntryKey for VacantEntry<'_, K, V> {
            type Key = K;
            fn key_mut(&mut self) -> &mut Self::Key {
                self.key_mut()
            }
        }
        /// Opt-in mutable access to [`IndexedEntry`] keys.
        ///
        /// See [`MutableEntryKey`] for more information.
        impl<K, V> MutableEntryKey for IndexedEntry<'_, K, V> {
            type Key = K;
            fn key_mut(&mut self) -> &mut Self::Key {
                self.key_mut()
            }
        }
        trait Sealed {}
        impl<K, V, S> Sealed for IndexMap<K, V, S> {}
        impl<K, V> Sealed for Entry<'_, K, V> {}
        impl<K, V> Sealed for OccupiedEntry<'_, K, V> {}
        impl<K, V> Sealed for VacantEntry<'_, K, V> {}
        impl<K, V> Sealed for IndexedEntry<'_, K, V> {}
    }
    mod slice {
        use super::{
            Bucket, IndexMap, IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys,
            Values, ValuesMut,
        };
        use crate::util::{slice_eq, try_simplify_range};
        use crate::GetDisjointMutError;
        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use core::cmp::Ordering;
        use core::fmt;
        use core::hash::{Hash, Hasher};
        use core::ops::{self, Bound, Index, IndexMut, RangeBounds};
        /// A dynamically-sized slice of key-value pairs in an [`IndexMap`].
        ///
        /// This supports indexed operations much like a `[(K, V)]` slice,
        /// but not any hashed operations on the map keys.
        ///
        /// Unlike `IndexMap`, `Slice` does consider the order for [`PartialEq`]
        /// and [`Eq`], and it also implements [`PartialOrd`], [`Ord`], and [`Hash`].
        #[repr(transparent)]
        pub struct Slice<K, V> {
            pub(crate) entries: [Bucket<K, V>],
        }
        #[allow(unsafe_code)]
        impl<K, V> Slice<K, V> {
            pub(crate) const fn from_slice(entries: &[Bucket<K, V>]) -> &Self {
                unsafe { &*(entries as *const [Bucket<K, V>] as *const Self) }
            }
            pub(super) fn from_mut_slice(entries: &mut [Bucket<K, V>]) -> &mut Self {
                unsafe { &mut *(entries as *mut [Bucket<K, V>] as *mut Self) }
            }
            pub(super) fn from_boxed(entries: Box<[Bucket<K, V>]>) -> Box<Self> {
                unsafe { Box::from_raw(Box::into_raw(entries) as *mut Self) }
            }
            fn into_boxed(self: Box<Self>) -> Box<[Bucket<K, V>]> {
                unsafe { Box::from_raw(Box::into_raw(self) as *mut [Bucket<K, V>]) }
            }
        }
        impl<K, V> Slice<K, V> {
            pub(crate) fn into_entries(self: Box<Self>) -> Vec<Bucket<K, V>> {
                self.into_boxed().into_vec()
            }
            /// Returns an empty slice.
            pub const fn new<'a>() -> &'a Self {
                Self::from_slice(&[])
            }
            /// Returns an empty mutable slice.
            pub fn new_mut<'a>() -> &'a mut Self {
                Self::from_mut_slice(&mut [])
            }
            /// Return the number of key-value pairs in the map slice.
            #[inline]
            pub const fn len(&self) -> usize {
                self.entries.len()
            }
            /// Returns true if the map slice contains no elements.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
            /// Get a key-value pair by index.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
                self.entries.get(index).map(Bucket::refs)
            }
            /// Get a key-value pair by index, with mutable access to the value.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
                self.entries.get_mut(index).map(Bucket::ref_mut)
            }
            /// Returns a slice of key-value pairs in the given range of indices.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Self> {
                let range = try_simplify_range(range, self.entries.len())?;
                self.entries.get(range).map(Slice::from_slice)
            }
            /// Returns a mutable slice of key-value pairs in the given range of indices.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_range_mut<R: RangeBounds<usize>>(
                &mut self,
                range: R,
            ) -> Option<&mut Self> {
                let range = try_simplify_range(range, self.entries.len())?;
                self.entries.get_mut(range).map(Slice::from_mut_slice)
            }
            /// Get the first key-value pair.
            pub fn first(&self) -> Option<(&K, &V)> {
                self.entries.first().map(Bucket::refs)
            }
            /// Get the first key-value pair, with mutable access to the value.
            pub fn first_mut(&mut self) -> Option<(&K, &mut V)> {
                self.entries.first_mut().map(Bucket::ref_mut)
            }
            /// Get the last key-value pair.
            pub fn last(&self) -> Option<(&K, &V)> {
                self.entries.last().map(Bucket::refs)
            }
            /// Get the last key-value pair, with mutable access to the value.
            pub fn last_mut(&mut self) -> Option<(&K, &mut V)> {
                self.entries.last_mut().map(Bucket::ref_mut)
            }
            /// Divides one slice into two at an index.
            ///
            /// ***Panics*** if `index > len`.
            /// For a non-panicking alternative see [`split_at_checked`][Self::split_at_checked].
            #[track_caller]
            pub fn split_at(&self, index: usize) -> (&Self, &Self) {
                let (first, second) = self.entries.split_at(index);
                (Self::from_slice(first), Self::from_slice(second))
            }
            /// Divides one mutable slice into two at an index.
            ///
            /// ***Panics*** if `index > len`.
            /// For a non-panicking alternative see [`split_at_mut_checked`][Self::split_at_mut_checked].
            #[track_caller]
            pub fn split_at_mut(&mut self, index: usize) -> (&mut Self, &mut Self) {
                let (first, second) = self.entries.split_at_mut(index);
                (Self::from_mut_slice(first), Self::from_mut_slice(second))
            }
            /// Divides one slice into two at an index.
            ///
            /// Returns `None` if `index > len`.
            pub fn split_at_checked(&self, index: usize) -> Option<(&Self, &Self)> {
                let (first, second) = self.entries.split_at_checked(index)?;
                Some((Self::from_slice(first), Self::from_slice(second)))
            }
            /// Divides one mutable slice into two at an index.
            ///
            /// Returns `None` if `index > len`.
            pub fn split_at_mut_checked(
                &mut self,
                index: usize,
            ) -> Option<(&mut Self, &mut Self)> {
                let (first, second) = self.entries.split_at_mut_checked(index)?;
                Some((Self::from_mut_slice(first), Self::from_mut_slice(second)))
            }
            /// Returns the first key-value pair and the rest of the slice,
            /// or `None` if it is empty.
            pub fn split_first(&self) -> Option<((&K, &V), &Self)> {
                if let [first, rest @ ..] = &self.entries {
                    Some((first.refs(), Self::from_slice(rest)))
                } else {
                    None
                }
            }
            /// Returns the first key-value pair and the rest of the slice,
            /// with mutable access to the value, or `None` if it is empty.
            pub fn split_first_mut(&mut self) -> Option<((&K, &mut V), &mut Self)> {
                if let [first, rest @ ..] = &mut self.entries {
                    Some((first.ref_mut(), Self::from_mut_slice(rest)))
                } else {
                    None
                }
            }
            /// Returns the last key-value pair and the rest of the slice,
            /// or `None` if it is empty.
            pub fn split_last(&self) -> Option<((&K, &V), &Self)> {
                if let [rest @ .., last] = &self.entries {
                    Some((last.refs(), Self::from_slice(rest)))
                } else {
                    None
                }
            }
            /// Returns the last key-value pair and the rest of the slice,
            /// with mutable access to the value, or `None` if it is empty.
            pub fn split_last_mut(&mut self) -> Option<((&K, &mut V), &mut Self)> {
                if let [rest @ .., last] = &mut self.entries {
                    Some((last.ref_mut(), Self::from_mut_slice(rest)))
                } else {
                    None
                }
            }
            /// Return an iterator over the key-value pairs of the map slice.
            pub fn iter(&self) -> Iter<'_, K, V> {
                Iter::new(&self.entries)
            }
            /// Return an iterator over the key-value pairs of the map slice.
            pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
                IterMut::new(&mut self.entries)
            }
            /// Return an iterator over the keys of the map slice.
            pub fn keys(&self) -> Keys<'_, K, V> {
                Keys::new(&self.entries)
            }
            /// Return an owning iterator over the keys of the map slice.
            pub fn into_keys(self: Box<Self>) -> IntoKeys<K, V> {
                IntoKeys::new(self.into_entries())
            }
            /// Return an iterator over the values of the map slice.
            pub fn values(&self) -> Values<'_, K, V> {
                Values::new(&self.entries)
            }
            /// Return an iterator over mutable references to the the values of the map slice.
            pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
                ValuesMut::new(&mut self.entries)
            }
            /// Return an owning iterator over the values of the map slice.
            pub fn into_values(self: Box<Self>) -> IntoValues<K, V> {
                IntoValues::new(self.into_entries())
            }
            /// Search over a sorted map for a key.
            ///
            /// Returns the position where that key is present, or the position where it can be inserted to
            /// maintain the sort. See [`slice::binary_search`] for more details.
            ///
            /// Computes in **O(log(n))** time, which is notably less scalable than looking the key up in
            /// the map this is a slice from using [`IndexMap::get_index_of`], but this can also position
            /// missing keys.
            pub fn binary_search_keys(&self, x: &K) -> Result<usize, usize>
            where
                K: Ord,
            {
                self.binary_search_by(|p, _| p.cmp(x))
            }
            /// Search over a sorted map with a comparator function.
            ///
            /// Returns the position where that value is present, or the position where it can be inserted
            /// to maintain the sort. See [`slice::binary_search_by`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[inline]
            pub fn binary_search_by<'a, F>(&'a self, mut f: F) -> Result<usize, usize>
            where
                F: FnMut(&'a K, &'a V) -> Ordering,
            {
                self.entries.binary_search_by(move |a| f(&a.key, &a.value))
            }
            /// Search over a sorted map with an extraction function.
            ///
            /// Returns the position where that value is present, or the position where it can be inserted
            /// to maintain the sort. See [`slice::binary_search_by_key`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[inline]
            pub fn binary_search_by_key<'a, B, F>(
                &'a self,
                b: &B,
                mut f: F,
            ) -> Result<usize, usize>
            where
                F: FnMut(&'a K, &'a V) -> B,
                B: Ord,
            {
                self.binary_search_by(|k, v| f(k, v).cmp(b))
            }
            /// Checks if the keys of this slice are sorted.
            #[inline]
            pub fn is_sorted(&self) -> bool
            where
                K: PartialOrd,
            {
                self.entries.is_sorted_by(|a, b| a.key <= b.key)
            }
            /// Checks if this slice is sorted using the given comparator function.
            #[inline]
            pub fn is_sorted_by<'a, F>(&'a self, mut cmp: F) -> bool
            where
                F: FnMut(&'a K, &'a V, &'a K, &'a V) -> bool,
            {
                self.entries
                    .is_sorted_by(move |a, b| cmp(&a.key, &a.value, &b.key, &b.value))
            }
            /// Checks if this slice is sorted using the given sort-key function.
            #[inline]
            pub fn is_sorted_by_key<'a, F, T>(&'a self, mut sort_key: F) -> bool
            where
                F: FnMut(&'a K, &'a V) -> T,
                T: PartialOrd,
            {
                self.entries.is_sorted_by_key(move |a| sort_key(&a.key, &a.value))
            }
            /// Returns the index of the partition point of a sorted map according to the given predicate
            /// (the index of the first element of the second partition).
            ///
            /// See [`slice::partition_point`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[must_use]
            pub fn partition_point<P>(&self, mut pred: P) -> usize
            where
                P: FnMut(&K, &V) -> bool,
            {
                self.entries.partition_point(move |a| pred(&a.key, &a.value))
            }
            /// Get an array of `N` key-value pairs by `N` indices
            ///
            /// Valid indices are *0 <= index < self.len()* and each index needs to be unique.
            pub fn get_disjoint_mut<const N: usize>(
                &mut self,
                indices: [usize; N],
            ) -> Result<[(&K, &mut V); N], GetDisjointMutError> {
                let indices = indices.map(Some);
                let key_values = self.get_disjoint_opt_mut(indices)?;
                Ok(key_values.map(Option::unwrap))
            }
            #[allow(unsafe_code)]
            pub(crate) fn get_disjoint_opt_mut<const N: usize>(
                &mut self,
                indices: [Option<usize>; N],
            ) -> Result<[Option<(&K, &mut V)>; N], GetDisjointMutError> {
                let len = self.len();
                for i in 0..N {
                    if let Some(idx) = indices[i] {
                        if idx >= len {
                            return Err(GetDisjointMutError::IndexOutOfBounds);
                        } else if indices[..i].contains(&Some(idx)) {
                            return Err(GetDisjointMutError::OverlappingIndices);
                        }
                    }
                }
                let entries_ptr = self.entries.as_mut_ptr();
                let out = indices
                    .map(|idx_opt| {
                        match idx_opt {
                            Some(idx) => {
                                let kv = unsafe { (*(entries_ptr.add(idx))).ref_mut() };
                                Some(kv)
                            }
                            None => None,
                        }
                    });
                Ok(out)
            }
        }
        impl<'a, K, V> IntoIterator for &'a Slice<K, V> {
            type IntoIter = Iter<'a, K, V>;
            type Item = (&'a K, &'a V);
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl<'a, K, V> IntoIterator for &'a mut Slice<K, V> {
            type IntoIter = IterMut<'a, K, V>;
            type Item = (&'a K, &'a mut V);
            fn into_iter(self) -> Self::IntoIter {
                self.iter_mut()
            }
        }
        impl<K, V> IntoIterator for Box<Slice<K, V>> {
            type IntoIter = IntoIter<K, V>;
            type Item = (K, V);
            fn into_iter(self) -> Self::IntoIter {
                IntoIter::new(self.into_entries())
            }
        }
        impl<K, V> Default for &'_ Slice<K, V> {
            fn default() -> Self {
                Slice::from_slice(&[])
            }
        }
        impl<K, V> Default for &'_ mut Slice<K, V> {
            fn default() -> Self {
                Slice::from_mut_slice(&mut [])
            }
        }
        impl<K, V> Default for Box<Slice<K, V>> {
            fn default() -> Self {
                Slice::from_boxed(Box::default())
            }
        }
        impl<K: Clone, V: Clone> Clone for Box<Slice<K, V>> {
            fn clone(&self) -> Self {
                Slice::from_boxed(self.entries.to_vec().into_boxed_slice())
            }
        }
        impl<K: Copy, V: Copy> From<&Slice<K, V>> for Box<Slice<K, V>> {
            fn from(slice: &Slice<K, V>) -> Self {
                Slice::from_boxed(Box::from(&slice.entries))
            }
        }
        impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Slice<K, V> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self).finish()
            }
        }
        impl<K, V, K2, V2> PartialEq<Slice<K2, V2>> for Slice<K, V>
        where
            K: PartialEq<K2>,
            V: PartialEq<V2>,
        {
            fn eq(&self, other: &Slice<K2, V2>) -> bool {
                slice_eq(
                    &self.entries,
                    &other.entries,
                    |b1, b2| { b1.key == b2.key && b1.value == b2.value },
                )
            }
        }
        impl<K, V, K2, V2> PartialEq<[(K2, V2)]> for Slice<K, V>
        where
            K: PartialEq<K2>,
            V: PartialEq<V2>,
        {
            fn eq(&self, other: &[(K2, V2)]) -> bool {
                slice_eq(&self.entries, other, |b, t| b.key == t.0 && b.value == t.1)
            }
        }
        impl<K, V, K2, V2> PartialEq<Slice<K2, V2>> for [(K, V)]
        where
            K: PartialEq<K2>,
            V: PartialEq<V2>,
        {
            fn eq(&self, other: &Slice<K2, V2>) -> bool {
                slice_eq(self, &other.entries, |t, b| t.0 == b.key && t.1 == b.value)
            }
        }
        impl<K, V, K2, V2, const N: usize> PartialEq<[(K2, V2); N]> for Slice<K, V>
        where
            K: PartialEq<K2>,
            V: PartialEq<V2>,
        {
            fn eq(&self, other: &[(K2, V2); N]) -> bool {
                <Self as PartialEq<[_]>>::eq(self, other)
            }
        }
        impl<K, V, const N: usize, K2, V2> PartialEq<Slice<K2, V2>> for [(K, V); N]
        where
            K: PartialEq<K2>,
            V: PartialEq<V2>,
        {
            fn eq(&self, other: &Slice<K2, V2>) -> bool {
                <[_] as PartialEq<_>>::eq(self, other)
            }
        }
        impl<K: Eq, V: Eq> Eq for Slice<K, V> {}
        impl<K: PartialOrd, V: PartialOrd> PartialOrd for Slice<K, V> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.iter().partial_cmp(other)
            }
        }
        impl<K: Ord, V: Ord> Ord for Slice<K, V> {
            fn cmp(&self, other: &Self) -> Ordering {
                self.iter().cmp(other)
            }
        }
        impl<K: Hash, V: Hash> Hash for Slice<K, V> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.len().hash(state);
                for (key, value) in self {
                    key.hash(state);
                    value.hash(state);
                }
            }
        }
        impl<K, V> Index<usize> for Slice<K, V> {
            type Output = V;
            fn index(&self, index: usize) -> &V {
                &self.entries[index].value
            }
        }
        impl<K, V> IndexMut<usize> for Slice<K, V> {
            fn index_mut(&mut self, index: usize) -> &mut V {
                &mut self.entries[index].value
            }
        }
        impl<K, V, S> Index<ops::Range<usize>> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::Range<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::Range<usize>> for IndexMap<K, V, S> {
            fn index_mut(&mut self, range: ops::Range<usize>) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::Range<usize>> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::Range<usize>) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::Range<usize>> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::Range<usize>) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<ops::RangeFrom<usize>> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeFrom<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::RangeFrom<usize>> for IndexMap<K, V, S> {
            fn index_mut(&mut self, range: ops::RangeFrom<usize>) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::RangeFrom<usize>> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeFrom<usize>) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::RangeFrom<usize>> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::RangeFrom<usize>) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<ops::RangeFull> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeFull) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::RangeFull> for IndexMap<K, V, S> {
            fn index_mut(&mut self, range: ops::RangeFull) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::RangeFull> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeFull) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::RangeFull> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::RangeFull) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<ops::RangeInclusive<usize>> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::RangeInclusive<usize>> for IndexMap<K, V, S> {
            fn index_mut(
                &mut self,
                range: ops::RangeInclusive<usize>,
            ) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::RangeInclusive<usize>> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeInclusive<usize>) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::RangeInclusive<usize>> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::RangeInclusive<usize>) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<ops::RangeTo<usize>> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeTo<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::RangeTo<usize>> for IndexMap<K, V, S> {
            fn index_mut(&mut self, range: ops::RangeTo<usize>) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::RangeTo<usize>> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeTo<usize>) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::RangeTo<usize>> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::RangeTo<usize>) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<ops::RangeToInclusive<usize>> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeToInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<ops::RangeToInclusive<usize>> for IndexMap<K, V, S> {
            fn index_mut(
                &mut self,
                range: ops::RangeToInclusive<usize>,
            ) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<ops::RangeToInclusive<usize>> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: ops::RangeToInclusive<usize>) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<ops::RangeToInclusive<usize>> for Slice<K, V> {
            fn index_mut(&mut self, range: ops::RangeToInclusive<usize>) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
        impl<K, V, S> Index<(Bound<usize>, Bound<usize>)> for IndexMap<K, V, S> {
            type Output = Slice<K, V>;
            fn index(&self, range: (Bound<usize>, Bound<usize>)) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<K, V, S> IndexMut<(Bound<usize>, Bound<usize>)> for IndexMap<K, V, S> {
            fn index_mut(
                &mut self,
                range: (Bound<usize>, Bound<usize>),
            ) -> &mut Self::Output {
                Slice::from_mut_slice(&mut self.as_entries_mut()[range])
            }
        }
        impl<K, V> Index<(Bound<usize>, Bound<usize>)> for Slice<K, V> {
            type Output = Slice<K, V>;
            fn index(&self, range: (Bound<usize>, Bound<usize>)) -> &Self {
                Self::from_slice(&self.entries[range])
            }
        }
        impl<K, V> IndexMut<(Bound<usize>, Bound<usize>)> for Slice<K, V> {
            fn index_mut(&mut self, range: (Bound<usize>, Bound<usize>)) -> &mut Self {
                Self::from_mut_slice(&mut self.entries[range])
            }
        }
    }
    pub mod raw_entry_v1 {
        //! Opt-in access to the experimental raw entry API.
        //!
        //! This module is designed to mimic the raw entry API of [`HashMap`][std::collections::hash_map],
        //! matching its unstable state as of Rust 1.75. See the tracking issue
        //! [rust#56167](https://github.com/rust-lang/rust/issues/56167) for more details.
        //!
        //! The trait [`RawEntryApiV1`] and the `_v1` suffix on its methods are meant to insulate this for
        //! the future, in case later breaking changes are needed. If the standard library stabilizes its
        //! `hash_raw_entry` feature (or some replacement), matching *inherent* methods will be added to
        //! `IndexMap` without such an opt-in trait.
        use super::{Core, OccupiedEntry};
        use crate::{Equivalent, HashValue, IndexMap};
        use core::fmt;
        use core::hash::{BuildHasher, Hash};
        use core::marker::PhantomData;
        use core::mem;
        /// Opt-in access to the experimental raw entry API.
        ///
        /// See the [`raw_entry_v1`][self] module documentation for more information.
        #[expect(private_bounds)]
        pub trait RawEntryApiV1<K, V, S>: Sealed {
            /// Creates a raw immutable entry builder for the [`IndexMap`].
            ///
            /// Raw entries provide the lowest level of control for searching and
            /// manipulating a map. They must be manually initialized with a hash and
            /// then manually searched.
            ///
            /// This is useful for
            /// * Hash memoization
            /// * Using a search key that doesn't work with the [`Equivalent`] trait
            /// * Using custom comparison logic without newtype wrappers
            ///
            /// Unless you are in such a situation, higher-level and more foolproof APIs like
            /// [`get`][IndexMap::get] should be preferred.
            ///
            /// Immutable raw entries have very limited use; you might instead want
            /// [`raw_entry_mut_v1`][Self::raw_entry_mut_v1].
            ///
            /// # Examples
            ///
            /// ```
            /// use core::hash::BuildHasher;
            /// use indexmap::map::{IndexMap, RawEntryApiV1};
            ///
            /// let mut map = IndexMap::new();
            /// map.extend([("a", 100), ("b", 200), ("c", 300)]);
            ///
            /// for k in ["a", "b", "c", "d", "e", "f"] {
            ///     let hash = map.hasher().hash_one(k);
            ///     let i = map.get_index_of(k);
            ///     let v = map.get(k);
            ///     let kv = map.get_key_value(k);
            ///     let ikv = map.get_full(k);
            ///
            ///     println!("Key: {} and value: {:?}", k, v);
            ///
            ///     assert_eq!(map.raw_entry_v1().from_key(k), kv);
            ///     assert_eq!(map.raw_entry_v1().from_hash(hash, |q| *q == k), kv);
            ///     assert_eq!(map.raw_entry_v1().from_key_hashed_nocheck(hash, k), kv);
            ///     assert_eq!(map.raw_entry_v1().from_hash_full(hash, |q| *q == k), ikv);
            ///     assert_eq!(map.raw_entry_v1().index_from_hash(hash, |q| *q == k), i);
            /// }
            /// ```
            fn raw_entry_v1(&self) -> RawEntryBuilder<'_, K, V, S>;
            /// Creates a raw entry builder for the [`IndexMap`].
            ///
            /// Raw entries provide the lowest level of control for searching and
            /// manipulating a map. They must be manually initialized with a hash and
            /// then manually searched. After this, insertions into a vacant entry
            /// still require an owned key to be provided.
            ///
            /// Raw entries are useful for such exotic situations as:
            ///
            /// * Hash memoization
            /// * Deferring the creation of an owned key until it is known to be required
            /// * Using a search key that doesn't work with the [`Equivalent`] trait
            /// * Using custom comparison logic without newtype wrappers
            ///
            /// Because raw entries provide much more low-level control, it's much easier
            /// to put the `IndexMap` into an inconsistent state which, while memory-safe,
            /// will cause the map to produce seemingly random results. Higher-level and more
            /// foolproof APIs like [`entry`][IndexMap::entry] should be preferred when possible.
            ///
            /// Raw entries give mutable access to the keys. This must not be used
            /// to modify how the key would compare or hash, as the map will not re-evaluate
            /// where the key should go, meaning the keys may become "lost" if their
            /// location does not reflect their state. For instance, if you change a key
            /// so that the map now contains keys which compare equal, search may start
            /// acting erratically, with two keys randomly masking each other. Implementations
            /// are free to assume this doesn't happen (within the limits of memory-safety).
            ///
            /// # Examples
            ///
            /// ```
            /// use core::hash::BuildHasher;
            /// use indexmap::map::{IndexMap, RawEntryApiV1};
            /// use indexmap::map::raw_entry_v1::RawEntryMut;
            ///
            /// let mut map = IndexMap::new();
            /// map.extend([("a", 100), ("b", 200), ("c", 300)]);
            ///
            /// // Existing key (insert and update)
            /// match map.raw_entry_mut_v1().from_key("a") {
            ///     RawEntryMut::Vacant(_) => unreachable!(),
            ///     RawEntryMut::Occupied(mut view) => {
            ///         assert_eq!(view.index(), 0);
            ///         assert_eq!(view.get(), &100);
            ///         let v = view.get_mut();
            ///         let new_v = (*v) * 10;
            ///         *v = new_v;
            ///         assert_eq!(view.insert(1111), 1000);
            ///     }
            /// }
            ///
            /// assert_eq!(map["a"], 1111);
            /// assert_eq!(map.len(), 3);
            ///
            /// // Existing key (take)
            /// let hash = map.hasher().hash_one("c");
            /// match map.raw_entry_mut_v1().from_key_hashed_nocheck(hash, "c") {
            ///     RawEntryMut::Vacant(_) => unreachable!(),
            ///     RawEntryMut::Occupied(view) => {
            ///         assert_eq!(view.index(), 2);
            ///         assert_eq!(view.shift_remove_entry(), ("c", 300));
            ///     }
            /// }
            /// assert_eq!(map.raw_entry_v1().from_key("c"), None);
            /// assert_eq!(map.len(), 2);
            ///
            /// // Nonexistent key (insert and update)
            /// let key = "d";
            /// let hash = map.hasher().hash_one(key);
            /// match map.raw_entry_mut_v1().from_hash(hash, |q| *q == key) {
            ///     RawEntryMut::Occupied(_) => unreachable!(),
            ///     RawEntryMut::Vacant(view) => {
            ///         assert_eq!(view.index(), 2);
            ///         let (k, value) = view.insert("d", 4000);
            ///         assert_eq!((*k, *value), ("d", 4000));
            ///         *value = 40000;
            ///     }
            /// }
            /// assert_eq!(map["d"], 40000);
            /// assert_eq!(map.len(), 3);
            ///
            /// match map.raw_entry_mut_v1().from_hash(hash, |q| *q == key) {
            ///     RawEntryMut::Vacant(_) => unreachable!(),
            ///     RawEntryMut::Occupied(view) => {
            ///         assert_eq!(view.index(), 2);
            ///         assert_eq!(view.swap_remove_entry(), ("d", 40000));
            ///     }
            /// }
            /// assert_eq!(map.get("d"), None);
            /// assert_eq!(map.len(), 2);
            /// ```
            fn raw_entry_mut_v1(&mut self) -> RawEntryBuilderMut<'_, K, V, S>;
        }
        impl<K, V, S> RawEntryApiV1<K, V, S> for IndexMap<K, V, S> {
            fn raw_entry_v1(&self) -> RawEntryBuilder<'_, K, V, S> {
                RawEntryBuilder { map: self }
            }
            fn raw_entry_mut_v1(&mut self) -> RawEntryBuilderMut<'_, K, V, S> {
                RawEntryBuilderMut { map: self }
            }
        }
        /// A builder for computing where in an [`IndexMap`] a key-value pair would be stored.
        ///
        /// This `struct` is created by the [`IndexMap::raw_entry_v1`] method, provided by the
        /// [`RawEntryApiV1`] trait. See its documentation for more.
        pub struct RawEntryBuilder<'a, K, V, S> {
            map: &'a IndexMap<K, V, S>,
        }
        impl<K, V, S> fmt::Debug for RawEntryBuilder<'_, K, V, S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RawEntryBuilder").finish_non_exhaustive()
            }
        }
        impl<'a, K, V, S> RawEntryBuilder<'a, K, V, S> {
            /// Access an entry by key.
            pub fn from_key<Q>(self, key: &Q) -> Option<(&'a K, &'a V)>
            where
                S: BuildHasher,
                Q: ?Sized + Hash + Equivalent<K>,
            {
                self.map.get_key_value(key)
            }
            /// Access an entry by a key and its hash.
            pub fn from_key_hashed_nocheck<Q>(
                self,
                hash: u64,
                key: &Q,
            ) -> Option<(&'a K, &'a V)>
            where
                Q: ?Sized + Equivalent<K>,
            {
                let hash = HashValue(hash as usize);
                let i = self.map.core.get_index_of(hash, key)?;
                self.map.get_index(i)
            }
            /// Access an entry by hash.
            pub fn from_hash<F>(self, hash: u64, is_match: F) -> Option<(&'a K, &'a V)>
            where
                F: FnMut(&K) -> bool,
            {
                let map = self.map;
                let i = self.index_from_hash(hash, is_match)?;
                map.get_index(i)
            }
            /// Access an entry by hash, including its index.
            pub fn from_hash_full<F>(
                self,
                hash: u64,
                is_match: F,
            ) -> Option<(usize, &'a K, &'a V)>
            where
                F: FnMut(&K) -> bool,
            {
                let map = self.map;
                let i = self.index_from_hash(hash, is_match)?;
                let (key, value) = map.get_index(i)?;
                Some((i, key, value))
            }
            /// Access the index of an entry by hash.
            pub fn index_from_hash<F>(self, hash: u64, is_match: F) -> Option<usize>
            where
                F: FnMut(&K) -> bool,
            {
                let hash = HashValue(hash as usize);
                self.map.core.get_index_of_raw(hash, is_match)
            }
        }
        /// A builder for computing where in an [`IndexMap`] a key-value pair would be stored.
        ///
        /// This `struct` is created by the [`IndexMap::raw_entry_mut_v1`] method, provided by the
        /// [`RawEntryApiV1`] trait. See its documentation for more.
        pub struct RawEntryBuilderMut<'a, K, V, S> {
            map: &'a mut IndexMap<K, V, S>,
        }
        impl<K, V, S> fmt::Debug for RawEntryBuilderMut<'_, K, V, S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RawEntryBuilderMut").finish_non_exhaustive()
            }
        }
        impl<'a, K, V, S> RawEntryBuilderMut<'a, K, V, S> {
            /// Access an entry by key.
            pub fn from_key<Q>(self, key: &Q) -> RawEntryMut<'a, K, V, S>
            where
                S: BuildHasher,
                Q: ?Sized + Hash + Equivalent<K>,
            {
                let hash = self.map.hash(key);
                self.from_key_hashed_nocheck(hash.get(), key)
            }
            /// Access an entry by a key and its hash.
            pub fn from_key_hashed_nocheck<Q>(
                self,
                hash: u64,
                key: &Q,
            ) -> RawEntryMut<'a, K, V, S>
            where
                Q: ?Sized + Equivalent<K>,
            {
                self.from_hash(hash, |k| Q::equivalent(key, k))
            }
            /// Access an entry by hash.
            pub fn from_hash<F>(self, hash: u64, is_match: F) -> RawEntryMut<'a, K, V, S>
            where
                F: FnMut(&K) -> bool,
            {
                let hash = HashValue(hash as usize);
                match OccupiedEntry::from_hash(&mut self.map.core, hash, is_match) {
                    Ok(inner) => {
                        RawEntryMut::Occupied(RawOccupiedEntryMut {
                            inner,
                            hash_builder: PhantomData,
                        })
                    }
                    Err(map) => {
                        RawEntryMut::Vacant(RawVacantEntryMut {
                            map,
                            hash_builder: &self.map.hash_builder,
                        })
                    }
                }
            }
        }
        /// Raw entry for an existing key-value pair or a vacant location to
        /// insert one.
        pub enum RawEntryMut<'a, K, V, S> {
            /// Existing slot with equivalent key.
            Occupied(RawOccupiedEntryMut<'a, K, V, S>),
            /// Vacant slot (no equivalent key in the map).
            Vacant(RawVacantEntryMut<'a, K, V, S>),
        }
        impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for RawEntryMut<'_, K, V, S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut tuple = f.debug_tuple("RawEntryMut");
                match self {
                    Self::Vacant(v) => tuple.field(v),
                    Self::Occupied(o) => tuple.field(o),
                };
                tuple.finish()
            }
        }
        impl<'a, K, V, S> RawEntryMut<'a, K, V, S> {
            /// Return the index where the key-value pair exists or may be inserted.
            #[inline]
            pub fn index(&self) -> usize {
                match self {
                    Self::Occupied(entry) => entry.index(),
                    Self::Vacant(entry) => entry.index(),
                }
            }
            /// Inserts the given default key and value in the entry if it is vacant and returns mutable
            /// references to them. Otherwise mutable references to an already existent pair are returned.
            pub fn or_insert(
                self,
                default_key: K,
                default_value: V,
            ) -> (&'a mut K, &'a mut V)
            where
                K: Hash,
                S: BuildHasher,
            {
                match self {
                    Self::Occupied(entry) => entry.into_key_value_mut(),
                    Self::Vacant(entry) => entry.insert(default_key, default_value),
                }
            }
            /// Inserts the result of the `call` function in the entry if it is vacant and returns mutable
            /// references to them. Otherwise mutable references to an already existent pair are returned.
            pub fn or_insert_with<F>(self, call: F) -> (&'a mut K, &'a mut V)
            where
                F: FnOnce() -> (K, V),
                K: Hash,
                S: BuildHasher,
            {
                match self {
                    Self::Occupied(entry) => entry.into_key_value_mut(),
                    Self::Vacant(entry) => {
                        let (key, value) = call();
                        entry.insert(key, value)
                    }
                }
            }
            /// Modifies the entry if it is occupied.
            pub fn and_modify<F>(mut self, f: F) -> Self
            where
                F: FnOnce(&mut K, &mut V),
            {
                if let Self::Occupied(entry) = &mut self {
                    let (k, v) = entry.get_key_value_mut();
                    f(k, v);
                }
                self
            }
        }
        /// A raw view into an occupied entry in an [`IndexMap`].
        /// It is part of the [`RawEntryMut`] enum.
        pub struct RawOccupiedEntryMut<'a, K, V, S> {
            inner: OccupiedEntry<'a, K, V>,
            hash_builder: PhantomData<&'a S>,
        }
        impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug
        for RawOccupiedEntryMut<'_, K, V, S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RawOccupiedEntryMut")
                    .field("key", self.key())
                    .field("value", self.get())
                    .finish_non_exhaustive()
            }
        }
        impl<'a, K, V, S> RawOccupiedEntryMut<'a, K, V, S> {
            /// Return the index of the key-value pair
            #[inline]
            pub fn index(&self) -> usize {
                self.inner.index()
            }
            /// Gets a reference to the entry's key in the map.
            ///
            /// Note that this is not the key that was used to find the entry. There may be an observable
            /// difference if the key type has any distinguishing features outside of `Hash` and `Eq`, like
            /// extra fields or the memory address of an allocation.
            pub fn key(&self) -> &K {
                self.inner.key()
            }
            /// Gets a mutable reference to the entry's key in the map.
            ///
            /// Note that this is not the key that was used to find the entry. There may be an observable
            /// difference if the key type has any distinguishing features outside of `Hash` and `Eq`, like
            /// extra fields or the memory address of an allocation.
            pub fn key_mut(&mut self) -> &mut K {
                &mut self.inner.get_bucket_mut().key
            }
            /// Converts into a mutable reference to the entry's key in the map,
            /// with a lifetime bound to the map itself.
            ///
            /// Note that this is not the key that was used to find the entry. There may be an observable
            /// difference if the key type has any distinguishing features outside of `Hash` and `Eq`, like
            /// extra fields or the memory address of an allocation.
            pub fn into_key(self) -> &'a mut K {
                &mut self.inner.into_bucket().key
            }
            /// Gets a reference to the entry's value in the map.
            pub fn get(&self) -> &V {
                self.inner.get()
            }
            /// Gets a mutable reference to the entry's value in the map.
            ///
            /// If you need a reference which may outlive the destruction of the
            /// [`RawEntryMut`] value, see [`into_mut`][Self::into_mut].
            pub fn get_mut(&mut self) -> &mut V {
                self.inner.get_mut()
            }
            /// Converts into a mutable reference to the entry's value in the map,
            /// with a lifetime bound to the map itself.
            pub fn into_mut(self) -> &'a mut V {
                self.inner.into_mut()
            }
            /// Gets a reference to the entry's key and value in the map.
            pub fn get_key_value(&self) -> (&K, &V) {
                self.inner.get_bucket().refs()
            }
            /// Gets a reference to the entry's key and value in the map.
            pub fn get_key_value_mut(&mut self) -> (&mut K, &mut V) {
                self.inner.get_bucket_mut().muts()
            }
            /// Converts into a mutable reference to the entry's key and value in the map,
            /// with a lifetime bound to the map itself.
            pub fn into_key_value_mut(self) -> (&'a mut K, &'a mut V) {
                self.inner.into_bucket().muts()
            }
            /// Sets the value of the entry, and returns the entry's old value.
            pub fn insert(&mut self, value: V) -> V {
                self.inner.insert(value)
            }
            /// Sets the key of the entry, and returns the entry's old key.
            pub fn insert_key(&mut self, key: K) -> K {
                mem::replace(self.key_mut(), key)
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// **NOTE:** This is equivalent to [`.swap_remove()`][Self::swap_remove], replacing this
            /// entry's position with the last element, and it is deprecated in favor of calling that
            /// explicitly. If you need to preserve the relative order of the keys in the map, use
            /// [`.shift_remove()`][Self::shift_remove] instead.
            #[deprecated(
                note = "`remove` disrupts the map order -- \
        use `swap_remove` or `shift_remove` for explicit behavior."
            )]
            pub fn remove(self) -> V {
                self.swap_remove()
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove(self) -> V {
                self.inner.swap_remove()
            }
            /// Remove the key, value pair stored in the map for this entry, and return the value.
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove(self) -> V {
                self.inner.shift_remove()
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// **NOTE:** This is equivalent to [`.swap_remove_entry()`][Self::swap_remove_entry],
            /// replacing this entry's position with the last element, and it is deprecated in favor of
            /// calling that explicitly. If you need to preserve the relative order of the keys in the map,
            /// use [`.shift_remove_entry()`][Self::shift_remove_entry] instead.
            #[deprecated(
                note = "`remove_entry` disrupts the map order -- \
        use `swap_remove_entry` or `shift_remove_entry` for explicit behavior."
            )]
            pub fn remove_entry(self) -> (K, V) {
                self.swap_remove_entry()
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::swap_remove`][alloc::vec::Vec::swap_remove], the pair is removed by swapping it
            /// with the last element of the map and popping it off.
            /// **This perturbs the position of what used to be the last element!**
            ///
            /// Computes in **O(1)** time (average).
            pub fn swap_remove_entry(self) -> (K, V) {
                self.inner.swap_remove_entry()
            }
            /// Remove and return the key, value pair stored in the map for this entry
            ///
            /// Like [`Vec::remove`][alloc::vec::Vec::remove], the pair is removed by shifting all of the
            /// elements that follow it, preserving their relative order.
            /// **This perturbs the index of all of those elements!**
            ///
            /// Computes in **O(n)** time (average).
            pub fn shift_remove_entry(self) -> (K, V) {
                self.inner.shift_remove_entry()
            }
            /// Moves the position of the entry to a new index
            /// by shifting all other entries in-between.
            ///
            /// This is equivalent to [`IndexMap::move_index`]
            /// coming `from` the current [`.index()`][Self::index].
            ///
            /// * If `self.index() < to`, the other pairs will shift down while the targeted pair moves up.
            /// * If `self.index() > to`, the other pairs will shift up while the targeted pair moves down.
            ///
            /// ***Panics*** if `to` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn move_index(self, to: usize) {
                self.inner.move_index(to);
            }
            /// Swaps the position of entry with another.
            ///
            /// This is equivalent to [`IndexMap::swap_indices`]
            /// with the current [`.index()`][Self::index] as one of the two being swapped.
            ///
            /// ***Panics*** if the `other` index is out of bounds.
            ///
            /// Computes in **O(1)** time (average).
            #[track_caller]
            pub fn swap_indices(self, other: usize) {
                self.inner.swap_indices(other);
            }
        }
        /// A view into a vacant raw entry in an [`IndexMap`].
        /// It is part of the [`RawEntryMut`] enum.
        pub struct RawVacantEntryMut<'a, K, V, S> {
            map: &'a mut Core<K, V>,
            hash_builder: &'a S,
        }
        impl<K, V, S> fmt::Debug for RawVacantEntryMut<'_, K, V, S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RawVacantEntryMut").finish_non_exhaustive()
            }
        }
        impl<'a, K, V, S> RawVacantEntryMut<'a, K, V, S> {
            /// Return the index where a key-value pair may be inserted.
            pub fn index(&self) -> usize {
                self.map.len()
            }
            /// Inserts the given key and value into the map,
            /// and returns mutable references to them.
            pub fn insert(self, key: K, value: V) -> (&'a mut K, &'a mut V)
            where
                K: Hash,
                S: BuildHasher,
            {
                let h = self.hash_builder.hash_one(&key);
                self.insert_hashed_nocheck(h, key, value)
            }
            /// Inserts the given key and value into the map with the provided hash,
            /// and returns mutable references to them.
            pub fn insert_hashed_nocheck(
                self,
                hash: u64,
                key: K,
                value: V,
            ) -> (&'a mut K, &'a mut V) {
                let hash = HashValue(hash as usize);
                self.map.insert_unique(hash, key, value).muts()
            }
            /// Inserts the given key and value into the map at the given index,
            /// shifting others to the right, and returns mutable references to them.
            ///
            /// ***Panics*** if `index` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn shift_insert(
                self,
                index: usize,
                key: K,
                value: V,
            ) -> (&'a mut K, &'a mut V)
            where
                K: Hash,
                S: BuildHasher,
            {
                let h = self.hash_builder.hash_one(&key);
                self.shift_insert_hashed_nocheck(index, h, key, value)
            }
            /// Inserts the given key and value into the map with the provided hash
            /// at the given index, and returns mutable references to them.
            ///
            /// ***Panics*** if `index` is out of bounds.
            ///
            /// Computes in **O(n)** time (average).
            #[track_caller]
            pub fn shift_insert_hashed_nocheck(
                self,
                index: usize,
                hash: u64,
                key: K,
                value: V,
            ) -> (&'a mut K, &'a mut V) {
                let hash = HashValue(hash as usize);
                self.map.shift_insert_unique(index, hash, key, value).muts()
            }
        }
        trait Sealed {}
        impl<K, V, S> Sealed for IndexMap<K, V, S> {}
    }
    pub use self::entry::{Entry, IndexedEntry};
    pub use crate::inner::{OccupiedEntry, VacantEntry};
    pub use self::iter::{
        Drain, ExtractIf, IntoIter, IntoKeys, IntoValues, Iter, IterMut, IterMut2, Keys,
        Splice, Values, ValuesMut,
    };
    pub use self::mutable::MutableEntryKey;
    pub use self::mutable::MutableKeys;
    pub use self::raw_entry_v1::RawEntryApiV1;
    pub use self::slice::Slice;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::cmp::Ordering;
    use core::fmt;
    use core::hash::{BuildHasher, Hash};
    use core::mem;
    use core::ops::{Index, IndexMut, RangeBounds};
    use std::hash::RandomState;
    use crate::inner::Core;
    use crate::util::{third, try_simplify_range};
    use crate::{Bucket, Equivalent, GetDisjointMutError, HashValue, TryReserveError};
    /// A hash table where the iteration order of the key-value pairs is independent
    /// of the hash values of the keys.
    ///
    /// The interface is closely compatible with the standard
    /// [`HashMap`][std::collections::HashMap],
    /// but also has additional features.
    ///
    /// # Order
    ///
    /// The key-value pairs have a consistent order that is determined by
    /// the sequence of insertion and removal calls on the map. The order does
    /// not depend on the keys or the hash function at all.
    ///
    /// All iterators traverse the map in *the order*.
    ///
    /// The insertion order is preserved, with **notable exceptions** like the
    /// [`.remove()`][Self::remove] or [`.swap_remove()`][Self::swap_remove] methods.
    /// Methods such as [`.sort_by()`][Self::sort_by] of
    /// course result in a new order, depending on the sorting order.
    ///
    /// # Indices
    ///
    /// The key-value pairs are indexed in a compact range without holes in the
    /// range `0..self.len()`. For example, the method `.get_full` looks up the
    /// index for a key, and the method `.get_index` looks up the key-value pair by
    /// index.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    ///
    /// // count the frequency of each letter in a sentence.
    /// let mut letters = IndexMap::new();
    /// for ch in "a short treatise on fungi".chars() {
    ///     *letters.entry(ch).or_insert(0) += 1;
    /// }
    ///
    /// assert_eq!(letters[&'s'], 2);
    /// assert_eq!(letters[&'t'], 3);
    /// assert_eq!(letters[&'u'], 1);
    /// assert_eq!(letters.get(&'y'), None);
    /// ```
    pub struct IndexMap<K, V, S = RandomState> {
        pub(crate) core: Core<K, V>,
        hash_builder: S,
    }
    impl<K, V, S> Clone for IndexMap<K, V, S>
    where
        K: Clone,
        V: Clone,
        S: Clone,
    {
        fn clone(&self) -> Self {
            IndexMap {
                core: self.core.clone(),
                hash_builder: self.hash_builder.clone(),
            }
        }
        fn clone_from(&mut self, other: &Self) {
            self.core.clone_from(&other.core);
            self.hash_builder.clone_from(&other.hash_builder);
        }
    }
    impl<K, V, S> fmt::Debug for IndexMap<K, V, S>
    where
        K: fmt::Debug,
        V: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map().entries(self.iter()).finish()
        }
    }
    impl<K, V> IndexMap<K, V> {
        /// Create a new map. (Does not allocate.)
        #[inline]
        pub fn new() -> Self {
            Self::with_capacity(0)
        }
        /// Create a new map with capacity for `n` key-value pairs. (Does not
        /// allocate if `n` is zero.)
        ///
        /// Computes in **O(n)** time.
        #[inline]
        pub fn with_capacity(n: usize) -> Self {
            Self::with_capacity_and_hasher(n, <_>::default())
        }
    }
    impl<K, V, S> IndexMap<K, V, S> {
        /// Create a new map with capacity for `n` key-value pairs. (Does not
        /// allocate if `n` is zero.)
        ///
        /// Computes in **O(n)** time.
        #[inline]
        pub fn with_capacity_and_hasher(n: usize, hash_builder: S) -> Self {
            if n == 0 {
                Self::with_hasher(hash_builder)
            } else {
                IndexMap {
                    core: Core::with_capacity(n),
                    hash_builder,
                }
            }
        }
        /// Create a new map with `hash_builder`.
        ///
        /// This function is `const`, so it
        /// can be called in `static` contexts.
        pub const fn with_hasher(hash_builder: S) -> Self {
            IndexMap {
                core: Core::new(),
                hash_builder,
            }
        }
        #[inline]
        pub(crate) fn into_entries(self) -> Vec<Bucket<K, V>> {
            self.core.into_entries()
        }
        #[inline]
        pub(crate) fn as_entries(&self) -> &[Bucket<K, V>] {
            self.core.as_entries()
        }
        #[inline]
        pub(crate) fn as_entries_mut(&mut self) -> &mut [Bucket<K, V>] {
            self.core.as_entries_mut()
        }
        pub(crate) fn with_entries<F>(&mut self, f: F)
        where
            F: FnOnce(&mut [Bucket<K, V>]),
        {
            self.core.with_entries(f);
        }
        /// Return the number of elements the map can hold without reallocating.
        ///
        /// This number is a lower bound; the map might be able to hold more,
        /// but is guaranteed to be able to hold at least this many.
        ///
        /// Computes in **O(1)** time.
        pub fn capacity(&self) -> usize {
            self.core.capacity()
        }
        /// Return a reference to the map's `BuildHasher`.
        pub fn hasher(&self) -> &S {
            &self.hash_builder
        }
        /// Return the number of key-value pairs in the map.
        ///
        /// Computes in **O(1)** time.
        #[inline]
        pub fn len(&self) -> usize {
            self.core.len()
        }
        /// Returns true if the map contains no elements.
        ///
        /// Computes in **O(1)** time.
        #[inline]
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
        /// Return an iterator over the key-value pairs of the map, in their order
        pub fn iter(&self) -> Iter<'_, K, V> {
            Iter::new(self.as_entries())
        }
        /// Return an iterator over the key-value pairs of the map, in their order
        pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
            IterMut::new(self.as_entries_mut())
        }
        /// Return an iterator over the keys of the map, in their order
        pub fn keys(&self) -> Keys<'_, K, V> {
            Keys::new(self.as_entries())
        }
        /// Return an owning iterator over the keys of the map, in their order
        pub fn into_keys(self) -> IntoKeys<K, V> {
            IntoKeys::new(self.into_entries())
        }
        /// Return an iterator over the values of the map, in their order
        pub fn values(&self) -> Values<'_, K, V> {
            Values::new(self.as_entries())
        }
        /// Return an iterator over mutable references to the values of the map,
        /// in their order
        pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
            ValuesMut::new(self.as_entries_mut())
        }
        /// Return an owning iterator over the values of the map, in their order
        pub fn into_values(self) -> IntoValues<K, V> {
            IntoValues::new(self.into_entries())
        }
        /// Remove all key-value pairs in the map, while preserving its capacity.
        ///
        /// Computes in **O(n)** time.
        pub fn clear(&mut self) {
            self.core.clear();
        }
        /// Shortens the map, keeping the first `len` elements and dropping the rest.
        ///
        /// If `len` is greater than the map's current length, this has no effect.
        pub fn truncate(&mut self, len: usize) {
            self.core.truncate(len);
        }
        /// Clears the `IndexMap` in the given index range, returning those
        /// key-value pairs as a drain iterator.
        ///
        /// The range may be any type that implements [`RangeBounds<usize>`],
        /// including all of the `std::ops::Range*` types, or even a tuple pair of
        /// `Bound` start and end values. To drain the map entirely, use `RangeFull`
        /// like `map.drain(..)`.
        ///
        /// This shifts down all entries following the drained range to fill the
        /// gap, and keeps the allocated memory for reuse.
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the map.
        #[track_caller]
        pub fn drain<R>(&mut self, range: R) -> Drain<'_, K, V>
        where
            R: RangeBounds<usize>,
        {
            Drain::new(self.core.drain(range))
        }
        /// Creates an iterator which uses a closure to determine if an element should be removed,
        /// for all elements in the given range.
        ///
        /// If the closure returns true, the element is removed from the map and yielded.
        /// If the closure returns false, or panics, the element remains in the map and will not be
        /// yielded.
        ///
        /// Note that `extract_if` lets you mutate every value in the filter closure, regardless of
        /// whether you choose to keep or remove it.
        ///
        /// The range may be any type that implements [`RangeBounds<usize>`],
        /// including all of the `std::ops::Range*` types, or even a tuple pair of
        /// `Bound` start and end values. To check the entire map, use `RangeFull`
        /// like `map.extract_if(.., predicate)`.
        ///
        /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
        /// or the iteration short-circuits, then the remaining elements will be retained.
        /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
        ///
        /// [`retain`]: IndexMap::retain
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the map.
        ///
        /// # Examples
        ///
        /// Splitting a map into even and odd keys, reusing the original map:
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// let mut map: IndexMap<i32, i32> = (0..8).map(|x| (x, x)).collect();
        /// let extracted: IndexMap<i32, i32> = map.extract_if(.., |k, _v| k % 2 == 0).collect();
        ///
        /// let evens = extracted.keys().copied().collect::<Vec<_>>();
        /// let odds = map.keys().copied().collect::<Vec<_>>();
        ///
        /// assert_eq!(evens, vec![0, 2, 4, 6]);
        /// assert_eq!(odds, vec![1, 3, 5, 7]);
        /// ```
        #[track_caller]
        pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> ExtractIf<'_, K, V, F>
        where
            F: FnMut(&K, &mut V) -> bool,
            R: RangeBounds<usize>,
        {
            ExtractIf::new(&mut self.core, range, pred)
        }
        /// Splits the collection into two at the given index.
        ///
        /// Returns a newly allocated map containing the elements in the range
        /// `[at, len)`. After the call, the original map will be left containing
        /// the elements `[0, at)` with its previous capacity unchanged.
        ///
        /// ***Panics*** if `at > len`.
        #[track_caller]
        pub fn split_off(&mut self, at: usize) -> Self
        where
            S: Clone,
        {
            Self {
                core: self.core.split_off(at),
                hash_builder: self.hash_builder.clone(),
            }
        }
        /// Reserve capacity for `additional` more key-value pairs.
        ///
        /// Computes in **O(n)** time.
        pub fn reserve(&mut self, additional: usize) {
            self.core.reserve(additional);
        }
        /// Reserve capacity for `additional` more key-value pairs, without over-allocating.
        ///
        /// Unlike `reserve`, this does not deliberately over-allocate the entry capacity to avoid
        /// frequent re-allocations. However, the underlying data structures may still have internal
        /// capacity requirements, and the allocator itself may give more space than requested, so this
        /// cannot be relied upon to be precisely minimal.
        ///
        /// Computes in **O(n)** time.
        pub fn reserve_exact(&mut self, additional: usize) {
            self.core.reserve_exact(additional);
        }
        /// Try to reserve capacity for `additional` more key-value pairs.
        ///
        /// Computes in **O(n)** time.
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
            self.core.try_reserve(additional)
        }
        /// Try to reserve capacity for `additional` more key-value pairs, without over-allocating.
        ///
        /// Unlike `try_reserve`, this does not deliberately over-allocate the entry capacity to avoid
        /// frequent re-allocations. However, the underlying data structures may still have internal
        /// capacity requirements, and the allocator itself may give more space than requested, so this
        /// cannot be relied upon to be precisely minimal.
        ///
        /// Computes in **O(n)** time.
        pub fn try_reserve_exact(
            &mut self,
            additional: usize,
        ) -> Result<(), TryReserveError> {
            self.core.try_reserve_exact(additional)
        }
        /// Shrink the capacity of the map as much as possible.
        ///
        /// Computes in **O(n)** time.
        pub fn shrink_to_fit(&mut self) {
            self.core.shrink_to(0);
        }
        /// Shrink the capacity of the map with a lower limit.
        ///
        /// Computes in **O(n)** time.
        pub fn shrink_to(&mut self, min_capacity: usize) {
            self.core.shrink_to(min_capacity);
        }
    }
    impl<K, V, S> IndexMap<K, V, S>
    where
        K: Hash + Eq,
        S: BuildHasher,
    {
        /// Insert a key-value pair in the map.
        ///
        /// If an equivalent key already exists in the map: the key remains and
        /// retains in its place in the order, its corresponding value is updated
        /// with `value`, and the older value is returned inside `Some(_)`.
        ///
        /// If no equivalent key existed in the map: the new key-value pair is
        /// inserted, last in order, and `None` is returned.
        ///
        /// Computes in **O(1)** time (amortized average).
        ///
        /// See also [`entry`][Self::entry] if you want to insert *or* modify,
        /// or [`insert_full`][Self::insert_full] if you need to get the index of
        /// the corresponding key-value pair.
        pub fn insert(&mut self, key: K, value: V) -> Option<V> {
            self.insert_full(key, value).1
        }
        /// Insert a key-value pair in the map, and get their index.
        ///
        /// If an equivalent key already exists in the map: the key remains and
        /// retains in its place in the order, its corresponding value is updated
        /// with `value`, and the older value is returned inside `(index, Some(_))`.
        ///
        /// If no equivalent key existed in the map: the new key-value pair is
        /// inserted, last in order, and `(index, None)` is returned.
        ///
        /// Computes in **O(1)** time (amortized average).
        ///
        /// See also [`entry`][Self::entry] if you want to insert *or* modify.
        pub fn insert_full(&mut self, key: K, value: V) -> (usize, Option<V>) {
            let hash = self.hash(&key);
            self.core.insert_full(hash, key, value)
        }
        /// Insert a key-value pair in the map at its ordered position among sorted keys.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search_keys`][Self::binary_search_keys], then either updating
        /// it or calling [`insert_before`][Self::insert_before] for a new key.
        ///
        /// If the sorted key is found in the map, its corresponding value is
        /// updated with `value`, and the older value is returned inside
        /// `(index, Some(_))`. Otherwise, the new key-value pair is inserted at
        /// the sorted position, and `(index, None)` is returned.
        ///
        /// If the existing keys are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the key-value
        /// pair is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average). Instead of repeating calls to
        /// `insert_sorted`, it may be faster to call batched [`insert`][Self::insert]
        /// or [`extend`][Self::extend] and only call [`sort_keys`][Self::sort_keys]
        /// or [`sort_unstable_keys`][Self::sort_unstable_keys] once.
        pub fn insert_sorted(&mut self, key: K, value: V) -> (usize, Option<V>)
        where
            K: Ord,
        {
            match self.binary_search_keys(&key) {
                Ok(i) => (i, Some(mem::replace(&mut self[i], value))),
                Err(i) => self.insert_before(i, key, value),
            }
        }
        /// Insert a key-value pair in the map at its ordered position among keys
        /// sorted by `cmp`.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search_by`][Self::binary_search_by], then calling
        /// [`insert_before`][Self::insert_before] with the given key and value.
        ///
        /// If the existing keys are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the key-value
        /// pair is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average).
        pub fn insert_sorted_by<F>(
            &mut self,
            key: K,
            value: V,
            mut cmp: F,
        ) -> (usize, Option<V>)
        where
            F: FnMut(&K, &V, &K, &V) -> Ordering,
        {
            let (Ok(i) | Err(i)) = self.binary_search_by(|k, v| cmp(k, v, &key, &value));
            self.insert_before(i, key, value)
        }
        /// Insert a key-value pair in the map at its ordered position
        /// using a sort-key extraction function.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search_by_key`][Self::binary_search_by_key] with `sort_key(key)`, then
        /// calling [`insert_before`][Self::insert_before] with the given key and value.
        ///
        /// If the existing keys are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the key-value
        /// pair is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average).
        pub fn insert_sorted_by_key<B, F>(
            &mut self,
            key: K,
            value: V,
            mut sort_key: F,
        ) -> (usize, Option<V>)
        where
            B: Ord,
            F: FnMut(&K, &V) -> B,
        {
            let search_key = sort_key(&key, &value);
            let (Ok(i) | Err(i)) = self.binary_search_by_key(&search_key, sort_key);
            self.insert_before(i, key, value)
        }
        /// Insert a key-value pair in the map before the entry at the given index, or at the end.
        ///
        /// If an equivalent key already exists in the map: the key remains and
        /// is moved to the new position in the map, its corresponding value is updated
        /// with `value`, and the older value is returned inside `Some(_)`. The returned index
        /// will either be the given index or one less, depending on how the entry moved.
        /// (See [`shift_insert`](Self::shift_insert) for different behavior here.)
        ///
        /// If no equivalent key existed in the map: the new key-value pair is
        /// inserted exactly at the given index, and `None` is returned.
        ///
        /// ***Panics*** if `index` is out of bounds.
        /// Valid indices are `0..=map.len()` (inclusive).
        ///
        /// Computes in **O(n)** time (average).
        ///
        /// See also [`entry`][Self::entry] if you want to insert *or* modify,
        /// perhaps only using the index for new entries with [`VacantEntry::shift_insert`].
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        /// let mut map: IndexMap<char, ()> = ('a'..='z').map(|c| (c, ())).collect();
        ///
        /// // The new key '*' goes exactly at the given index.
        /// assert_eq!(map.get_index_of(&'*'), None);
        /// assert_eq!(map.insert_before(10, '*', ()), (10, None));
        /// assert_eq!(map.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the key 'a' up will shift others down, so this moves *before* 10 to index 9.
        /// assert_eq!(map.insert_before(10, 'a', ()), (9, Some(())));
        /// assert_eq!(map.get_index_of(&'a'), Some(9));
        /// assert_eq!(map.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the key 'z' down will shift others up, so this moves to exactly 10.
        /// assert_eq!(map.insert_before(10, 'z', ()), (10, Some(())));
        /// assert_eq!(map.get_index_of(&'z'), Some(10));
        /// assert_eq!(map.get_index_of(&'*'), Some(11));
        ///
        /// // Moving or inserting before the endpoint is also valid.
        /// assert_eq!(map.len(), 27);
        /// assert_eq!(map.insert_before(map.len(), '*', ()), (26, Some(())));
        /// assert_eq!(map.get_index_of(&'*'), Some(26));
        /// assert_eq!(map.insert_before(map.len(), '+', ()), (27, None));
        /// assert_eq!(map.get_index_of(&'+'), Some(27));
        /// assert_eq!(map.len(), 28);
        /// ```
        #[track_caller]
        pub fn insert_before(
            &mut self,
            mut index: usize,
            key: K,
            value: V,
        ) -> (usize, Option<V>) {
            let len = self.len();
            if !(index <= len) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "index out of bounds: the len is {0} but the index is {1}. Expected index <= len",
                            len,
                            index,
                        ),
                    );
                }
            }
            match self.entry(key) {
                Entry::Occupied(mut entry) => {
                    if index > entry.index() {
                        index -= 1;
                    }
                    let old = mem::replace(entry.get_mut(), value);
                    entry.move_index(index);
                    (index, Some(old))
                }
                Entry::Vacant(entry) => {
                    entry.shift_insert(index, value);
                    (index, None)
                }
            }
        }
        /// Insert a key-value pair in the map at the given index.
        ///
        /// If an equivalent key already exists in the map: the key remains and
        /// is moved to the given index in the map, its corresponding value is updated
        /// with `value`, and the older value is returned inside `Some(_)`.
        /// Note that existing entries **cannot** be moved to `index == map.len()`!
        /// (See [`insert_before`](Self::insert_before) for different behavior here.)
        ///
        /// If no equivalent key existed in the map: the new key-value pair is
        /// inserted at the given index, and `None` is returned.
        ///
        /// ***Panics*** if `index` is out of bounds.
        /// Valid indices are `0..map.len()` (exclusive) when moving an existing entry, or
        /// `0..=map.len()` (inclusive) when inserting a new key.
        ///
        /// Computes in **O(n)** time (average).
        ///
        /// See also [`entry`][Self::entry] if you want to insert *or* modify,
        /// perhaps only using the index for new entries with [`VacantEntry::shift_insert`].
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        /// let mut map: IndexMap<char, ()> = ('a'..='z').map(|c| (c, ())).collect();
        ///
        /// // The new key '*' goes exactly at the given index.
        /// assert_eq!(map.get_index_of(&'*'), None);
        /// assert_eq!(map.shift_insert(10, '*', ()), None);
        /// assert_eq!(map.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the key 'a' up to 10 will shift others down, including the '*' that was at 10.
        /// assert_eq!(map.shift_insert(10, 'a', ()), Some(()));
        /// assert_eq!(map.get_index_of(&'a'), Some(10));
        /// assert_eq!(map.get_index_of(&'*'), Some(9));
        ///
        /// // Moving the key 'z' down to 9 will shift others up, including the '*' that was at 9.
        /// assert_eq!(map.shift_insert(9, 'z', ()), Some(()));
        /// assert_eq!(map.get_index_of(&'z'), Some(9));
        /// assert_eq!(map.get_index_of(&'*'), Some(10));
        ///
        /// // Existing keys can move to len-1 at most, but new keys can insert at the endpoint.
        /// assert_eq!(map.len(), 27);
        /// assert_eq!(map.shift_insert(map.len() - 1, '*', ()), Some(()));
        /// assert_eq!(map.get_index_of(&'*'), Some(26));
        /// assert_eq!(map.shift_insert(map.len(), '+', ()), None);
        /// assert_eq!(map.get_index_of(&'+'), Some(27));
        /// assert_eq!(map.len(), 28);
        /// ```
        ///
        /// ```should_panic
        /// use indexmap::IndexMap;
        /// let mut map: IndexMap<char, ()> = ('a'..='z').map(|c| (c, ())).collect();
        ///
        /// // This is an invalid index for moving an existing key!
        /// map.shift_insert(map.len(), 'a', ());
        /// ```
        #[track_caller]
        pub fn shift_insert(&mut self, index: usize, key: K, value: V) -> Option<V> {
            let len = self.len();
            match self.entry(key) {
                Entry::Occupied(mut entry) => {
                    if !(index < len) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "index out of bounds: the len is {0} but the index is {1}",
                                    len,
                                    index,
                                ),
                            );
                        }
                    }
                    let old = mem::replace(entry.get_mut(), value);
                    entry.move_index(index);
                    Some(old)
                }
                Entry::Vacant(entry) => {
                    if !(index <= len) {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "index out of bounds: the len is {0} but the index is {1}. Expected index <= len",
                                    len,
                                    index,
                                ),
                            );
                        }
                    }
                    entry.shift_insert(index, value);
                    None
                }
            }
        }
        /// Replaces the key at the given index. The new key does not need to be
        /// equivalent to the one it is replacing, but it must be unique to the rest
        /// of the map.
        ///
        /// Returns `Ok(old_key)` if successful, or `Err((other_index, key))` if an
        /// equivalent key already exists at a different index. The map will be
        /// unchanged in the error case.
        ///
        /// Direct indexing can be used to change the corresponding value: simply
        /// `map[index] = value`, or `mem::replace(&mut map[index], value)` to
        /// retrieve the old value as well.
        ///
        /// ***Panics*** if `index` is out of bounds.
        ///
        /// Computes in **O(1)** time (average).
        #[track_caller]
        pub fn replace_index(&mut self, index: usize, key: K) -> Result<K, (usize, K)> {
            let entry = &mut self.as_entries_mut()[index];
            if key == entry.key {
                return Ok(mem::replace(&mut entry.key, key));
            }
            let hash = self.hash(&key);
            if let Some(i) = self.core.get_index_of(hash, &key) {
                if true {
                    match (&i, &index) {
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
                return Err((i, key));
            }
            Ok(self.core.replace_index_unique(index, hash, key))
        }
        /// Get the given key's corresponding entry in the map for insertion and/or
        /// in-place manipulation.
        ///
        /// Computes in **O(1)** time (amortized average).
        pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
            let hash = self.hash(&key);
            Entry::new(&mut self.core, hash, key)
        }
        /// Creates a splicing iterator that replaces the specified range in the map
        /// with the given `replace_with` key-value iterator and yields the removed
        /// items. `replace_with` does not need to be the same length as `range`.
        ///
        /// The `range` is removed even if the iterator is not consumed until the
        /// end. It is unspecified how many elements are removed from the map if the
        /// `Splice` value is leaked.
        ///
        /// The input iterator `replace_with` is only consumed when the `Splice`
        /// value is dropped. If a key from the iterator matches an existing entry
        /// in the map (outside of `range`), then the value will be updated in that
        /// position. Otherwise, the new key-value pair will be inserted in the
        /// replaced `range`.
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the map.
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// let mut map = IndexMap::from([(0, '_'), (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd')]);
        /// let new = [(5, 'E'), (4, 'D'), (3, 'C'), (2, 'B'), (1, 'A')];
        /// let removed: Vec<_> = map.splice(2..4, new).collect();
        ///
        /// // 1 and 4 got new values, while 5, 3, and 2 were newly inserted.
        /// assert!(map.into_iter().eq([(0, '_'), (1, 'A'), (5, 'E'), (3, 'C'), (2, 'B'), (4, 'D')]));
        /// assert_eq!(removed, &[(2, 'b'), (3, 'c')]);
        /// ```
        #[track_caller]
        pub fn splice<R, I>(
            &mut self,
            range: R,
            replace_with: I,
        ) -> Splice<'_, I::IntoIter, K, V, S>
        where
            R: RangeBounds<usize>,
            I: IntoIterator<Item = (K, V)>,
        {
            Splice::new(self, range, replace_with.into_iter())
        }
        /// Moves all key-value pairs from `other` into `self`, leaving `other` empty.
        ///
        /// This is equivalent to calling [`insert`][Self::insert] for each
        /// key-value pair from `other` in order, which means that for keys that
        /// already exist in `self`, their value is updated in the current position.
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// // Note: Key (3) is present in both maps.
        /// let mut a = IndexMap::from([(3, "c"), (2, "b"), (1, "a")]);
        /// let mut b = IndexMap::from([(3, "d"), (4, "e"), (5, "f")]);
        /// let old_capacity = b.capacity();
        ///
        /// a.append(&mut b);
        ///
        /// assert_eq!(a.len(), 5);
        /// assert_eq!(b.len(), 0);
        /// assert_eq!(b.capacity(), old_capacity);
        ///
        /// assert!(a.keys().eq(&[3, 2, 1, 4, 5]));
        /// assert_eq!(a[&3], "d"); // "c" was overwritten.
        /// ```
        pub fn append<S2>(&mut self, other: &mut IndexMap<K, V, S2>) {
            self.extend(other.drain(..));
        }
    }
    impl<K, V, S> IndexMap<K, V, S>
    where
        S: BuildHasher,
    {
        pub(crate) fn hash<Q: ?Sized + Hash>(&self, key: &Q) -> HashValue {
            let h = self.hash_builder.hash_one(key);
            HashValue(h as usize)
        }
        /// Return `true` if an equivalent to `key` exists in the map.
        ///
        /// Computes in **O(1)** time (average).
        pub fn contains_key<Q>(&self, key: &Q) -> bool
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            self.get_index_of(key).is_some()
        }
        /// Return a reference to the stored value for `key`, if it is present,
        /// else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &self.as_entries()[i];
                Some(&entry.value)
            } else {
                None
            }
        }
        /// Return references to the stored key-value pair for the lookup `key`,
        /// if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &self.as_entries()[i];
                Some((&entry.key, &entry.value))
            } else {
                None
            }
        }
        /// Return the index with references to the stored key-value pair for the
        /// lookup `key`, if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_full<Q>(&self, key: &Q) -> Option<(usize, &K, &V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &self.as_entries()[i];
                Some((i, &entry.key, &entry.value))
            } else {
                None
            }
        }
        /// Return the item index for `key`, if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            match self.as_entries() {
                [] => None,
                [x] => key.equivalent(&x.key).then_some(0),
                _ => {
                    let hash = self.hash(key);
                    self.core.get_index_of(hash, key)
                }
            }
        }
        /// Return a mutable reference to the stored value for `key`,
        /// if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &mut self.as_entries_mut()[i];
                Some(&mut entry.value)
            } else {
                None
            }
        }
        /// Return a reference and mutable references to the stored key-value pair
        /// for the lookup `key`, if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_key_value_mut<Q>(&mut self, key: &Q) -> Option<(&K, &mut V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &mut self.as_entries_mut()[i];
                Some((&entry.key, &mut entry.value))
            } else {
                None
            }
        }
        /// Return the index with a reference and mutable reference to the stored
        /// key-value pair for the lookup `key`, if it is present, else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_full_mut<Q>(&mut self, key: &Q) -> Option<(usize, &K, &mut V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            if let Some(i) = self.get_index_of(key) {
                let entry = &mut self.as_entries_mut()[i];
                Some((i, &entry.key, &mut entry.value))
            } else {
                None
            }
        }
        /// Return the values for `N` keys. If any key is duplicated, this function will panic.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut map = indexmap::IndexMap::from([(1, 'a'), (3, 'b'), (2, 'c')]);
        /// assert_eq!(map.get_disjoint_mut([&2, &1]), [Some(&mut 'c'), Some(&mut 'a')]);
        /// ```
        pub fn get_disjoint_mut<Q, const N: usize>(
            &mut self,
            keys: [&Q; N],
        ) -> [Option<&mut V>; N]
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            let indices = keys.map(|key| self.get_index_of(key));
            match self.as_mut_slice().get_disjoint_opt_mut(indices) {
                Err(GetDisjointMutError::IndexOutOfBounds) => {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "internal error: entered unreachable code: {0}",
                                format_args!(
                                    "Internal error: indices should never be OOB as we got them from get_index_of",
                                ),
                            ),
                        );
                    };
                }
                Err(GetDisjointMutError::OverlappingIndices) => {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("duplicate keys found"),
                        );
                    };
                }
                Ok(key_values) => key_values.map(|kv_opt| kv_opt.map(|kv| kv.1)),
            }
        }
        /// Remove the key-value pair equivalent to `key` and return
        /// its value.
        ///
        /// **NOTE:** This is equivalent to [`.swap_remove(key)`][Self::swap_remove], replacing this
        /// entry's position with the last element, and it is deprecated in favor of calling that
        /// explicitly. If you need to preserve the relative order of the keys in the map, use
        /// [`.shift_remove(key)`][Self::shift_remove] instead.
        #[deprecated(
            note = "`remove` disrupts the map order -- \
        use `swap_remove` or `shift_remove` for explicit behavior."
        )]
        pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            self.swap_remove(key)
        }
        /// Remove and return the key-value pair equivalent to `key`.
        ///
        /// **NOTE:** This is equivalent to [`.swap_remove_entry(key)`][Self::swap_remove_entry],
        /// replacing this entry's position with the last element, and it is deprecated in favor of
        /// calling that explicitly. If you need to preserve the relative order of the keys in the map,
        /// use [`.shift_remove_entry(key)`][Self::shift_remove_entry] instead.
        #[deprecated(
            note = "`remove_entry` disrupts the map order -- \
        use `swap_remove_entry` or `shift_remove_entry` for explicit behavior."
        )]
        pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            self.swap_remove_entry(key)
        }
        /// Remove the key-value pair equivalent to `key` and return
        /// its value.
        ///
        /// Like [`Vec::swap_remove`], the pair is removed by swapping it with the
        /// last element of the map and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<V>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            self.swap_remove_full(key).map(third)
        }
        /// Remove and return the key-value pair equivalent to `key`.
        ///
        /// Like [`Vec::swap_remove`], the pair is removed by swapping it with the
        /// last element of the map and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            match self.swap_remove_full(key) {
                Some((_, key, value)) => Some((key, value)),
                None => None,
            }
        }
        /// Remove the key-value pair equivalent to `key` and return it and
        /// the index it had.
        ///
        /// Like [`Vec::swap_remove`], the pair is removed by swapping it with the
        /// last element of the map and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove_full<Q>(&mut self, key: &Q) -> Option<(usize, K, V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            match self.as_entries() {
                [x] if key.equivalent(&x.key) => {
                    let (k, v) = self.core.pop()?;
                    Some((0, k, v))
                }
                [_] | [] => None,
                _ => {
                    let hash = self.hash(key);
                    self.core.swap_remove_full(hash, key)
                }
            }
        }
        /// Remove the key-value pair equivalent to `key` and return
        /// its value.
        ///
        /// Like [`Vec::remove`], the pair is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove<Q>(&mut self, key: &Q) -> Option<V>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            self.shift_remove_full(key).map(third)
        }
        /// Remove and return the key-value pair equivalent to `key`.
        ///
        /// Like [`Vec::remove`], the pair is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            match self.shift_remove_full(key) {
                Some((_, key, value)) => Some((key, value)),
                None => None,
            }
        }
        /// Remove the key-value pair equivalent to `key` and return it and
        /// the index it had.
        ///
        /// Like [`Vec::remove`], the pair is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `None` if `key` is not in map.
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove_full<Q>(&mut self, key: &Q) -> Option<(usize, K, V)>
        where
            Q: ?Sized + Hash + Equivalent<K>,
        {
            match self.as_entries() {
                [x] if key.equivalent(&x.key) => {
                    let (k, v) = self.core.pop()?;
                    Some((0, k, v))
                }
                [_] | [] => None,
                _ => {
                    let hash = self.hash(key);
                    self.core.shift_remove_full(hash, key)
                }
            }
        }
    }
    impl<K, V, S> IndexMap<K, V, S> {
        /// Remove the last key-value pair
        ///
        /// This preserves the order of the remaining elements.
        ///
        /// Computes in **O(1)** time (average).
        #[doc(alias = "pop_last")]
        pub fn pop(&mut self) -> Option<(K, V)> {
            self.core.pop()
        }
        /// Removes and returns the last key-value pair from a map if the predicate
        /// returns `true`, or [`None`] if the predicate returns false or the map
        /// is empty (the predicate will not be called in that case).
        ///
        /// This preserves the order of the remaining elements.
        ///
        /// Computes in **O(1)** time (average).
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// let init = [(1, 'a'), (2, 'b'), (3, 'c'), (4, 'd')];
        /// let mut map = IndexMap::from(init);
        /// let pred = |key: &i32, _value: &mut char| *key % 2 == 0;
        ///
        /// assert_eq!(map.pop_if(pred), Some((4, 'd')));
        /// assert_eq!(map.as_slice(), &init[..3]);
        /// assert_eq!(map.pop_if(pred), None);
        /// ```
        pub fn pop_if(
            &mut self,
            predicate: impl FnOnce(&K, &mut V) -> bool,
        ) -> Option<(K, V)> {
            let (last_key, last_value) = self.last_mut()?;
            if predicate(last_key, last_value) { self.core.pop() } else { None }
        }
        /// Scan through each key-value pair in the map and keep those where the
        /// closure `keep` returns `true`.
        ///
        /// The elements are visited in order, and remaining elements keep their
        /// order.
        ///
        /// Computes in **O(n)** time (average).
        pub fn retain<F>(&mut self, mut keep: F)
        where
            F: FnMut(&K, &mut V) -> bool,
        {
            self.core.retain_in_order(move |k, v| keep(k, v));
        }
        /// Sort the map's key-value pairs by the default ordering of the keys.
        ///
        /// This is a stable sort -- but equivalent keys should not normally coexist in
        /// a map at all, so [`sort_unstable_keys`][Self::sort_unstable_keys] is preferred
        /// because it is generally faster and doesn't allocate auxiliary memory.
        ///
        /// See [`sort_by`](Self::sort_by) for details.
        pub fn sort_keys(&mut self)
        where
            K: Ord,
        {
            self.with_entries(move |entries| {
                entries.sort_by(move |a, b| K::cmp(&a.key, &b.key));
            });
        }
        /// Sort the map's key-value pairs in place using the comparison
        /// function `cmp`.
        ///
        /// The comparison function receives two key and value pairs to compare (you
        /// can sort by keys or values or their combination as needed).
        ///
        /// Computes in **O(n log n + c)** time and **O(n)** space where *n* is
        /// the length of the map and *c* the capacity. The sort is stable.
        pub fn sort_by<F>(&mut self, mut cmp: F)
        where
            F: FnMut(&K, &V, &K, &V) -> Ordering,
        {
            self.with_entries(move |entries| {
                entries.sort_by(move |a, b| cmp(&a.key, &a.value, &b.key, &b.value));
            });
        }
        /// Sort the key-value pairs of the map and return a by-value iterator of
        /// the key-value pairs with the result.
        ///
        /// The sort is stable.
        pub fn sorted_by<F>(self, mut cmp: F) -> IntoIter<K, V>
        where
            F: FnMut(&K, &V, &K, &V) -> Ordering,
        {
            let mut entries = self.into_entries();
            entries.sort_by(move |a, b| cmp(&a.key, &a.value, &b.key, &b.value));
            IntoIter::new(entries)
        }
        /// Sort the map's key-value pairs in place using a sort-key extraction function.
        ///
        /// Computes in **O(n log n + c)** time and **O(n)** space where *n* is
        /// the length of the map and *c* the capacity. The sort is stable.
        pub fn sort_by_key<T, F>(&mut self, mut sort_key: F)
        where
            T: Ord,
            F: FnMut(&K, &V) -> T,
        {
            self.with_entries(move |entries| {
                entries.sort_by_key(move |a| sort_key(&a.key, &a.value));
            });
        }
        /// Sort the map's key-value pairs by the default ordering of the keys, but
        /// may not preserve the order of equal elements.
        ///
        /// See [`sort_unstable_by`](Self::sort_unstable_by) for details.
        pub fn sort_unstable_keys(&mut self)
        where
            K: Ord,
        {
            self.with_entries(move |entries| {
                entries.sort_unstable_by(move |a, b| K::cmp(&a.key, &b.key));
            });
        }
        /// Sort the map's key-value pairs in place using the comparison function `cmp`, but
        /// may not preserve the order of equal elements.
        ///
        /// The comparison function receives two key and value pairs to compare (you
        /// can sort by keys or values or their combination as needed).
        ///
        /// Computes in **O(n log n + c)** time where *n* is
        /// the length of the map and *c* is the capacity. The sort is unstable.
        pub fn sort_unstable_by<F>(&mut self, mut cmp: F)
        where
            F: FnMut(&K, &V, &K, &V) -> Ordering,
        {
            self.with_entries(move |entries| {
                entries
                    .sort_unstable_by(move |a, b| cmp(
                        &a.key,
                        &a.value,
                        &b.key,
                        &b.value,
                    ));
            });
        }
        /// Sort the key-value pairs of the map and return a by-value iterator of
        /// the key-value pairs with the result.
        ///
        /// The sort is unstable.
        #[inline]
        pub fn sorted_unstable_by<F>(self, mut cmp: F) -> IntoIter<K, V>
        where
            F: FnMut(&K, &V, &K, &V) -> Ordering,
        {
            let mut entries = self.into_entries();
            entries
                .sort_unstable_by(move |a, b| cmp(&a.key, &a.value, &b.key, &b.value));
            IntoIter::new(entries)
        }
        /// Sort the map's key-value pairs in place using a sort-key extraction function.
        ///
        /// Computes in **O(n log n + c)** time where *n* is
        /// the length of the map and *c* is the capacity. The sort is unstable.
        pub fn sort_unstable_by_key<T, F>(&mut self, mut sort_key: F)
        where
            T: Ord,
            F: FnMut(&K, &V) -> T,
        {
            self.with_entries(move |entries| {
                entries.sort_unstable_by_key(move |a| sort_key(&a.key, &a.value));
            });
        }
        /// Sort the map's key-value pairs in place using a sort-key extraction function.
        ///
        /// During sorting, the function is called at most once per entry, by using temporary storage
        /// to remember the results of its evaluation. The order of calls to the function is
        /// unspecified and may change between versions of `indexmap` or the standard library.
        ///
        /// Computes in **O(m n + n log n + c)** time () and **O(n)** space, where the function is
        /// **O(m)**, *n* is the length of the map, and *c* the capacity. The sort is stable.
        pub fn sort_by_cached_key<T, F>(&mut self, mut sort_key: F)
        where
            T: Ord,
            F: FnMut(&K, &V) -> T,
        {
            self.with_entries(move |entries| {
                entries.sort_by_cached_key(move |a| sort_key(&a.key, &a.value));
            });
        }
        /// Search over a sorted map for a key.
        ///
        /// Returns the position where that key is present, or the position where it can be inserted to
        /// maintain the sort. See [`slice::binary_search`] for more details.
        ///
        /// Computes in **O(log(n))** time, which is notably less scalable than looking the key up
        /// using [`get_index_of`][IndexMap::get_index_of], but this can also position missing keys.
        pub fn binary_search_keys(&self, x: &K) -> Result<usize, usize>
        where
            K: Ord,
        {
            self.as_slice().binary_search_keys(x)
        }
        /// Search over a sorted map with a comparator function.
        ///
        /// Returns the position where that value is present, or the position where it can be inserted
        /// to maintain the sort. See [`slice::binary_search_by`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[inline]
        pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
        where
            F: FnMut(&'a K, &'a V) -> Ordering,
        {
            self.as_slice().binary_search_by(f)
        }
        /// Search over a sorted map with an extraction function.
        ///
        /// Returns the position where that value is present, or the position where it can be inserted
        /// to maintain the sort. See [`slice::binary_search_by_key`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[inline]
        pub fn binary_search_by_key<'a, B, F>(
            &'a self,
            b: &B,
            f: F,
        ) -> Result<usize, usize>
        where
            F: FnMut(&'a K, &'a V) -> B,
            B: Ord,
        {
            self.as_slice().binary_search_by_key(b, f)
        }
        /// Checks if the keys of this map are sorted.
        #[inline]
        pub fn is_sorted(&self) -> bool
        where
            K: PartialOrd,
        {
            self.as_slice().is_sorted()
        }
        /// Checks if this map is sorted using the given comparator function.
        #[inline]
        pub fn is_sorted_by<'a, F>(&'a self, cmp: F) -> bool
        where
            F: FnMut(&'a K, &'a V, &'a K, &'a V) -> bool,
        {
            self.as_slice().is_sorted_by(cmp)
        }
        /// Checks if this map is sorted using the given sort-key function.
        #[inline]
        pub fn is_sorted_by_key<'a, F, T>(&'a self, sort_key: F) -> bool
        where
            F: FnMut(&'a K, &'a V) -> T,
            T: PartialOrd,
        {
            self.as_slice().is_sorted_by_key(sort_key)
        }
        /// Returns the index of the partition point of a sorted map according to the given predicate
        /// (the index of the first element of the second partition).
        ///
        /// See [`slice::partition_point`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[must_use]
        pub fn partition_point<P>(&self, pred: P) -> usize
        where
            P: FnMut(&K, &V) -> bool,
        {
            self.as_slice().partition_point(pred)
        }
        /// Reverses the order of the map's key-value pairs in place.
        ///
        /// Computes in **O(n)** time and **O(1)** space.
        pub fn reverse(&mut self) {
            self.core.reverse()
        }
        /// Returns a slice of all the key-value pairs in the map.
        ///
        /// Computes in **O(1)** time.
        pub fn as_slice(&self) -> &Slice<K, V> {
            Slice::from_slice(self.as_entries())
        }
        /// Returns a mutable slice of all the key-value pairs in the map.
        ///
        /// Computes in **O(1)** time.
        pub fn as_mut_slice(&mut self) -> &mut Slice<K, V> {
            Slice::from_mut_slice(self.as_entries_mut())
        }
        /// Converts into a boxed slice of all the key-value pairs in the map.
        ///
        /// Note that this will drop the inner hash table and any excess capacity.
        pub fn into_boxed_slice(self) -> Box<Slice<K, V>> {
            Slice::from_boxed(self.into_entries().into_boxed_slice())
        }
        /// Get a key-value pair by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
            self.as_entries().get(index).map(Bucket::refs)
        }
        /// Get a key-value pair by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
            self.as_entries_mut().get_mut(index).map(Bucket::ref_mut)
        }
        /// Get an entry in the map by index for in-place manipulation.
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_index_entry(
            &mut self,
            index: usize,
        ) -> Option<IndexedEntry<'_, K, V>> {
            IndexedEntry::new(&mut self.core, index)
        }
        /// Get an array of `N` key-value pairs by `N` indices
        ///
        /// Valid indices are *0 <= index < self.len()* and each index needs to be unique.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut map = indexmap::IndexMap::from([(1, 'a'), (3, 'b'), (2, 'c')]);
        /// assert_eq!(map.get_disjoint_indices_mut([2, 0]), Ok([(&2, &mut 'c'), (&1, &mut 'a')]));
        /// ```
        pub fn get_disjoint_indices_mut<const N: usize>(
            &mut self,
            indices: [usize; N],
        ) -> Result<[(&K, &mut V); N], GetDisjointMutError> {
            self.as_mut_slice().get_disjoint_mut(indices)
        }
        /// Returns a slice of key-value pairs in the given range of indices.
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_range<R: RangeBounds<usize>>(
            &self,
            range: R,
        ) -> Option<&Slice<K, V>> {
            let entries = self.as_entries();
            let range = try_simplify_range(range, entries.len())?;
            entries.get(range).map(Slice::from_slice)
        }
        /// Returns a mutable slice of key-value pairs in the given range of indices.
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_range_mut<R: RangeBounds<usize>>(
            &mut self,
            range: R,
        ) -> Option<&mut Slice<K, V>> {
            let entries = self.as_entries_mut();
            let range = try_simplify_range(range, entries.len())?;
            entries.get_mut(range).map(Slice::from_mut_slice)
        }
        /// Get the first key-value pair
        ///
        /// Computes in **O(1)** time.
        #[doc(alias = "first_key_value")]
        pub fn first(&self) -> Option<(&K, &V)> {
            self.as_entries().first().map(Bucket::refs)
        }
        /// Get the first key-value pair, with mutable access to the value
        ///
        /// Computes in **O(1)** time.
        pub fn first_mut(&mut self) -> Option<(&K, &mut V)> {
            self.as_entries_mut().first_mut().map(Bucket::ref_mut)
        }
        /// Get the first entry in the map for in-place manipulation.
        ///
        /// Computes in **O(1)** time.
        pub fn first_entry(&mut self) -> Option<IndexedEntry<'_, K, V>> {
            self.get_index_entry(0)
        }
        /// Get the last key-value pair
        ///
        /// Computes in **O(1)** time.
        #[doc(alias = "last_key_value")]
        pub fn last(&self) -> Option<(&K, &V)> {
            self.as_entries().last().map(Bucket::refs)
        }
        /// Get the last key-value pair, with mutable access to the value
        ///
        /// Computes in **O(1)** time.
        pub fn last_mut(&mut self) -> Option<(&K, &mut V)> {
            self.as_entries_mut().last_mut().map(Bucket::ref_mut)
        }
        /// Get the last entry in the map for in-place manipulation.
        ///
        /// Computes in **O(1)** time.
        pub fn last_entry(&mut self) -> Option<IndexedEntry<'_, K, V>> {
            self.get_index_entry(self.len().checked_sub(1)?)
        }
        /// Remove the key-value pair by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Like [`Vec::swap_remove`], the pair is removed by swapping it with the
        /// last element of the map and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove_index(&mut self, index: usize) -> Option<(K, V)> {
            self.core.swap_remove_index(index)
        }
        /// Remove the key-value pair by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Like [`Vec::remove`], the pair is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove_index(&mut self, index: usize) -> Option<(K, V)> {
            self.core.shift_remove_index(index)
        }
        /// Moves the position of a key-value pair from one index to another
        /// by shifting all other pairs in-between.
        ///
        /// * If `from < to`, the other pairs will shift down while the targeted pair moves up.
        /// * If `from > to`, the other pairs will shift up while the targeted pair moves down.
        ///
        /// ***Panics*** if `from` or `to` are out of bounds.
        ///
        /// Computes in **O(n)** time (average).
        #[track_caller]
        pub fn move_index(&mut self, from: usize, to: usize) {
            self.core.move_index(from, to)
        }
        /// Swaps the position of two key-value pairs in the map.
        ///
        /// ***Panics*** if `a` or `b` are out of bounds.
        ///
        /// Computes in **O(1)** time (average).
        #[track_caller]
        pub fn swap_indices(&mut self, a: usize, b: usize) {
            self.core.swap_indices(a, b)
        }
    }
    /// Access [`IndexMap`] values corresponding to a key.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
    ///     map.insert(word.to_lowercase(), word.to_uppercase());
    /// }
    /// assert_eq!(map["lorem"], "LOREM");
    /// assert_eq!(map["ipsum"], "IPSUM");
    /// ```
    ///
    /// ```should_panic
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// map.insert("foo", 1);
    /// println!("{:?}", map["bar"]); // panics!
    /// ```
    impl<K, V, Q: ?Sized, S> Index<&Q> for IndexMap<K, V, S>
    where
        Q: Hash + Equivalent<K>,
        S: BuildHasher,
    {
        type Output = V;
        /// Returns a reference to the value corresponding to the supplied `key`.
        ///
        /// ***Panics*** if `key` is not present in the map.
        fn index(&self, key: &Q) -> &V {
            self.get(key).expect("no entry found for key")
        }
    }
    /// Access [`IndexMap`] values corresponding to a key.
    ///
    /// Mutable indexing allows changing / updating values of key-value
    /// pairs that are already present.
    ///
    /// You can **not** insert new pairs with index syntax, use `.insert()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
    ///     map.insert(word.to_lowercase(), word.to_string());
    /// }
    /// let lorem = &mut map["lorem"];
    /// assert_eq!(lorem, "Lorem");
    /// lorem.retain(char::is_lowercase);
    /// assert_eq!(map["lorem"], "orem");
    /// ```
    ///
    /// ```should_panic
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// map.insert("foo", 1);
    /// map["bar"] = 1; // panics!
    /// ```
    impl<K, V, Q: ?Sized, S> IndexMut<&Q> for IndexMap<K, V, S>
    where
        Q: Hash + Equivalent<K>,
        S: BuildHasher,
    {
        /// Returns a mutable reference to the value corresponding to the supplied `key`.
        ///
        /// ***Panics*** if `key` is not present in the map.
        fn index_mut(&mut self, key: &Q) -> &mut V {
            self.get_mut(key).expect("no entry found for key")
        }
    }
    /// Access [`IndexMap`] values at indexed positions.
    ///
    /// See [`Index<usize> for Keys`][keys] to access a map's keys instead.
    ///
    /// [keys]: Keys#impl-Index<usize>-for-Keys<'a,+K,+V>
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
    ///     map.insert(word.to_lowercase(), word.to_uppercase());
    /// }
    /// assert_eq!(map[0], "LOREM");
    /// assert_eq!(map[1], "IPSUM");
    /// map.reverse();
    /// assert_eq!(map[0], "AMET");
    /// assert_eq!(map[1], "SIT");
    /// map.sort_keys();
    /// assert_eq!(map[0], "AMET");
    /// assert_eq!(map[1], "DOLOR");
    /// ```
    ///
    /// ```should_panic
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// map.insert("foo", 1);
    /// println!("{:?}", map[10]); // panics!
    /// ```
    impl<K, V, S> Index<usize> for IndexMap<K, V, S> {
        type Output = V;
        /// Returns a reference to the value at the supplied `index`.
        ///
        /// ***Panics*** if `index` is out of bounds.
        fn index(&self, index: usize) -> &V {
            if let Some((_, value)) = self.get_index(index) {
                value
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "index out of bounds: the len is {0} but the index is {1}",
                            self.len(),
                            index,
                        ),
                    );
                };
            }
        }
    }
    /// Access [`IndexMap`] values at indexed positions.
    ///
    /// Mutable indexing allows changing / updating indexed values
    /// that are already present.
    ///
    /// You can **not** insert new values with index syntax -- use [`.insert()`][IndexMap::insert].
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
    ///     map.insert(word.to_lowercase(), word.to_string());
    /// }
    /// let lorem = &mut map[0];
    /// assert_eq!(lorem, "Lorem");
    /// lorem.retain(char::is_lowercase);
    /// assert_eq!(map["lorem"], "orem");
    /// ```
    ///
    /// ```should_panic
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// map.insert("foo", 1);
    /// map[10] = 1; // panics!
    /// ```
    impl<K, V, S> IndexMut<usize> for IndexMap<K, V, S> {
        /// Returns a mutable reference to the value at the supplied `index`.
        ///
        /// ***Panics*** if `index` is out of bounds.
        fn index_mut(&mut self, index: usize) -> &mut V {
            let len: usize = self.len();
            if let Some((_, value)) = self.get_index_mut(index) {
                value
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "index out of bounds: the len is {0} but the index is {1}",
                            len,
                            index,
                        ),
                    );
                };
            }
        }
    }
    impl<K, V, S> FromIterator<(K, V)> for IndexMap<K, V, S>
    where
        K: Hash + Eq,
        S: BuildHasher + Default,
    {
        /// Create an `IndexMap` from the sequence of key-value pairs in the
        /// iterable.
        ///
        /// `from_iter` uses the same logic as `extend`. See
        /// [`extend`][IndexMap::extend] for more details.
        fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
            let iter = iterable.into_iter();
            let (low, _) = iter.size_hint();
            let mut map = Self::with_capacity_and_hasher(low, <_>::default());
            map.extend(iter);
            map
        }
    }
    impl<K, V, const N: usize> From<[(K, V); N]> for IndexMap<K, V, RandomState>
    where
        K: Hash + Eq,
    {
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexMap;
        ///
        /// let map1 = IndexMap::from([(1, 2), (3, 4)]);
        /// let map2: IndexMap<_, _> = [(1, 2), (3, 4)].into();
        /// assert_eq!(map1, map2);
        /// ```
        fn from(arr: [(K, V); N]) -> Self {
            Self::from_iter(arr)
        }
    }
    impl<K, V, S> Extend<(K, V)> for IndexMap<K, V, S>
    where
        K: Hash + Eq,
        S: BuildHasher,
    {
        /// Extend the map with all key-value pairs in the iterable.
        ///
        /// This is equivalent to calling [`insert`][IndexMap::insert] for each of
        /// them in order, which means that for keys that already existed
        /// in the map, their value is updated but it keeps the existing order.
        ///
        /// New keys are inserted in the order they appear in the sequence. If
        /// equivalents of a key occur more than once, the last corresponding value
        /// prevails.
        fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iterable: I) {
            let iter = iterable.into_iter();
            let (lower_len, _) = iter.size_hint();
            let reserve = if self.is_empty() {
                lower_len
            } else {
                lower_len.div_ceil(2)
            };
            self.reserve(reserve);
            iter.for_each(move |(k, v)| {
                self.insert(k, v);
            });
        }
    }
    impl<'a, K, V, S> Extend<(&'a K, &'a V)> for IndexMap<K, V, S>
    where
        K: Hash + Eq + Copy,
        V: Copy,
        S: BuildHasher,
    {
        /// Extend the map with all key-value pairs in the iterable.
        ///
        /// See the first extend method for more details.
        fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iterable: I) {
            self.extend(iterable.into_iter().map(|(&key, &value)| (key, value)));
        }
    }
    impl<K, V, S> Default for IndexMap<K, V, S>
    where
        S: Default,
    {
        /// Return an empty [`IndexMap`]
        fn default() -> Self {
            Self::with_capacity_and_hasher(0, S::default())
        }
    }
    impl<K, V1, S1, V2, S2> PartialEq<IndexMap<K, V2, S2>> for IndexMap<K, V1, S1>
    where
        K: Hash + Eq,
        V1: PartialEq<V2>,
        S1: BuildHasher,
        S2: BuildHasher,
    {
        fn eq(&self, other: &IndexMap<K, V2, S2>) -> bool {
            if self.len() != other.len() {
                return false;
            }
            self.iter()
                .all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
    impl<K, V, S> Eq for IndexMap<K, V, S>
    where
        K: Eq + Hash,
        V: Eq,
        S: BuildHasher,
    {}
}
pub mod set {
    //! A hash set implemented using [`IndexMap`]
    mod iter {
        use super::{Bucket, IndexSet, Slice};
        use crate::inner::{Core, ExtractCore};
        use alloc::vec::{self, Vec};
        use core::fmt;
        use core::hash::{BuildHasher, Hash};
        use core::iter::{Chain, FusedIterator};
        use core::ops::RangeBounds;
        use core::slice::Iter as SliceIter;
        impl<'a, T, S> IntoIterator for &'a IndexSet<T, S> {
            type Item = &'a T;
            type IntoIter = Iter<'a, T>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl<T, S> IntoIterator for IndexSet<T, S> {
            type Item = T;
            type IntoIter = IntoIter<T>;
            fn into_iter(self) -> Self::IntoIter {
                IntoIter::new(self.into_entries())
            }
        }
        /// An iterator over the items of an [`IndexSet`].
        ///
        /// This `struct` is created by the [`IndexSet::iter`] method.
        /// See its documentation for more.
        pub struct Iter<'a, T> {
            iter: SliceIter<'a, Bucket<T>>,
        }
        impl<'a, T> Iter<'a, T> {
            pub(super) fn new(entries: &'a [Bucket<T>]) -> Self {
                Self { iter: entries.iter() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &'a Slice<T> {
                Slice::from_slice(self.iter.as_slice())
            }
        }
        impl<'a, T> Iterator for Iter<'a, T> {
            type Item = &'a T;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key_ref)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key_ref)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key_ref).collect()
            }
        }
        impl<T> DoubleEndedIterator for Iter<'_, T> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key_ref)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key_ref)
            }
        }
        impl<T> ExactSizeIterator for Iter<'_, T> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<T> FusedIterator for Iter<'_, T> {}
        impl<T> Clone for Iter<'_, T> {
            fn clone(&self) -> Self {
                Iter { iter: self.iter.clone() }
            }
        }
        impl<T: fmt::Debug> fmt::Debug for Iter<'_, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        impl<T> Default for Iter<'_, T> {
            fn default() -> Self {
                Self { iter: [].iter() }
            }
        }
        /// An owning iterator over the items of an [`IndexSet`].
        ///
        /// This `struct` is created by the [`IndexSet::into_iter`] method
        /// (provided by the [`IntoIterator`] trait). See its documentation for more.
        pub struct IntoIter<T> {
            iter: vec::IntoIter<Bucket<T>>,
        }
        #[automatically_derived]
        impl<T: ::core::clone::Clone> ::core::clone::Clone for IntoIter<T> {
            #[inline]
            fn clone(&self) -> IntoIter<T> {
                IntoIter {
                    iter: ::core::clone::Clone::clone(&self.iter),
                }
            }
        }
        impl<T> IntoIter<T> {
            pub(super) fn new(entries: Vec<Bucket<T>>) -> Self {
                Self { iter: entries.into_iter() }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<T> {
                Slice::from_slice(self.iter.as_slice())
            }
        }
        impl<T> Iterator for IntoIter<T> {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key).collect()
            }
        }
        impl<T> DoubleEndedIterator for IntoIter<T> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key)
            }
        }
        impl<T> ExactSizeIterator for IntoIter<T> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<T> FusedIterator for IntoIter<T> {}
        impl<T: fmt::Debug> fmt::Debug for IntoIter<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::key_ref);
                f.debug_list().entries(iter).finish()
            }
        }
        impl<T> Default for IntoIter<T> {
            fn default() -> Self {
                Self {
                    iter: Vec::new().into_iter(),
                }
            }
        }
        /// A draining iterator over the items of an [`IndexSet`].
        ///
        /// This `struct` is created by the [`IndexSet::drain`] method.
        /// See its documentation for more.
        pub struct Drain<'a, T> {
            iter: vec::Drain<'a, Bucket<T>>,
        }
        impl<'a, T> Drain<'a, T> {
            pub(super) fn new(iter: vec::Drain<'a, Bucket<T>>) -> Self {
                Self { iter }
            }
            /// Returns a slice of the remaining entries in the iterator.
            pub fn as_slice(&self) -> &Slice<T> {
                Slice::from_slice(self.iter.as_slice())
            }
        }
        impl<T> Iterator for Drain<'_, T> {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(Bucket::key)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn count(self) -> usize {
                self.iter.len()
            }
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth(n).map(Bucket::key)
            }
            fn last(mut self) -> Option<Self::Item> {
                self.next_back()
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                self.iter.map(Bucket::key).collect()
            }
        }
        impl<T> DoubleEndedIterator for Drain<'_, T> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(Bucket::key)
            }
            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                self.iter.nth_back(n).map(Bucket::key)
            }
        }
        impl<T> ExactSizeIterator for Drain<'_, T> {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<T> FusedIterator for Drain<'_, T> {}
        impl<T: fmt::Debug> fmt::Debug for Drain<'_, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let iter = self.iter.as_slice().iter().map(Bucket::key_ref);
                f.debug_list().entries(iter).finish()
            }
        }
        /// A lazy iterator producing elements in the difference of [`IndexSet`]s.
        ///
        /// This `struct` is created by the [`IndexSet::difference`] method.
        /// See its documentation for more.
        pub struct Difference<'a, T, S> {
            iter: Iter<'a, T>,
            other: &'a IndexSet<T, S>,
        }
        impl<'a, T, S> Difference<'a, T, S> {
            pub(super) fn new<S1>(
                set: &'a IndexSet<T, S1>,
                other: &'a IndexSet<T, S>,
            ) -> Self {
                Self { iter: set.iter(), other }
            }
        }
        impl<'a, T, S> Iterator for Difference<'a, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            type Item = &'a T;
            fn next(&mut self) -> Option<Self::Item> {
                while let Some(item) = self.iter.next() {
                    if !self.other.contains(item) {
                        return Some(item);
                    }
                }
                None
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (0, self.iter.size_hint().1)
            }
        }
        impl<T, S> DoubleEndedIterator for Difference<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                while let Some(item) = self.iter.next_back() {
                    if !self.other.contains(item) {
                        return Some(item);
                    }
                }
                None
            }
        }
        impl<T, S> FusedIterator for Difference<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {}
        impl<T, S> Clone for Difference<'_, T, S> {
            fn clone(&self) -> Self {
                Difference {
                    iter: self.iter.clone(),
                    ..*self
                }
            }
        }
        impl<T, S> fmt::Debug for Difference<'_, T, S>
        where
            T: fmt::Debug + Eq + Hash,
            S: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        /// A lazy iterator producing elements in the intersection of [`IndexSet`]s.
        ///
        /// This `struct` is created by the [`IndexSet::intersection`] method.
        /// See its documentation for more.
        pub struct Intersection<'a, T, S> {
            iter: Iter<'a, T>,
            other: &'a IndexSet<T, S>,
        }
        impl<'a, T, S> Intersection<'a, T, S> {
            pub(super) fn new<S1>(
                set: &'a IndexSet<T, S1>,
                other: &'a IndexSet<T, S>,
            ) -> Self {
                Self { iter: set.iter(), other }
            }
        }
        impl<'a, T, S> Iterator for Intersection<'a, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            type Item = &'a T;
            fn next(&mut self) -> Option<Self::Item> {
                while let Some(item) = self.iter.next() {
                    if self.other.contains(item) {
                        return Some(item);
                    }
                }
                None
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (0, self.iter.size_hint().1)
            }
        }
        impl<T, S> DoubleEndedIterator for Intersection<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                while let Some(item) = self.iter.next_back() {
                    if self.other.contains(item) {
                        return Some(item);
                    }
                }
                None
            }
        }
        impl<T, S> FusedIterator for Intersection<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {}
        impl<T, S> Clone for Intersection<'_, T, S> {
            fn clone(&self) -> Self {
                Intersection {
                    iter: self.iter.clone(),
                    ..*self
                }
            }
        }
        impl<T, S> fmt::Debug for Intersection<'_, T, S>
        where
            T: fmt::Debug + Eq + Hash,
            S: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        /// A lazy iterator producing elements in the symmetric difference of [`IndexSet`]s.
        ///
        /// This `struct` is created by the [`IndexSet::symmetric_difference`] method.
        /// See its documentation for more.
        pub struct SymmetricDifference<'a, T, S1, S2> {
            iter: Chain<Difference<'a, T, S2>, Difference<'a, T, S1>>,
        }
        impl<'a, T, S1, S2> SymmetricDifference<'a, T, S1, S2>
        where
            T: Eq + Hash,
            S1: BuildHasher,
            S2: BuildHasher,
        {
            pub(super) fn new(
                set1: &'a IndexSet<T, S1>,
                set2: &'a IndexSet<T, S2>,
            ) -> Self {
                let diff1 = set1.difference(set2);
                let diff2 = set2.difference(set1);
                Self { iter: diff1.chain(diff2) }
            }
        }
        impl<'a, T, S1, S2> Iterator for SymmetricDifference<'a, T, S1, S2>
        where
            T: Eq + Hash,
            S1: BuildHasher,
            S2: BuildHasher,
        {
            type Item = &'a T;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn fold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.iter.fold(init, f)
            }
        }
        impl<T, S1, S2> DoubleEndedIterator for SymmetricDifference<'_, T, S1, S2>
        where
            T: Eq + Hash,
            S1: BuildHasher,
            S2: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
            fn rfold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.iter.rfold(init, f)
            }
        }
        impl<T, S1, S2> FusedIterator for SymmetricDifference<'_, T, S1, S2>
        where
            T: Eq + Hash,
            S1: BuildHasher,
            S2: BuildHasher,
        {}
        impl<T, S1, S2> Clone for SymmetricDifference<'_, T, S1, S2> {
            fn clone(&self) -> Self {
                SymmetricDifference {
                    iter: self.iter.clone(),
                }
            }
        }
        impl<T, S1, S2> fmt::Debug for SymmetricDifference<'_, T, S1, S2>
        where
            T: fmt::Debug + Eq + Hash,
            S1: BuildHasher,
            S2: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        /// A lazy iterator producing elements in the union of [`IndexSet`]s.
        ///
        /// This `struct` is created by the [`IndexSet::union`] method.
        /// See its documentation for more.
        pub struct Union<'a, T, S> {
            iter: Chain<Iter<'a, T>, Difference<'a, T, S>>,
        }
        impl<'a, T, S> Union<'a, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            pub(super) fn new<S2>(
                set1: &'a IndexSet<T, S>,
                set2: &'a IndexSet<T, S2>,
            ) -> Self
            where
                S2: BuildHasher,
            {
                Self {
                    iter: set1.iter().chain(set2.difference(set1)),
                }
            }
        }
        impl<'a, T, S> Iterator for Union<'a, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            type Item = &'a T;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn fold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.iter.fold(init, f)
            }
        }
        impl<T, S> DoubleEndedIterator for Union<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
            fn rfold<B, F>(self, init: B, f: F) -> B
            where
                F: FnMut(B, Self::Item) -> B,
            {
                self.iter.rfold(init, f)
            }
        }
        impl<T, S> FusedIterator for Union<'_, T, S>
        where
            T: Eq + Hash,
            S: BuildHasher,
        {}
        impl<T, S> Clone for Union<'_, T, S> {
            fn clone(&self) -> Self {
                Union { iter: self.iter.clone() }
            }
        }
        impl<T, S> fmt::Debug for Union<'_, T, S>
        where
            T: fmt::Debug + Eq + Hash,
            S: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }
        /// A splicing iterator for `IndexSet`.
        ///
        /// This `struct` is created by [`IndexSet::splice()`].
        /// See its documentation for more.
        pub struct Splice<'a, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {
            iter: crate::map::Splice<'a, UnitValue<I>, T, (), S>,
        }
        impl<'a, I, T, S> Splice<'a, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {
            #[track_caller]
            pub(super) fn new<R>(
                set: &'a mut IndexSet<T, S>,
                range: R,
                replace_with: I,
            ) -> Self
            where
                R: RangeBounds<usize>,
            {
                Self {
                    iter: set.map.splice(range, UnitValue(replace_with)),
                }
            }
        }
        impl<I, T, S> Iterator for Splice<'_, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                Some(self.iter.next()?.0)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }
        impl<I, T, S> DoubleEndedIterator for Splice<'_, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                Some(self.iter.next_back()?.0)
            }
        }
        impl<I, T, S> ExactSizeIterator for Splice<'_, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {
            fn len(&self) -> usize {
                self.iter.len()
            }
        }
        impl<I, T, S> FusedIterator for Splice<'_, I, T, S>
        where
            I: Iterator<Item = T>,
            T: Hash + Eq,
            S: BuildHasher,
        {}
        struct UnitValue<I>(I);
        impl<I: Iterator> Iterator for UnitValue<I> {
            type Item = (I::Item, ());
            fn next(&mut self) -> Option<Self::Item> {
                self.0.next().map(|x| (x, ()))
            }
        }
        impl<I, T, S> fmt::Debug for Splice<'_, I, T, S>
        where
            I: fmt::Debug + Iterator<Item = T>,
            T: fmt::Debug + Hash + Eq,
            S: BuildHasher,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.iter, f)
            }
        }
        impl<I: fmt::Debug> fmt::Debug for UnitValue<I> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.0, f)
            }
        }
        /// An extracting iterator for `IndexSet`.
        ///
        /// This `struct` is created by [`IndexSet::extract_if()`].
        /// See its documentation for more.
        pub struct ExtractIf<'a, T, F> {
            inner: ExtractCore<'a, T, ()>,
            pred: F,
        }
        impl<T, F> ExtractIf<'_, T, F> {
            #[track_caller]
            pub(super) fn new<R>(
                core: &mut Core<T, ()>,
                range: R,
                pred: F,
            ) -> ExtractIf<'_, T, F>
            where
                R: RangeBounds<usize>,
                F: FnMut(&T) -> bool,
            {
                ExtractIf {
                    inner: core.extract(range),
                    pred,
                }
            }
        }
        impl<T, F> Iterator for ExtractIf<'_, T, F>
        where
            F: FnMut(&T) -> bool,
        {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                self.inner
                    .extract_if(|bucket| (self.pred)(bucket.key_ref()))
                    .map(Bucket::key)
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (0, Some(self.inner.remaining()))
            }
        }
        impl<T, F> FusedIterator for ExtractIf<'_, T, F>
        where
            F: FnMut(&T) -> bool,
        {}
        impl<T, F> fmt::Debug for ExtractIf<'_, T, F>
        where
            T: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("ExtractIf").finish_non_exhaustive()
            }
        }
    }
    mod mutable {
        use core::hash::{BuildHasher, Hash};
        use super::{Equivalent, IndexSet};
        use crate::map::MutableKeys;
        /// Opt-in mutable access to [`IndexSet`] values.
        ///
        /// These methods expose `&mut T`, mutable references to the value as it is stored
        /// in the set.
        /// You are allowed to modify the values in the set **if the modification
        /// does not change the value's hash and equality**.
        ///
        /// If values are modified erroneously, you can no longer look them up.
        /// This is sound (memory safe) but a logical error hazard (just like
        /// implementing `PartialEq`, `Eq`, or `Hash` incorrectly would be).
        ///
        /// `use` this trait to enable its methods for `IndexSet`.
        ///
        /// This trait is sealed and cannot be implemented for types outside this crate.
        #[expect(private_bounds)]
        pub trait MutableValues: Sealed {
            type Value;
            /// Return item index and mutable reference to the value
            ///
            /// Computes in **O(1)** time (average).
            fn get_full_mut2<Q>(
                &mut self,
                value: &Q,
            ) -> Option<(usize, &mut Self::Value)>
            where
                Q: ?Sized + Hash + Equivalent<Self::Value>;
            /// Return mutable reference to the value at an index.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            ///
            /// Computes in **O(1)** time.
            fn get_index_mut2(&mut self, index: usize) -> Option<&mut Self::Value>;
            /// Scan through each value in the set and keep those where the
            /// closure `keep` returns `true`.
            ///
            /// The values are visited in order, and remaining values keep their order.
            ///
            /// Computes in **O(n)** time (average).
            fn retain2<F>(&mut self, keep: F)
            where
                F: FnMut(&mut Self::Value) -> bool;
        }
        /// Opt-in mutable access to [`IndexSet`] values.
        ///
        /// See [`MutableValues`] for more information.
        impl<T, S> MutableValues for IndexSet<T, S>
        where
            S: BuildHasher,
        {
            type Value = T;
            fn get_full_mut2<Q>(&mut self, value: &Q) -> Option<(usize, &mut T)>
            where
                Q: ?Sized + Hash + Equivalent<T>,
            {
                match self.map.get_full_mut2(value) {
                    Some((index, value, ())) => Some((index, value)),
                    None => None,
                }
            }
            fn get_index_mut2(&mut self, index: usize) -> Option<&mut T> {
                match self.map.get_index_mut2(index) {
                    Some((value, ())) => Some(value),
                    None => None,
                }
            }
            fn retain2<F>(&mut self, mut keep: F)
            where
                F: FnMut(&mut T) -> bool,
            {
                self.map.retain2(move |value, ()| keep(value));
            }
        }
        trait Sealed {}
        impl<T, S> Sealed for IndexSet<T, S> {}
    }
    mod slice {
        use super::{Bucket, IndexSet, IntoIter, Iter};
        use crate::util::{slice_eq, try_simplify_range};
        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use core::cmp::Ordering;
        use core::fmt;
        use core::hash::{Hash, Hasher};
        use core::ops::{self, Bound, Index, RangeBounds};
        /// A dynamically-sized slice of values in an [`IndexSet`].
        ///
        /// This supports indexed operations much like a `[T]` slice,
        /// but not any hashed operations on the values.
        ///
        /// Unlike `IndexSet`, `Slice` does consider the order for [`PartialEq`]
        /// and [`Eq`], and it also implements [`PartialOrd`], [`Ord`], and [`Hash`].
        #[repr(transparent)]
        pub struct Slice<T> {
            pub(crate) entries: [Bucket<T>],
        }
        #[allow(unsafe_code)]
        impl<T> Slice<T> {
            pub(super) const fn from_slice(entries: &[Bucket<T>]) -> &Self {
                unsafe { &*(entries as *const [Bucket<T>] as *const Self) }
            }
            pub(super) fn from_boxed(entries: Box<[Bucket<T>]>) -> Box<Self> {
                unsafe { Box::from_raw(Box::into_raw(entries) as *mut Self) }
            }
            fn into_boxed(self: Box<Self>) -> Box<[Bucket<T>]> {
                unsafe { Box::from_raw(Box::into_raw(self) as *mut [Bucket<T>]) }
            }
        }
        impl<T> Slice<T> {
            pub(crate) fn into_entries(self: Box<Self>) -> Vec<Bucket<T>> {
                self.into_boxed().into_vec()
            }
            /// Returns an empty slice.
            pub const fn new<'a>() -> &'a Self {
                Self::from_slice(&[])
            }
            /// Return the number of elements in the set slice.
            pub const fn len(&self) -> usize {
                self.entries.len()
            }
            /// Returns true if the set slice contains no elements.
            pub const fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
            /// Get a value by index.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_index(&self, index: usize) -> Option<&T> {
                self.entries.get(index).map(Bucket::key_ref)
            }
            /// Returns a slice of values in the given range of indices.
            ///
            /// Valid indices are `0 <= index < self.len()`.
            pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Self> {
                let range = try_simplify_range(range, self.entries.len())?;
                self.entries.get(range).map(Self::from_slice)
            }
            /// Get the first value.
            pub fn first(&self) -> Option<&T> {
                self.entries.first().map(Bucket::key_ref)
            }
            /// Get the last value.
            pub fn last(&self) -> Option<&T> {
                self.entries.last().map(Bucket::key_ref)
            }
            /// Divides one slice into two at an index.
            ///
            /// ***Panics*** if `index > len`.
            /// For a non-panicking alternative see [`split_at_checked`][Self::split_at_checked].
            #[track_caller]
            pub fn split_at(&self, index: usize) -> (&Self, &Self) {
                let (first, second) = self.entries.split_at(index);
                (Self::from_slice(first), Self::from_slice(second))
            }
            /// Divides one slice into two at an index.
            ///
            /// Returns `None` if `index > len`.
            pub fn split_at_checked(&self, index: usize) -> Option<(&Self, &Self)> {
                let (first, second) = self.entries.split_at_checked(index)?;
                Some((Self::from_slice(first), Self::from_slice(second)))
            }
            /// Returns the first value and the rest of the slice,
            /// or `None` if it is empty.
            pub fn split_first(&self) -> Option<(&T, &Self)> {
                if let [first, rest @ ..] = &self.entries {
                    Some((&first.key, Self::from_slice(rest)))
                } else {
                    None
                }
            }
            /// Returns the last value and the rest of the slice,
            /// or `None` if it is empty.
            pub fn split_last(&self) -> Option<(&T, &Self)> {
                if let [rest @ .., last] = &self.entries {
                    Some((&last.key, Self::from_slice(rest)))
                } else {
                    None
                }
            }
            /// Return an iterator over the values of the set slice.
            pub fn iter(&self) -> Iter<'_, T> {
                Iter::new(&self.entries)
            }
            /// Search over a sorted set for a value.
            ///
            /// Returns the position where that value is present, or the position where it can be inserted
            /// to maintain the sort. See [`slice::binary_search`] for more details.
            ///
            /// Computes in **O(log(n))** time, which is notably less scalable than looking the value up in
            /// the set this is a slice from using [`IndexSet::get_index_of`], but this can also position
            /// missing values.
            pub fn binary_search(&self, x: &T) -> Result<usize, usize>
            where
                T: Ord,
            {
                self.binary_search_by(|p| p.cmp(x))
            }
            /// Search over a sorted set with a comparator function.
            ///
            /// Returns the position where that value is present, or the position where it can be inserted
            /// to maintain the sort. See [`slice::binary_search_by`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[inline]
            pub fn binary_search_by<'a, F>(&'a self, mut f: F) -> Result<usize, usize>
            where
                F: FnMut(&'a T) -> Ordering,
            {
                self.entries.binary_search_by(move |a| f(&a.key))
            }
            /// Search over a sorted set with an extraction function.
            ///
            /// Returns the position where that value is present, or the position where it can be inserted
            /// to maintain the sort. See [`slice::binary_search_by_key`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[inline]
            pub fn binary_search_by_key<'a, B, F>(
                &'a self,
                b: &B,
                mut f: F,
            ) -> Result<usize, usize>
            where
                F: FnMut(&'a T) -> B,
                B: Ord,
            {
                self.binary_search_by(|k| f(k).cmp(b))
            }
            /// Checks if the values of this slice are sorted.
            #[inline]
            pub fn is_sorted(&self) -> bool
            where
                T: PartialOrd,
            {
                self.entries.is_sorted_by(|a, b| a.key <= b.key)
            }
            /// Checks if this slice is sorted using the given comparator function.
            #[inline]
            pub fn is_sorted_by<'a, F>(&'a self, mut cmp: F) -> bool
            where
                F: FnMut(&'a T, &'a T) -> bool,
            {
                self.entries.is_sorted_by(move |a, b| cmp(&a.key, &b.key))
            }
            /// Checks if this slice is sorted using the given sort-key function.
            #[inline]
            pub fn is_sorted_by_key<'a, F, K>(&'a self, mut sort_key: F) -> bool
            where
                F: FnMut(&'a T) -> K,
                K: PartialOrd,
            {
                self.entries.is_sorted_by_key(move |a| sort_key(&a.key))
            }
            /// Returns the index of the partition point of a sorted set according to the given predicate
            /// (the index of the first element of the second partition).
            ///
            /// See [`slice::partition_point`] for more details.
            ///
            /// Computes in **O(log(n))** time.
            #[must_use]
            pub fn partition_point<P>(&self, mut pred: P) -> usize
            where
                P: FnMut(&T) -> bool,
            {
                self.entries.partition_point(move |a| pred(&a.key))
            }
        }
        impl<'a, T> IntoIterator for &'a Slice<T> {
            type IntoIter = Iter<'a, T>;
            type Item = &'a T;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
        impl<T> IntoIterator for Box<Slice<T>> {
            type IntoIter = IntoIter<T>;
            type Item = T;
            fn into_iter(self) -> Self::IntoIter {
                IntoIter::new(self.into_entries())
            }
        }
        impl<T> Default for &'_ Slice<T> {
            fn default() -> Self {
                Slice::from_slice(&[])
            }
        }
        impl<T> Default for Box<Slice<T>> {
            fn default() -> Self {
                Slice::from_boxed(Box::default())
            }
        }
        impl<T: Clone> Clone for Box<Slice<T>> {
            fn clone(&self) -> Self {
                Slice::from_boxed(self.entries.to_vec().into_boxed_slice())
            }
        }
        impl<T: Copy> From<&Slice<T>> for Box<Slice<T>> {
            fn from(slice: &Slice<T>) -> Self {
                Slice::from_boxed(Box::from(&slice.entries))
            }
        }
        impl<T: fmt::Debug> fmt::Debug for Slice<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self).finish()
            }
        }
        impl<T, U> PartialEq<Slice<U>> for Slice<T>
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &Slice<U>) -> bool {
                slice_eq(&self.entries, &other.entries, |b1, b2| b1.key == b2.key)
            }
        }
        impl<T, U> PartialEq<[U]> for Slice<T>
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &[U]) -> bool {
                slice_eq(&self.entries, other, |b, o| b.key == *o)
            }
        }
        impl<T, U> PartialEq<Slice<U>> for [T]
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &Slice<U>) -> bool {
                slice_eq(self, &other.entries, |o, b| *o == b.key)
            }
        }
        impl<T, U, const N: usize> PartialEq<[U; N]> for Slice<T>
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &[U; N]) -> bool {
                <Self as PartialEq<[U]>>::eq(self, other)
            }
        }
        impl<T, const N: usize, U> PartialEq<Slice<U>> for [T; N]
        where
            T: PartialEq<U>,
        {
            fn eq(&self, other: &Slice<U>) -> bool {
                <[T] as PartialEq<Slice<U>>>::eq(self, other)
            }
        }
        impl<T: Eq> Eq for Slice<T> {}
        impl<T: PartialOrd> PartialOrd for Slice<T> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.iter().partial_cmp(other)
            }
        }
        impl<T: Ord> Ord for Slice<T> {
            fn cmp(&self, other: &Self) -> Ordering {
                self.iter().cmp(other)
            }
        }
        impl<T: Hash> Hash for Slice<T> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.len().hash(state);
                for value in self {
                    value.hash(state);
                }
            }
        }
        impl<T> Index<usize> for Slice<T> {
            type Output = T;
            fn index(&self, index: usize) -> &Self::Output {
                &self.entries[index].key
            }
        }
        impl<T, S> Index<ops::Range<usize>> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::Range<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::Range<usize>> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::Range<usize>) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<ops::RangeFrom<usize>> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::RangeFrom<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::RangeFrom<usize>> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::RangeFrom<usize>) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<ops::RangeFull> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::RangeFull) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::RangeFull> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::RangeFull) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<ops::RangeInclusive<usize>> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::RangeInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::RangeInclusive<usize>> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::RangeInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<ops::RangeTo<usize>> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::RangeTo<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::RangeTo<usize>> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::RangeTo<usize>) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<ops::RangeToInclusive<usize>> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: ops::RangeToInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<ops::RangeToInclusive<usize>> for Slice<T> {
            type Output = Self;
            fn index(&self, range: ops::RangeToInclusive<usize>) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
        impl<T, S> Index<(Bound<usize>, Bound<usize>)> for IndexSet<T, S> {
            type Output = Slice<T>;
            fn index(&self, range: (Bound<usize>, Bound<usize>)) -> &Self::Output {
                Slice::from_slice(&self.as_entries()[range])
            }
        }
        impl<T> Index<(Bound<usize>, Bound<usize>)> for Slice<T> {
            type Output = Self;
            fn index(&self, range: (Bound<usize>, Bound<usize>)) -> &Self::Output {
                Slice::from_slice(&self.entries[range])
            }
        }
    }
    pub use self::iter::{
        Difference, Drain, ExtractIf, Intersection, IntoIter, Iter, Splice,
        SymmetricDifference, Union,
    };
    pub use self::mutable::MutableValues;
    pub use self::slice::Slice;
    use crate::TryReserveError;
    use std::hash::RandomState;
    use crate::util::try_simplify_range;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::cmp::Ordering;
    use core::fmt;
    use core::hash::{BuildHasher, Hash};
    use core::ops::{BitAnd, BitOr, BitXor, Index, RangeBounds, Sub};
    use super::{Equivalent, IndexMap};
    type Bucket<T> = super::Bucket<T, ()>;
    /// A hash set where the iteration order of the values is independent of their
    /// hash values.
    ///
    /// The interface is closely compatible with the standard
    /// [`HashSet`][std::collections::HashSet],
    /// but also has additional features.
    ///
    /// # Order
    ///
    /// The values have a consistent order that is determined by the sequence of
    /// insertion and removal calls on the set. The order does not depend on the
    /// values or the hash function at all. Note that insertion order and value
    /// are not affected if a re-insertion is attempted once an element is
    /// already present.
    ///
    /// All iterators traverse the set *in order*.  Set operation iterators like
    /// [`IndexSet::union`] produce a concatenated order, as do their matching "bitwise"
    /// operators.  See their documentation for specifics.
    ///
    /// The insertion order is preserved, with **notable exceptions** like the
    /// [`.remove()`][Self::remove] or [`.swap_remove()`][Self::swap_remove] methods.
    /// Methods such as [`.sort_by()`][Self::sort_by] of
    /// course result in a new order, depending on the sorting order.
    ///
    /// # Indices
    ///
    /// The values are indexed in a compact range without holes in the range
    /// `0..self.len()`. For example, the method `.get_full` looks up the index for
    /// a value, and the method `.get_index` looks up the value by index.
    ///
    /// # Complexity
    ///
    /// Internally, `IndexSet<T, S>` just holds an [`IndexMap<T, (), S>`](IndexMap). Thus the complexity
    /// of the two are the same for most methods.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexSet;
    ///
    /// // Collects which letters appear in a sentence.
    /// let letters: IndexSet<_> = "a short treatise on fungi".chars().collect();
    ///
    /// assert!(letters.contains(&'s'));
    /// assert!(letters.contains(&'t'));
    /// assert!(letters.contains(&'u'));
    /// assert!(!letters.contains(&'y'));
    /// ```
    pub struct IndexSet<T, S = RandomState> {
        pub(crate) map: IndexMap<T, (), S>,
    }
    impl<T, S> Clone for IndexSet<T, S>
    where
        T: Clone,
        S: Clone,
    {
        fn clone(&self) -> Self {
            IndexSet { map: self.map.clone() }
        }
        fn clone_from(&mut self, other: &Self) {
            self.map.clone_from(&other.map);
        }
    }
    impl<T, S> fmt::Debug for IndexSet<T, S>
    where
        T: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }
    impl<T> IndexSet<T> {
        /// Create a new set. (Does not allocate.)
        pub fn new() -> Self {
            IndexSet { map: IndexMap::new() }
        }
        /// Create a new set with capacity for `n` elements.
        /// (Does not allocate if `n` is zero.)
        ///
        /// Computes in **O(n)** time.
        pub fn with_capacity(n: usize) -> Self {
            IndexSet {
                map: IndexMap::with_capacity(n),
            }
        }
    }
    impl<T, S> IndexSet<T, S> {
        /// Create a new set with capacity for `n` elements.
        /// (Does not allocate if `n` is zero.)
        ///
        /// Computes in **O(n)** time.
        pub fn with_capacity_and_hasher(n: usize, hash_builder: S) -> Self {
            IndexSet {
                map: IndexMap::with_capacity_and_hasher(n, hash_builder),
            }
        }
        /// Create a new set with `hash_builder`.
        ///
        /// This function is `const`, so it
        /// can be called in `static` contexts.
        pub const fn with_hasher(hash_builder: S) -> Self {
            IndexSet {
                map: IndexMap::with_hasher(hash_builder),
            }
        }
        #[inline]
        pub(crate) fn into_entries(self) -> Vec<Bucket<T>> {
            self.map.into_entries()
        }
        #[inline]
        pub(crate) fn as_entries(&self) -> &[Bucket<T>] {
            self.map.as_entries()
        }
        pub(crate) fn with_entries<F>(&mut self, f: F)
        where
            F: FnOnce(&mut [Bucket<T>]),
        {
            self.map.with_entries(f);
        }
        /// Return the number of elements the set can hold without reallocating.
        ///
        /// This number is a lower bound; the set might be able to hold more,
        /// but is guaranteed to be able to hold at least this many.
        ///
        /// Computes in **O(1)** time.
        pub fn capacity(&self) -> usize {
            self.map.capacity()
        }
        /// Return a reference to the set's `BuildHasher`.
        pub fn hasher(&self) -> &S {
            self.map.hasher()
        }
        /// Return the number of elements in the set.
        ///
        /// Computes in **O(1)** time.
        pub fn len(&self) -> usize {
            self.map.len()
        }
        /// Returns true if the set contains no elements.
        ///
        /// Computes in **O(1)** time.
        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
        /// Return an iterator over the values of the set, in their order
        pub fn iter(&self) -> Iter<'_, T> {
            Iter::new(self.as_entries())
        }
        /// Remove all elements in the set, while preserving its capacity.
        ///
        /// Computes in **O(n)** time.
        pub fn clear(&mut self) {
            self.map.clear();
        }
        /// Shortens the set, keeping the first `len` elements and dropping the rest.
        ///
        /// If `len` is greater than the set's current length, this has no effect.
        pub fn truncate(&mut self, len: usize) {
            self.map.truncate(len);
        }
        /// Clears the `IndexSet` in the given index range, returning those values
        /// as a drain iterator.
        ///
        /// The range may be any type that implements [`RangeBounds<usize>`],
        /// including all of the `std::ops::Range*` types, or even a tuple pair of
        /// `Bound` start and end values. To drain the set entirely, use `RangeFull`
        /// like `set.drain(..)`.
        ///
        /// This shifts down all entries following the drained range to fill the
        /// gap, and keeps the allocated memory for reuse.
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the set.
        #[track_caller]
        pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
        where
            R: RangeBounds<usize>,
        {
            Drain::new(self.map.core.drain(range))
        }
        /// Creates an iterator which uses a closure to determine if a value should be removed,
        /// for all values in the given range.
        ///
        /// If the closure returns true, then the value is removed and yielded.
        /// If the closure returns false, the value will remain in the list and will not be yielded
        /// by the iterator.
        ///
        /// The range may be any type that implements [`RangeBounds<usize>`],
        /// including all of the `std::ops::Range*` types, or even a tuple pair of
        /// `Bound` start and end values. To check the entire set, use `RangeFull`
        /// like `set.extract_if(.., predicate)`.
        ///
        /// If the returned `ExtractIf` is not exhausted, e.g. because it is dropped without iterating
        /// or the iteration short-circuits, then the remaining elements will be retained.
        /// Use [`retain`] with a negated predicate if you do not need the returned iterator.
        ///
        /// [`retain`]: IndexSet::retain
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the set.
        ///
        /// # Examples
        ///
        /// Splitting a set into even and odd values, reusing the original set:
        ///
        /// ```
        /// use indexmap::IndexSet;
        ///
        /// let mut set: IndexSet<i32> = (0..8).collect();
        /// let extracted: IndexSet<i32> = set.extract_if(.., |v| v % 2 == 0).collect();
        ///
        /// let evens = extracted.into_iter().collect::<Vec<_>>();
        /// let odds = set.into_iter().collect::<Vec<_>>();
        ///
        /// assert_eq!(evens, vec![0, 2, 4, 6]);
        /// assert_eq!(odds, vec![1, 3, 5, 7]);
        /// ```
        #[track_caller]
        pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> ExtractIf<'_, T, F>
        where
            F: FnMut(&T) -> bool,
            R: RangeBounds<usize>,
        {
            ExtractIf::new(&mut self.map.core, range, pred)
        }
        /// Splits the collection into two at the given index.
        ///
        /// Returns a newly allocated set containing the elements in the range
        /// `[at, len)`. After the call, the original set will be left containing
        /// the elements `[0, at)` with its previous capacity unchanged.
        ///
        /// ***Panics*** if `at > len`.
        #[track_caller]
        pub fn split_off(&mut self, at: usize) -> Self
        where
            S: Clone,
        {
            Self {
                map: self.map.split_off(at),
            }
        }
        /// Reserve capacity for `additional` more values.
        ///
        /// Computes in **O(n)** time.
        pub fn reserve(&mut self, additional: usize) {
            self.map.reserve(additional);
        }
        /// Reserve capacity for `additional` more values, without over-allocating.
        ///
        /// Unlike `reserve`, this does not deliberately over-allocate the entry capacity to avoid
        /// frequent re-allocations. However, the underlying data structures may still have internal
        /// capacity requirements, and the allocator itself may give more space than requested, so this
        /// cannot be relied upon to be precisely minimal.
        ///
        /// Computes in **O(n)** time.
        pub fn reserve_exact(&mut self, additional: usize) {
            self.map.reserve_exact(additional);
        }
        /// Try to reserve capacity for `additional` more values.
        ///
        /// Computes in **O(n)** time.
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
            self.map.try_reserve(additional)
        }
        /// Try to reserve capacity for `additional` more values, without over-allocating.
        ///
        /// Unlike `try_reserve`, this does not deliberately over-allocate the entry capacity to avoid
        /// frequent re-allocations. However, the underlying data structures may still have internal
        /// capacity requirements, and the allocator itself may give more space than requested, so this
        /// cannot be relied upon to be precisely minimal.
        ///
        /// Computes in **O(n)** time.
        pub fn try_reserve_exact(
            &mut self,
            additional: usize,
        ) -> Result<(), TryReserveError> {
            self.map.try_reserve_exact(additional)
        }
        /// Shrink the capacity of the set as much as possible.
        ///
        /// Computes in **O(n)** time.
        pub fn shrink_to_fit(&mut self) {
            self.map.shrink_to_fit();
        }
        /// Shrink the capacity of the set with a lower limit.
        ///
        /// Computes in **O(n)** time.
        pub fn shrink_to(&mut self, min_capacity: usize) {
            self.map.shrink_to(min_capacity);
        }
    }
    impl<T, S> IndexSet<T, S>
    where
        T: Hash + Eq,
        S: BuildHasher,
    {
        /// Insert the value into the set.
        ///
        /// If an equivalent item already exists in the set, it returns
        /// `false` leaving the original value in the set and without
        /// altering its insertion order. Otherwise, it inserts the new
        /// item and returns `true`.
        ///
        /// Computes in **O(1)** time (amortized average).
        pub fn insert(&mut self, value: T) -> bool {
            self.map.insert(value, ()).is_none()
        }
        /// Insert the value into the set, and get its index.
        ///
        /// If an equivalent item already exists in the set, it returns
        /// the index of the existing item and `false`, leaving the
        /// original value in the set and without altering its insertion
        /// order. Otherwise, it inserts the new item and returns the index
        /// of the inserted item and `true`.
        ///
        /// Computes in **O(1)** time (amortized average).
        pub fn insert_full(&mut self, value: T) -> (usize, bool) {
            let (index, existing) = self.map.insert_full(value, ());
            (index, existing.is_none())
        }
        /// Insert the value into the set at its ordered position among sorted values.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search`][Self::binary_search], and if needed calling
        /// [`insert_before`][Self::insert_before] for a new value.
        ///
        /// If the sorted item is found in the set, it returns the index of that
        /// existing item and `false`, without any change. Otherwise, it inserts the
        /// new item and returns its sorted index and `true`.
        ///
        /// If the existing items are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the value
        /// is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average). Instead of repeating calls to
        /// `insert_sorted`, it may be faster to call batched [`insert`][Self::insert]
        /// or [`extend`][Self::extend] and only call [`sort`][Self::sort] or
        /// [`sort_unstable`][Self::sort_unstable] once.
        pub fn insert_sorted(&mut self, value: T) -> (usize, bool)
        where
            T: Ord,
        {
            let (index, existing) = self.map.insert_sorted(value, ());
            (index, existing.is_none())
        }
        /// Insert the value into the set at its ordered position among values
        /// sorted by `cmp`.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search_by`][Self::binary_search_by], then calling
        /// [`insert_before`][Self::insert_before].
        ///
        /// If the existing items are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the value
        /// is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average).
        pub fn insert_sorted_by<F>(&mut self, value: T, mut cmp: F) -> (usize, bool)
        where
            F: FnMut(&T, &T) -> Ordering,
        {
            let (index, existing) = self
                .map
                .insert_sorted_by(value, (), |a, (), b, ()| cmp(a, b));
            (index, existing.is_none())
        }
        /// Insert the value into the set at its ordered position among values
        /// using a sort-key extraction function.
        ///
        /// This is equivalent to finding the position with
        /// [`binary_search_by_key`][Self::binary_search_by_key] with `sort_key(key)`,
        /// then calling [`insert_before`][Self::insert_before].
        ///
        /// If the existing items are **not** already sorted, then the insertion
        /// index is unspecified (like [`slice::binary_search`]), but the value
        /// is moved to or inserted at that position regardless.
        ///
        /// Computes in **O(n)** time (average).
        pub fn insert_sorted_by_key<B, F>(
            &mut self,
            value: T,
            mut sort_key: F,
        ) -> (usize, bool)
        where
            B: Ord,
            F: FnMut(&T) -> B,
        {
            let (index, existing) = self
                .map
                .insert_sorted_by_key(value, (), |k, _| sort_key(k));
            (index, existing.is_none())
        }
        /// Insert the value into the set before the value at the given index, or at the end.
        ///
        /// If an equivalent item already exists in the set, it returns `false` leaving the
        /// original value in the set, but moved to the new position. The returned index
        /// will either be the given index or one less, depending on how the value moved.
        /// (See [`shift_insert`](Self::shift_insert) for different behavior here.)
        ///
        /// Otherwise, it inserts the new value exactly at the given index and returns `true`.
        ///
        /// ***Panics*** if `index` is out of bounds.
        /// Valid indices are `0..=set.len()` (inclusive).
        ///
        /// Computes in **O(n)** time (average).
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        /// let mut set: IndexSet<char> = ('a'..='z').collect();
        ///
        /// // The new value '*' goes exactly at the given index.
        /// assert_eq!(set.get_index_of(&'*'), None);
        /// assert_eq!(set.insert_before(10, '*'), (10, true));
        /// assert_eq!(set.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the value 'a' up will shift others down, so this moves *before* 10 to index 9.
        /// assert_eq!(set.insert_before(10, 'a'), (9, false));
        /// assert_eq!(set.get_index_of(&'a'), Some(9));
        /// assert_eq!(set.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the value 'z' down will shift others up, so this moves to exactly 10.
        /// assert_eq!(set.insert_before(10, 'z'), (10, false));
        /// assert_eq!(set.get_index_of(&'z'), Some(10));
        /// assert_eq!(set.get_index_of(&'*'), Some(11));
        ///
        /// // Moving or inserting before the endpoint is also valid.
        /// assert_eq!(set.len(), 27);
        /// assert_eq!(set.insert_before(set.len(), '*'), (26, false));
        /// assert_eq!(set.get_index_of(&'*'), Some(26));
        /// assert_eq!(set.insert_before(set.len(), '+'), (27, true));
        /// assert_eq!(set.get_index_of(&'+'), Some(27));
        /// assert_eq!(set.len(), 28);
        /// ```
        #[track_caller]
        pub fn insert_before(&mut self, index: usize, value: T) -> (usize, bool) {
            let (index, existing) = self.map.insert_before(index, value, ());
            (index, existing.is_none())
        }
        /// Insert the value into the set at the given index.
        ///
        /// If an equivalent item already exists in the set, it returns `false` leaving
        /// the original value in the set, but moved to the given index.
        /// Note that existing values **cannot** be moved to `index == set.len()`!
        /// (See [`insert_before`](Self::insert_before) for different behavior here.)
        ///
        /// Otherwise, it inserts the new value at the given index and returns `true`.
        ///
        /// ***Panics*** if `index` is out of bounds.
        /// Valid indices are `0..set.len()` (exclusive) when moving an existing value, or
        /// `0..=set.len()` (inclusive) when inserting a new value.
        ///
        /// Computes in **O(n)** time (average).
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        /// let mut set: IndexSet<char> = ('a'..='z').collect();
        ///
        /// // The new value '*' goes exactly at the given index.
        /// assert_eq!(set.get_index_of(&'*'), None);
        /// assert_eq!(set.shift_insert(10, '*'), true);
        /// assert_eq!(set.get_index_of(&'*'), Some(10));
        ///
        /// // Moving the value 'a' up to 10 will shift others down, including the '*' that was at 10.
        /// assert_eq!(set.shift_insert(10, 'a'), false);
        /// assert_eq!(set.get_index_of(&'a'), Some(10));
        /// assert_eq!(set.get_index_of(&'*'), Some(9));
        ///
        /// // Moving the value 'z' down to 9 will shift others up, including the '*' that was at 9.
        /// assert_eq!(set.shift_insert(9, 'z'), false);
        /// assert_eq!(set.get_index_of(&'z'), Some(9));
        /// assert_eq!(set.get_index_of(&'*'), Some(10));
        ///
        /// // Existing values can move to len-1 at most, but new values can insert at the endpoint.
        /// assert_eq!(set.len(), 27);
        /// assert_eq!(set.shift_insert(set.len() - 1, '*'), false);
        /// assert_eq!(set.get_index_of(&'*'), Some(26));
        /// assert_eq!(set.shift_insert(set.len(), '+'), true);
        /// assert_eq!(set.get_index_of(&'+'), Some(27));
        /// assert_eq!(set.len(), 28);
        /// ```
        ///
        /// ```should_panic
        /// use indexmap::IndexSet;
        /// let mut set: IndexSet<char> = ('a'..='z').collect();
        ///
        /// // This is an invalid index for moving an existing value!
        /// set.shift_insert(set.len(), 'a');
        /// ```
        #[track_caller]
        pub fn shift_insert(&mut self, index: usize, value: T) -> bool {
            self.map.shift_insert(index, value, ()).is_none()
        }
        /// Adds a value to the set, replacing the existing value, if any, that is
        /// equal to the given one, without altering its insertion order. Returns
        /// the replaced value.
        ///
        /// Computes in **O(1)** time (average).
        pub fn replace(&mut self, value: T) -> Option<T> {
            self.replace_full(value).1
        }
        /// Adds a value to the set, replacing the existing value, if any, that is
        /// equal to the given one, without altering its insertion order. Returns
        /// the index of the item and its replaced value.
        ///
        /// Computes in **O(1)** time (average).
        pub fn replace_full(&mut self, value: T) -> (usize, Option<T>) {
            let hash = self.map.hash(&value);
            match self.map.core.replace_full(hash, value, ()) {
                (i, Some((replaced, ()))) => (i, Some(replaced)),
                (i, None) => (i, None),
            }
        }
        /// Replaces the value at the given index. The new value does not need to be
        /// equivalent to the one it is replacing, but it must be unique to the rest
        /// of the set.
        ///
        /// Returns `Ok(old_value)` if successful, or `Err((other_index, value))` if
        /// an equivalent value already exists at a different index. The set will be
        /// unchanged in the error case.
        ///
        /// ***Panics*** if `index` is out of bounds.
        ///
        /// Computes in **O(1)** time (average).
        #[track_caller]
        pub fn replace_index(
            &mut self,
            index: usize,
            value: T,
        ) -> Result<T, (usize, T)> {
            self.map.replace_index(index, value)
        }
        /// Return an iterator over the values that are in `self` but not `other`.
        ///
        /// Values are produced in the same order that they appear in `self`.
        pub fn difference<'a, S2>(
            &'a self,
            other: &'a IndexSet<T, S2>,
        ) -> Difference<'a, T, S2>
        where
            S2: BuildHasher,
        {
            Difference::new(self, other)
        }
        /// Return an iterator over the values that are in `self` or `other`,
        /// but not in both.
        ///
        /// Values from `self` are produced in their original order, followed by
        /// values from `other` in their original order.
        pub fn symmetric_difference<'a, S2>(
            &'a self,
            other: &'a IndexSet<T, S2>,
        ) -> SymmetricDifference<'a, T, S, S2>
        where
            S2: BuildHasher,
        {
            SymmetricDifference::new(self, other)
        }
        /// Return an iterator over the values that are in both `self` and `other`.
        ///
        /// Values are produced in the same order that they appear in `self`.
        pub fn intersection<'a, S2>(
            &'a self,
            other: &'a IndexSet<T, S2>,
        ) -> Intersection<'a, T, S2>
        where
            S2: BuildHasher,
        {
            Intersection::new(self, other)
        }
        /// Return an iterator over all values that are in `self` or `other`.
        ///
        /// Values from `self` are produced in their original order, followed by
        /// values that are unique to `other` in their original order.
        pub fn union<'a, S2>(&'a self, other: &'a IndexSet<T, S2>) -> Union<'a, T, S>
        where
            S2: BuildHasher,
        {
            Union::new(self, other)
        }
        /// Creates a splicing iterator that replaces the specified range in the set
        /// with the given `replace_with` iterator and yields the removed items.
        /// `replace_with` does not need to be the same length as `range`.
        ///
        /// The `range` is removed even if the iterator is not consumed until the
        /// end. It is unspecified how many elements are removed from the set if the
        /// `Splice` value is leaked.
        ///
        /// The input iterator `replace_with` is only consumed when the `Splice`
        /// value is dropped. If a value from the iterator matches an existing entry
        /// in the set (outside of `range`), then the original will be unchanged.
        /// Otherwise, the new value will be inserted in the replaced `range`.
        ///
        /// ***Panics*** if the starting point is greater than the end point or if
        /// the end point is greater than the length of the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        ///
        /// let mut set = IndexSet::from([0, 1, 2, 3, 4]);
        /// let new = [5, 4, 3, 2, 1];
        /// let removed: Vec<_> = set.splice(2..4, new).collect();
        ///
        /// // 1 and 4 kept their positions, while 5, 3, and 2 were newly inserted.
        /// assert!(set.into_iter().eq([0, 1, 5, 3, 2, 4]));
        /// assert_eq!(removed, &[2, 3]);
        /// ```
        #[track_caller]
        pub fn splice<R, I>(
            &mut self,
            range: R,
            replace_with: I,
        ) -> Splice<'_, I::IntoIter, T, S>
        where
            R: RangeBounds<usize>,
            I: IntoIterator<Item = T>,
        {
            Splice::new(self, range, replace_with.into_iter())
        }
        /// Moves all values from `other` into `self`, leaving `other` empty.
        ///
        /// This is equivalent to calling [`insert`][Self::insert] for each value
        /// from `other` in order, which means that values that already exist
        /// in `self` are unchanged in their current position.
        ///
        /// See also [`union`][Self::union] to iterate the combined values by
        /// reference, without modifying `self` or `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        ///
        /// let mut a = IndexSet::from([3, 2, 1]);
        /// let mut b = IndexSet::from([3, 4, 5]);
        /// let old_capacity = b.capacity();
        ///
        /// a.append(&mut b);
        ///
        /// assert_eq!(a.len(), 5);
        /// assert_eq!(b.len(), 0);
        /// assert_eq!(b.capacity(), old_capacity);
        ///
        /// assert!(a.iter().eq(&[3, 2, 1, 4, 5]));
        /// ```
        pub fn append<S2>(&mut self, other: &mut IndexSet<T, S2>) {
            self.map.append(&mut other.map);
        }
    }
    impl<T, S> IndexSet<T, S>
    where
        S: BuildHasher,
    {
        /// Return `true` if an equivalent to `value` exists in the set.
        ///
        /// Computes in **O(1)** time (average).
        pub fn contains<Q>(&self, value: &Q) -> bool
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.contains_key(value)
        }
        /// Return a reference to the value stored in the set, if it is present,
        /// else `None`.
        ///
        /// Computes in **O(1)** time (average).
        pub fn get<Q>(&self, value: &Q) -> Option<&T>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.get_key_value(value).map(|(x, &())| x)
        }
        /// Return item index and value
        pub fn get_full<Q>(&self, value: &Q) -> Option<(usize, &T)>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.get_full(value).map(|(i, x, &())| (i, x))
        }
        /// Return item index, if it exists in the set
        ///
        /// Computes in **O(1)** time (average).
        pub fn get_index_of<Q>(&self, value: &Q) -> Option<usize>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.get_index_of(value)
        }
        /// Remove the value from the set, and return `true` if it was present.
        ///
        /// **NOTE:** This is equivalent to [`.swap_remove(value)`][Self::swap_remove], replacing this
        /// value's position with the last element, and it is deprecated in favor of calling that
        /// explicitly. If you need to preserve the relative order of the values in the set, use
        /// [`.shift_remove(value)`][Self::shift_remove] instead.
        #[deprecated(
            note = "`remove` disrupts the set order -- \
        use `swap_remove` or `shift_remove` for explicit behavior."
        )]
        pub fn remove<Q>(&mut self, value: &Q) -> bool
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.swap_remove(value)
        }
        /// Remove the value from the set, and return `true` if it was present.
        ///
        /// Like [`Vec::swap_remove`], the value is removed by swapping it with the
        /// last element of the set and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `false` if `value` was not in the set.
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove<Q>(&mut self, value: &Q) -> bool
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.swap_remove(value).is_some()
        }
        /// Remove the value from the set, and return `true` if it was present.
        ///
        /// Like [`Vec::remove`], the value is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `false` if `value` was not in the set.
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove<Q>(&mut self, value: &Q) -> bool
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.shift_remove(value).is_some()
        }
        /// Removes and returns the value in the set, if any, that is equal to the
        /// given one.
        ///
        /// **NOTE:** This is equivalent to [`.swap_take(value)`][Self::swap_take], replacing this
        /// value's position with the last element, and it is deprecated in favor of calling that
        /// explicitly. If you need to preserve the relative order of the values in the set, use
        /// [`.shift_take(value)`][Self::shift_take] instead.
        #[deprecated(
            note = "`take` disrupts the set order -- \
        use `swap_take` or `shift_take` for explicit behavior."
        )]
        pub fn take<Q>(&mut self, value: &Q) -> Option<T>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.swap_take(value)
        }
        /// Removes and returns the value in the set, if any, that is equal to the
        /// given one.
        ///
        /// Like [`Vec::swap_remove`], the value is removed by swapping it with the
        /// last element of the set and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `None` if `value` was not in the set.
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_take<Q>(&mut self, value: &Q) -> Option<T>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.swap_remove_entry(value).map(|(x, ())| x)
        }
        /// Removes and returns the value in the set, if any, that is equal to the
        /// given one.
        ///
        /// Like [`Vec::remove`], the value is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `None` if `value` was not in the set.
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_take<Q>(&mut self, value: &Q) -> Option<T>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.shift_remove_entry(value).map(|(x, ())| x)
        }
        /// Remove the value from the set return it and the index it had.
        ///
        /// Like [`Vec::swap_remove`], the value is removed by swapping it with the
        /// last element of the set and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Return `None` if `value` was not in the set.
        pub fn swap_remove_full<Q>(&mut self, value: &Q) -> Option<(usize, T)>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.swap_remove_full(value).map(|(i, x, ())| (i, x))
        }
        /// Remove the value from the set return it and the index it had.
        ///
        /// Like [`Vec::remove`], the value is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Return `None` if `value` was not in the set.
        pub fn shift_remove_full<Q>(&mut self, value: &Q) -> Option<(usize, T)>
        where
            Q: ?Sized + Hash + Equivalent<T>,
        {
            self.map.shift_remove_full(value).map(|(i, x, ())| (i, x))
        }
    }
    impl<T, S> IndexSet<T, S> {
        /// Remove the last value
        ///
        /// This preserves the order of the remaining elements.
        ///
        /// Computes in **O(1)** time (average).
        #[doc(alias = "pop_last")]
        pub fn pop(&mut self) -> Option<T> {
            self.map.pop().map(|(x, ())| x)
        }
        /// Removes and returns the last value from a set if the predicate
        /// returns `true`, or [`None`] if the predicate returns false or the set
        /// is empty (the predicate will not be called in that case).
        ///
        /// This preserves the order of the remaining elements.
        ///
        /// Computes in **O(1)** time (average).
        ///
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        ///
        /// let mut set = IndexSet::from([1, 2, 3, 4]);
        /// let pred = |x: &i32| *x % 2 == 0;
        ///
        /// assert_eq!(set.pop_if(pred), Some(4));
        /// assert_eq!(set.as_slice(), &[1, 2, 3]);
        /// assert_eq!(set.pop_if(pred), None);
        /// ```
        pub fn pop_if(&mut self, predicate: impl FnOnce(&T) -> bool) -> Option<T> {
            let last = self.last()?;
            if predicate(last) { self.pop() } else { None }
        }
        /// Scan through each value in the set and keep those where the
        /// closure `keep` returns `true`.
        ///
        /// The elements are visited in order, and remaining elements keep their
        /// order.
        ///
        /// Computes in **O(n)** time (average).
        pub fn retain<F>(&mut self, mut keep: F)
        where
            F: FnMut(&T) -> bool,
        {
            self.map.retain(move |x, &mut ()| keep(x))
        }
        /// Sort the set's values by their default ordering.
        ///
        /// This is a stable sort -- but equivalent values should not normally coexist in
        /// a set at all, so [`sort_unstable`][Self::sort_unstable] is preferred
        /// because it is generally faster and doesn't allocate auxiliary memory.
        ///
        /// See [`sort_by`](Self::sort_by) for details.
        pub fn sort(&mut self)
        where
            T: Ord,
        {
            self.map.sort_keys()
        }
        /// Sort the set's values in place using the comparison function `cmp`.
        ///
        /// Computes in **O(n log n)** time and **O(n)** space. The sort is stable.
        pub fn sort_by<F>(&mut self, mut cmp: F)
        where
            F: FnMut(&T, &T) -> Ordering,
        {
            self.map.sort_by(move |a, (), b, ()| cmp(a, b));
        }
        /// Sort the values of the set and return a by-value iterator of
        /// the values with the result.
        ///
        /// The sort is stable.
        pub fn sorted_by<F>(self, mut cmp: F) -> IntoIter<T>
        where
            F: FnMut(&T, &T) -> Ordering,
        {
            let mut entries = self.into_entries();
            entries.sort_by(move |a, b| cmp(&a.key, &b.key));
            IntoIter::new(entries)
        }
        /// Sort the set's values in place using a key extraction function.
        ///
        /// Computes in **O(n log n)** time and **O(n)** space. The sort is stable.
        pub fn sort_by_key<K, F>(&mut self, mut sort_key: F)
        where
            K: Ord,
            F: FnMut(&T) -> K,
        {
            self.with_entries(move |entries| {
                entries.sort_by_key(move |a| sort_key(&a.key));
            });
        }
        /// Sort the set's values by their default ordering.
        ///
        /// See [`sort_unstable_by`](Self::sort_unstable_by) for details.
        pub fn sort_unstable(&mut self)
        where
            T: Ord,
        {
            self.map.sort_unstable_keys()
        }
        /// Sort the set's values in place using the comparison function `cmp`.
        ///
        /// Computes in **O(n log n)** time. The sort is unstable.
        pub fn sort_unstable_by<F>(&mut self, mut cmp: F)
        where
            F: FnMut(&T, &T) -> Ordering,
        {
            self.map.sort_unstable_by(move |a, _, b, _| cmp(a, b))
        }
        /// Sort the values of the set and return a by-value iterator of
        /// the values with the result.
        pub fn sorted_unstable_by<F>(self, mut cmp: F) -> IntoIter<T>
        where
            F: FnMut(&T, &T) -> Ordering,
        {
            let mut entries = self.into_entries();
            entries.sort_unstable_by(move |a, b| cmp(&a.key, &b.key));
            IntoIter::new(entries)
        }
        /// Sort the set's values in place using a key extraction function.
        ///
        /// Computes in **O(n log n)** time. The sort is unstable.
        pub fn sort_unstable_by_key<K, F>(&mut self, mut sort_key: F)
        where
            K: Ord,
            F: FnMut(&T) -> K,
        {
            self.with_entries(move |entries| {
                entries.sort_unstable_by_key(move |a| sort_key(&a.key));
            });
        }
        /// Sort the set's values in place using a key extraction function.
        ///
        /// During sorting, the function is called at most once per entry, by using temporary storage
        /// to remember the results of its evaluation. The order of calls to the function is
        /// unspecified and may change between versions of `indexmap` or the standard library.
        ///
        /// Computes in **O(m n + n log n + c)** time () and **O(n)** space, where the function is
        /// **O(m)**, *n* is the length of the map, and *c* the capacity. The sort is stable.
        pub fn sort_by_cached_key<K, F>(&mut self, mut sort_key: F)
        where
            K: Ord,
            F: FnMut(&T) -> K,
        {
            self.with_entries(move |entries| {
                entries.sort_by_cached_key(move |a| sort_key(&a.key));
            });
        }
        /// Search over a sorted set for a value.
        ///
        /// Returns the position where that value is present, or the position where it can be inserted
        /// to maintain the sort. See [`slice::binary_search`] for more details.
        ///
        /// Computes in **O(log(n))** time, which is notably less scalable than looking the value up
        /// using [`get_index_of`][IndexSet::get_index_of], but this can also position missing values.
        pub fn binary_search(&self, x: &T) -> Result<usize, usize>
        where
            T: Ord,
        {
            self.as_slice().binary_search(x)
        }
        /// Search over a sorted set with a comparator function.
        ///
        /// Returns the position where that value is present, or the position where it can be inserted
        /// to maintain the sort. See [`slice::binary_search_by`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[inline]
        pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
        where
            F: FnMut(&'a T) -> Ordering,
        {
            self.as_slice().binary_search_by(f)
        }
        /// Search over a sorted set with an extraction function.
        ///
        /// Returns the position where that value is present, or the position where it can be inserted
        /// to maintain the sort. See [`slice::binary_search_by_key`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[inline]
        pub fn binary_search_by_key<'a, B, F>(
            &'a self,
            b: &B,
            f: F,
        ) -> Result<usize, usize>
        where
            F: FnMut(&'a T) -> B,
            B: Ord,
        {
            self.as_slice().binary_search_by_key(b, f)
        }
        /// Checks if the values of this set are sorted.
        #[inline]
        pub fn is_sorted(&self) -> bool
        where
            T: PartialOrd,
        {
            self.as_slice().is_sorted()
        }
        /// Checks if this set is sorted using the given comparator function.
        #[inline]
        pub fn is_sorted_by<'a, F>(&'a self, cmp: F) -> bool
        where
            F: FnMut(&'a T, &'a T) -> bool,
        {
            self.as_slice().is_sorted_by(cmp)
        }
        /// Checks if this set is sorted using the given sort-key function.
        #[inline]
        pub fn is_sorted_by_key<'a, F, K>(&'a self, sort_key: F) -> bool
        where
            F: FnMut(&'a T) -> K,
            K: PartialOrd,
        {
            self.as_slice().is_sorted_by_key(sort_key)
        }
        /// Returns the index of the partition point of a sorted set according to the given predicate
        /// (the index of the first element of the second partition).
        ///
        /// See [`slice::partition_point`] for more details.
        ///
        /// Computes in **O(log(n))** time.
        #[must_use]
        pub fn partition_point<P>(&self, pred: P) -> usize
        where
            P: FnMut(&T) -> bool,
        {
            self.as_slice().partition_point(pred)
        }
        /// Reverses the order of the set's values in place.
        ///
        /// Computes in **O(n)** time and **O(1)** space.
        pub fn reverse(&mut self) {
            self.map.reverse()
        }
        /// Returns a slice of all the values in the set.
        ///
        /// Computes in **O(1)** time.
        pub fn as_slice(&self) -> &Slice<T> {
            Slice::from_slice(self.as_entries())
        }
        /// Converts into a boxed slice of all the values in the set.
        ///
        /// Note that this will drop the inner hash table and any excess capacity.
        pub fn into_boxed_slice(self) -> Box<Slice<T>> {
            Slice::from_boxed(self.into_entries().into_boxed_slice())
        }
        /// Get a value by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_index(&self, index: usize) -> Option<&T> {
            self.as_entries().get(index).map(Bucket::key_ref)
        }
        /// Returns a slice of values in the given range of indices.
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Computes in **O(1)** time.
        pub fn get_range<R: RangeBounds<usize>>(&self, range: R) -> Option<&Slice<T>> {
            let entries = self.as_entries();
            let range = try_simplify_range(range, entries.len())?;
            entries.get(range).map(Slice::from_slice)
        }
        /// Get the first value
        ///
        /// Computes in **O(1)** time.
        pub fn first(&self) -> Option<&T> {
            self.as_entries().first().map(Bucket::key_ref)
        }
        /// Get the last value
        ///
        /// Computes in **O(1)** time.
        pub fn last(&self) -> Option<&T> {
            self.as_entries().last().map(Bucket::key_ref)
        }
        /// Remove the value by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Like [`Vec::swap_remove`], the value is removed by swapping it with the
        /// last element of the set and popping it off. **This perturbs
        /// the position of what used to be the last element!**
        ///
        /// Computes in **O(1)** time (average).
        pub fn swap_remove_index(&mut self, index: usize) -> Option<T> {
            self.map.swap_remove_index(index).map(|(x, ())| x)
        }
        /// Remove the value by index
        ///
        /// Valid indices are `0 <= index < self.len()`.
        ///
        /// Like [`Vec::remove`], the value is removed by shifting all of the
        /// elements that follow it, preserving their relative order.
        /// **This perturbs the index of all of those elements!**
        ///
        /// Computes in **O(n)** time (average).
        pub fn shift_remove_index(&mut self, index: usize) -> Option<T> {
            self.map.shift_remove_index(index).map(|(x, ())| x)
        }
        /// Moves the position of a value from one index to another
        /// by shifting all other values in-between.
        ///
        /// * If `from < to`, the other values will shift down while the targeted value moves up.
        /// * If `from > to`, the other values will shift up while the targeted value moves down.
        ///
        /// ***Panics*** if `from` or `to` are out of bounds.
        ///
        /// Computes in **O(n)** time (average).
        #[track_caller]
        pub fn move_index(&mut self, from: usize, to: usize) {
            self.map.move_index(from, to)
        }
        /// Swaps the position of two values in the set.
        ///
        /// ***Panics*** if `a` or `b` are out of bounds.
        ///
        /// Computes in **O(1)** time (average).
        #[track_caller]
        pub fn swap_indices(&mut self, a: usize, b: usize) {
            self.map.swap_indices(a, b)
        }
    }
    /// Access [`IndexSet`] values at indexed positions.
    ///
    /// # Examples
    ///
    /// ```
    /// use indexmap::IndexSet;
    ///
    /// let mut set = IndexSet::new();
    /// for word in "Lorem ipsum dolor sit amet".split_whitespace() {
    ///     set.insert(word.to_string());
    /// }
    /// assert_eq!(set[0], "Lorem");
    /// assert_eq!(set[1], "ipsum");
    /// set.reverse();
    /// assert_eq!(set[0], "amet");
    /// assert_eq!(set[1], "sit");
    /// set.sort();
    /// assert_eq!(set[0], "Lorem");
    /// assert_eq!(set[1], "amet");
    /// ```
    ///
    /// ```should_panic
    /// use indexmap::IndexSet;
    ///
    /// let mut set = IndexSet::new();
    /// set.insert("foo");
    /// println!("{:?}", set[10]); // panics!
    /// ```
    impl<T, S> Index<usize> for IndexSet<T, S> {
        type Output = T;
        /// Returns a reference to the value at the supplied `index`.
        ///
        /// ***Panics*** if `index` is out of bounds.
        fn index(&self, index: usize) -> &T {
            if let Some(value) = self.get_index(index) {
                value
            } else {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "index out of bounds: the len is {0} but the index is {1}",
                            self.len(),
                            index,
                        ),
                    );
                };
            }
        }
    }
    impl<T, S> FromIterator<T> for IndexSet<T, S>
    where
        T: Hash + Eq,
        S: BuildHasher + Default,
    {
        fn from_iter<I: IntoIterator<Item = T>>(iterable: I) -> Self {
            let iter = iterable.into_iter().map(|x| (x, ()));
            IndexSet {
                map: IndexMap::from_iter(iter),
            }
        }
    }
    impl<T, const N: usize> From<[T; N]> for IndexSet<T, RandomState>
    where
        T: Eq + Hash,
    {
        /// # Examples
        ///
        /// ```
        /// use indexmap::IndexSet;
        ///
        /// let set1 = IndexSet::from([1, 2, 3, 4]);
        /// let set2: IndexSet<_> = [1, 2, 3, 4].into();
        /// assert_eq!(set1, set2);
        /// ```
        fn from(arr: [T; N]) -> Self {
            Self::from_iter(arr)
        }
    }
    impl<T, S> Extend<T> for IndexSet<T, S>
    where
        T: Hash + Eq,
        S: BuildHasher,
    {
        fn extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) {
            let iter = iterable.into_iter().map(|x| (x, ()));
            self.map.extend(iter);
        }
    }
    impl<'a, T, S> Extend<&'a T> for IndexSet<T, S>
    where
        T: Hash + Eq + Copy + 'a,
        S: BuildHasher,
    {
        fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iterable: I) {
            let iter = iterable.into_iter().copied();
            self.extend(iter);
        }
    }
    impl<T, S> Default for IndexSet<T, S>
    where
        S: Default,
    {
        /// Return an empty [`IndexSet`]
        fn default() -> Self {
            IndexSet {
                map: IndexMap::default(),
            }
        }
    }
    impl<T, S1, S2> PartialEq<IndexSet<T, S2>> for IndexSet<T, S1>
    where
        T: Hash + Eq,
        S1: BuildHasher,
        S2: BuildHasher,
    {
        fn eq(&self, other: &IndexSet<T, S2>) -> bool {
            self.len() == other.len() && self.is_subset(other)
        }
    }
    impl<T, S> Eq for IndexSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {}
    impl<T, S> IndexSet<T, S>
    where
        T: Eq + Hash,
        S: BuildHasher,
    {
        /// Returns `true` if `self` has no elements in common with `other`.
        pub fn is_disjoint<S2>(&self, other: &IndexSet<T, S2>) -> bool
        where
            S2: BuildHasher,
        {
            if self.len() <= other.len() {
                self.iter().all(move |value| !other.contains(value))
            } else {
                other.iter().all(move |value| !self.contains(value))
            }
        }
        /// Returns `true` if all elements of `self` are contained in `other`.
        pub fn is_subset<S2>(&self, other: &IndexSet<T, S2>) -> bool
        where
            S2: BuildHasher,
        {
            self.len() <= other.len()
                && self.iter().all(move |value| other.contains(value))
        }
        /// Returns `true` if all elements of `other` are contained in `self`.
        pub fn is_superset<S2>(&self, other: &IndexSet<T, S2>) -> bool
        where
            S2: BuildHasher,
        {
            other.is_subset(self)
        }
    }
    impl<T, S1, S2> BitAnd<&IndexSet<T, S2>> for &IndexSet<T, S1>
    where
        T: Eq + Hash + Clone,
        S1: BuildHasher + Default,
        S2: BuildHasher,
    {
        type Output = IndexSet<T, S1>;
        /// Returns the set intersection, cloned into a new set.
        ///
        /// Values are collected in the same order that they appear in `self`.
        fn bitand(self, other: &IndexSet<T, S2>) -> Self::Output {
            self.intersection(other).cloned().collect()
        }
    }
    impl<T, S1, S2> BitOr<&IndexSet<T, S2>> for &IndexSet<T, S1>
    where
        T: Eq + Hash + Clone,
        S1: BuildHasher + Default,
        S2: BuildHasher,
    {
        type Output = IndexSet<T, S1>;
        /// Returns the set union, cloned into a new set.
        ///
        /// Values from `self` are collected in their original order, followed by
        /// values that are unique to `other` in their original order.
        fn bitor(self, other: &IndexSet<T, S2>) -> Self::Output {
            self.union(other).cloned().collect()
        }
    }
    impl<T, S1, S2> BitXor<&IndexSet<T, S2>> for &IndexSet<T, S1>
    where
        T: Eq + Hash + Clone,
        S1: BuildHasher + Default,
        S2: BuildHasher,
    {
        type Output = IndexSet<T, S1>;
        /// Returns the set symmetric-difference, cloned into a new set.
        ///
        /// Values from `self` are collected in their original order, followed by
        /// values from `other` in their original order.
        fn bitxor(self, other: &IndexSet<T, S2>) -> Self::Output {
            self.symmetric_difference(other).cloned().collect()
        }
    }
    impl<T, S1, S2> Sub<&IndexSet<T, S2>> for &IndexSet<T, S1>
    where
        T: Eq + Hash + Clone,
        S1: BuildHasher + Default,
        S2: BuildHasher,
    {
        type Output = IndexSet<T, S1>;
        /// Returns the set difference, cloned into a new set.
        ///
        /// Values are collected in the same order that they appear in `self`.
        fn sub(self, other: &IndexSet<T, S2>) -> Self::Output {
            self.difference(other).cloned().collect()
        }
    }
}
pub use crate::map::IndexMap;
pub use crate::set::IndexSet;
pub use equivalent::Equivalent;
/// Hash value newtype. Not larger than usize, since anything larger
/// isn't used for selecting position anyway.
struct HashValue(usize);
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for HashValue {}
#[automatically_derived]
impl ::core::clone::Clone for HashValue {
    #[inline]
    fn clone(&self) -> HashValue {
        let _: ::core::clone::AssertParamIsClone<usize>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for HashValue {}
#[automatically_derived]
impl ::core::fmt::Debug for HashValue {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_tuple_field1_finish(f, "HashValue", &&self.0)
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for HashValue {}
#[automatically_derived]
impl ::core::cmp::PartialEq for HashValue {
    #[inline]
    fn eq(&self, other: &HashValue) -> bool {
        self.0 == other.0
    }
}
impl HashValue {
    #[inline(always)]
    fn get(self) -> u64 {
        self.0 as u64
    }
}
struct Bucket<K, V> {
    hash: HashValue,
    key: K,
    value: V,
}
#[automatically_derived]
impl<K: ::core::marker::Copy, V: ::core::marker::Copy> ::core::marker::Copy
for Bucket<K, V> {}
#[automatically_derived]
impl<K: ::core::fmt::Debug, V: ::core::fmt::Debug> ::core::fmt::Debug for Bucket<K, V> {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "Bucket",
            "hash",
            &self.hash,
            "key",
            &self.key,
            "value",
            &&self.value,
        )
    }
}
impl<K, V> Clone for Bucket<K, V>
where
    K: Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Bucket {
            hash: self.hash,
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
    fn clone_from(&mut self, other: &Self) {
        self.hash = other.hash;
        self.key.clone_from(&other.key);
        self.value.clone_from(&other.value);
    }
}
impl<K, V> Bucket<K, V> {
    fn key_ref(&self) -> &K {
        &self.key
    }
    fn value_ref(&self) -> &V {
        &self.value
    }
    fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }
    fn key(self) -> K {
        self.key
    }
    fn value(self) -> V {
        self.value
    }
    fn key_value(self) -> (K, V) {
        (self.key, self.value)
    }
    fn refs(&self) -> (&K, &V) {
        (&self.key, &self.value)
    }
    fn ref_mut(&mut self) -> (&K, &mut V) {
        (&self.key, &mut self.value)
    }
    fn muts(&mut self) -> (&mut K, &mut V) {
        (&mut self.key, &mut self.value)
    }
}
/// The error type for [`try_reserve`][IndexMap::try_reserve] methods.
pub struct TryReserveError {
    kind: TryReserveErrorKind,
}
#[automatically_derived]
impl ::core::clone::Clone for TryReserveError {
    #[inline]
    fn clone(&self) -> TryReserveError {
        TryReserveError {
            kind: ::core::clone::Clone::clone(&self.kind),
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for TryReserveError {}
#[automatically_derived]
impl ::core::cmp::PartialEq for TryReserveError {
    #[inline]
    fn eq(&self, other: &TryReserveError) -> bool {
        self.kind == other.kind
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for TryReserveError {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<TryReserveErrorKind>;
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for TryReserveError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field1_finish(
            f,
            "TryReserveError",
            "kind",
            &&self.kind,
        )
    }
}
enum TryReserveErrorKind {
    Std(alloc::collections::TryReserveError),
    CapacityOverflow,
    AllocError { layout: alloc::alloc::Layout },
}
#[automatically_derived]
impl ::core::clone::Clone for TryReserveErrorKind {
    #[inline]
    fn clone(&self) -> TryReserveErrorKind {
        match self {
            TryReserveErrorKind::Std(__self_0) => {
                TryReserveErrorKind::Std(::core::clone::Clone::clone(__self_0))
            }
            TryReserveErrorKind::CapacityOverflow => {
                TryReserveErrorKind::CapacityOverflow
            }
            TryReserveErrorKind::AllocError { layout: __self_0 } => {
                TryReserveErrorKind::AllocError {
                    layout: ::core::clone::Clone::clone(__self_0),
                }
            }
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for TryReserveErrorKind {}
#[automatically_derived]
impl ::core::cmp::PartialEq for TryReserveErrorKind {
    #[inline]
    fn eq(&self, other: &TryReserveErrorKind) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (
                    TryReserveErrorKind::Std(__self_0),
                    TryReserveErrorKind::Std(__arg1_0),
                ) => __self_0 == __arg1_0,
                (
                    TryReserveErrorKind::AllocError { layout: __self_0 },
                    TryReserveErrorKind::AllocError { layout: __arg1_0 },
                ) => __self_0 == __arg1_0,
                _ => true,
            }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for TryReserveErrorKind {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<alloc::collections::TryReserveError>;
        let _: ::core::cmp::AssertParamIsEq<alloc::alloc::Layout>;
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for TryReserveErrorKind {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            TryReserveErrorKind::Std(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Std", &__self_0)
            }
            TryReserveErrorKind::CapacityOverflow => {
                ::core::fmt::Formatter::write_str(f, "CapacityOverflow")
            }
            TryReserveErrorKind::AllocError { layout: __self_0 } => {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "AllocError",
                    "layout",
                    &__self_0,
                )
            }
        }
    }
}
impl TryReserveError {
    fn from_alloc(error: alloc::collections::TryReserveError) -> Self {
        Self {
            kind: TryReserveErrorKind::Std(error),
        }
    }
    fn from_hashbrown(error: hashbrown::TryReserveError) -> Self {
        Self {
            kind: match error {
                hashbrown::TryReserveError::CapacityOverflow => {
                    TryReserveErrorKind::CapacityOverflow
                }
                hashbrown::TryReserveError::AllocError { layout } => {
                    TryReserveErrorKind::AllocError {
                        layout,
                    }
                }
            },
        }
    }
}
impl core::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match &self.kind {
            TryReserveErrorKind::Std(e) => return core::fmt::Display::fmt(e, f),
            TryReserveErrorKind::CapacityOverflow => {
                " because the computed capacity exceeded the collection's maximum"
            }
            TryReserveErrorKind::AllocError { .. } => {
                " because the memory allocator returned an error"
            }
        };
        f.write_str("memory allocation failed")?;
        f.write_str(reason)
    }
}
impl core::error::Error for TryReserveError {}
/// The error type returned by [`get_disjoint_indices_mut`][`IndexMap::get_disjoint_indices_mut`].
///
/// It indicates one of two possible errors:
/// - An index is out-of-bounds.
/// - The same index appeared multiple times in the array.
pub enum GetDisjointMutError {
    /// An index provided was out-of-bounds for the slice.
    IndexOutOfBounds,
    /// Two indices provided were overlapping.
    OverlappingIndices,
}
#[automatically_derived]
impl ::core::fmt::Debug for GetDisjointMutError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                GetDisjointMutError::IndexOutOfBounds => "IndexOutOfBounds",
                GetDisjointMutError::OverlappingIndices => "OverlappingIndices",
            },
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for GetDisjointMutError {
    #[inline]
    fn clone(&self) -> GetDisjointMutError {
        match self {
            GetDisjointMutError::IndexOutOfBounds => {
                GetDisjointMutError::IndexOutOfBounds
            }
            GetDisjointMutError::OverlappingIndices => {
                GetDisjointMutError::OverlappingIndices
            }
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for GetDisjointMutError {}
#[automatically_derived]
impl ::core::cmp::PartialEq for GetDisjointMutError {
    #[inline]
    fn eq(&self, other: &GetDisjointMutError) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for GetDisjointMutError {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {}
}
impl core::fmt::Display for GetDisjointMutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let msg = match self {
            GetDisjointMutError::IndexOutOfBounds => "an index is out of bounds",
            GetDisjointMutError::OverlappingIndices => "there were overlapping indices",
        };
        core::fmt::Display::fmt(msg, f)
    }
}
impl core::error::Error for GetDisjointMutError {}
