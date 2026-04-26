#![feature(prelude_import)]
#![warn(missing_docs, clippy::default_numeric_fallback)]
#![crate_name = "itertools"]
#![doc(test(attr(deny(warnings), allow(deprecated, unstable_name_collisions))))]
//! Extra iterator adaptors, functions and macros.
//!
//! To extend [`Iterator`] with methods in this crate, import
//! the [`Itertools`] trait:
//!
//! ```
//! # #[allow(unused_imports)]
//! use itertools::Itertools;
//! ```
//!
//! Now, new methods like [`interleave`](Itertools::interleave)
//! are available on all iterators:
//!
//! ```
//! use itertools::Itertools;
//!
//! let it = (1..3).interleave(vec![-1, -2]);
//! itertools::assert_equal(it, vec![1, -1, 2, -2]);
//! ```
//!
//! Most iterator methods are also provided as functions (with the benefit
//! that they convert parameters using [`IntoIterator`]):
//!
//! ```
//! use itertools::interleave;
//!
//! for elt in interleave(&[1, 2, 3], &[2, 3, 4]) {
//!     /* loop body */
//!     # let _ = elt;
//! }
//! ```
//!
//! ## Crate Features
//!
//! - `use_std`
//!   - Enabled by default.
//!   - Disable to compile itertools using `#![no_std]`. This disables
//!     any item that depend on allocations (see the `use_alloc` feature)
//!     and hash maps (like `unique`, `counts`, `into_grouping_map` and more).
//! - `use_alloc`
//!   - Enabled by default.
//!   - Enables any item that depend on allocations (like `chunk_by`,
//!     `kmerge`, `join` and many more).
//!
//! ## Rust Version
//!
//! This version of itertools requires Rust 1.63.0 or later.
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
extern crate alloc;
use alloc::{collections::VecDeque, string::String, vec::Vec};
pub use either::Either;
use core::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Write;
use std::hash::Hash;
use std::iter::{once, IntoIterator};
type VecDequeIntoIter<T> = alloc::collections::vec_deque::IntoIter<T>;
type VecIntoIter<T> = alloc::vec::IntoIter<T>;
use std::iter::FromIterator;
#[macro_use]
mod impl_macros {
    //!
    //! Implementation's internal macros
}
#[doc(hidden)]
pub use std::iter as __std_iter;
/// The concrete iterator types.
pub mod structs {
    pub use crate::adaptors::MultiProduct;
    pub use crate::adaptors::{
        Batching, Coalesce, Dedup, DedupBy, DedupByWithCount, DedupWithCount,
        FilterMapOk, FilterOk, Interleave, InterleaveShortest, MapInto, MapOk, Positions,
        Product, PutBack, TakeWhileRef, TupleCombinations, Update, WhileSome,
    };
    pub use crate::combinations::{ArrayCombinations, Combinations};
    pub use crate::combinations_with_replacement::CombinationsWithReplacement;
    pub use crate::cons_tuples_impl::ConsTuples;
    pub use crate::duplicates_impl::{Duplicates, DuplicatesBy};
    pub use crate::exactly_one_err::ExactlyOneError;
    pub use crate::flatten_ok::FlattenOk;
    pub use crate::format::{Format, FormatWith};
    #[allow(deprecated)]
    pub use crate::groupbylazy::GroupBy;
    pub use crate::groupbylazy::{Chunk, ChunkBy, Chunks, Group, Groups, IntoChunks};
    pub use crate::grouping_map::{GroupingMap, GroupingMapBy};
    pub use crate::intersperse::{Intersperse, IntersperseWith};
    pub use crate::kmerge_impl::{KMerge, KMergeBy};
    pub use crate::merge_join::{Merge, MergeBy, MergeJoinBy};
    pub use crate::multipeek_impl::MultiPeek;
    pub use crate::pad_tail::PadUsing;
    pub use crate::peek_nth::PeekNth;
    pub use crate::peeking_take_while::PeekingTakeWhile;
    pub use crate::permutations::Permutations;
    pub use crate::powerset::Powerset;
    pub use crate::process_results_impl::ProcessResults;
    pub use crate::put_back_n_impl::PutBackN;
    pub use crate::rciter_impl::RcIter;
    pub use crate::repeatn::RepeatN;
    #[allow(deprecated)]
    pub use crate::sources::{Iterate, Unfold};
    pub use crate::take_while_inclusive::TakeWhileInclusive;
    pub use crate::tee::Tee;
    pub use crate::tuple_impl::{CircularTupleWindows, TupleBuffer, TupleWindows, Tuples};
    pub use crate::unique_impl::{Unique, UniqueBy};
    pub use crate::with_position::WithPosition;
    pub use crate::zip_eq_impl::ZipEq;
    pub use crate::zip_longest::ZipLongest;
    pub use crate::ziptuple::Zip;
}
/// Traits helpful for using certain `Itertools` methods in generic contexts.
pub mod traits {
    pub use crate::iter_index::IteratorIndex;
    pub use crate::tuple_impl::HomogeneousTuple;
}
pub use crate::concat_impl::concat;
pub use crate::cons_tuples_impl::cons_tuples;
pub use crate::diff::diff_with;
pub use crate::diff::Diff;
pub use crate::kmerge_impl::kmerge_by;
pub use crate::minmax::MinMaxResult;
pub use crate::peeking_take_while::PeekingNext;
pub use crate::process_results_impl::process_results;
pub use crate::repeatn::repeat_n;
#[allow(deprecated)]
pub use crate::sources::{iterate, unfold};
#[allow(deprecated)]
pub use crate::structs::*;
pub use crate::unziptuple::{multiunzip, MultiUnzip};
pub use crate::with_position::Position;
pub use crate::ziptuple::multizip;
mod adaptors {
    //! Licensed under the Apache License, Version 2.0
    //! <https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
    //! <https://opensource.org/licenses/MIT>, at your
    //! option. This file may not be copied, modified, or distributed
    //! except according to those terms.
    mod coalesce {
        use std::fmt;
        use std::iter::FusedIterator;
        use crate::size_hint;
        #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
        pub struct CoalesceBy<I, F, C>
        where
            I: Iterator,
            C: CountItem<I::Item>,
        {
            iter: I,
            /// `last` is `None` while no item have been taken out of `iter` (at definition).
            /// Then `last` will be `Some(Some(item))` until `iter` is exhausted,
            /// in which case `last` will be `Some(None)`.
            last: Option<Option<C::CItem>>,
            f: F,
        }
        impl<I, F, C> Clone for CoalesceBy<I, F, C>
        where
            I: Clone + Iterator,
            F: Clone,
            C: CountItem<I::Item>,
            C::CItem: Clone,
        {
            #[inline]
            fn clone(&self) -> Self {
                Self {
                    last: self.last.clone(),
                    iter: self.iter.clone(),
                    f: self.f.clone(),
                }
            }
        }
        impl<I, F, C> fmt::Debug for CoalesceBy<I, F, C>
        where
            I: Iterator + fmt::Debug,
            C: CountItem<I::Item>,
            C::CItem: fmt::Debug,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("CoalesceBy")
                    .field("iter", &self.iter)
                    .field("last", &self.last)
                    .finish()
            }
        }
        pub trait CoalescePredicate<Item, T> {
            fn coalesce_pair(&mut self, t: T, item: Item) -> Result<T, (T, T)>;
        }
        impl<I, F, C> Iterator for CoalesceBy<I, F, C>
        where
            I: Iterator,
            F: CoalescePredicate<I::Item, C::CItem>,
            C: CountItem<I::Item>,
        {
            type Item = C::CItem;
            fn next(&mut self) -> Option<Self::Item> {
                let Self { iter, last, f } = self;
                let init = match last {
                    Some(elt) => elt.take(),
                    None => {
                        *last = Some(None);
                        iter.next().map(C::new)
                    }
                }?;
                Some(
                    iter
                        .try_fold(
                            init,
                            |accum, next| match f.coalesce_pair(accum, next) {
                                Ok(joined) => Ok(joined),
                                Err((last_, next_)) => {
                                    *last = Some(Some(next_));
                                    Err(last_)
                                }
                            },
                        )
                        .unwrap_or_else(|x| x),
                )
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                let (low, hi) = size_hint::add_scalar(
                    self.iter.size_hint(),
                    #[allow(non_exhaustive_omitted_patterns)]
                    match self.last {
                        Some(Some(_)) => true,
                        _ => false,
                    } as usize,
                );
                ((low > 0) as usize, hi)
            }
            fn fold<Acc, FnAcc>(self, acc: Acc, mut fn_acc: FnAcc) -> Acc
            where
                FnAcc: FnMut(Acc, Self::Item) -> Acc,
            {
                let Self { mut iter, last, mut f } = self;
                if let Some(last) = last.unwrap_or_else(|| iter.next().map(C::new)) {
                    let (last, acc) = iter
                        .fold(
                            (last, acc),
                            |(last, acc), elt| {
                                match f.coalesce_pair(last, elt) {
                                    Ok(joined) => (joined, acc),
                                    Err((last_, next_)) => (next_, fn_acc(acc, last_)),
                                }
                            },
                        );
                    fn_acc(acc, last)
                } else {
                    acc
                }
            }
        }
        impl<I, F, C> FusedIterator for CoalesceBy<I, F, C>
        where
            I: Iterator,
            F: CoalescePredicate<I::Item, C::CItem>,
            C: CountItem<I::Item>,
        {}
        pub struct NoCount;
        pub struct WithCount;
        pub trait CountItem<T> {
            type CItem;
            fn new(t: T) -> Self::CItem;
        }
        impl<T> CountItem<T> for NoCount {
            type CItem = T;
            #[inline(always)]
            fn new(t: T) -> T {
                t
            }
        }
        impl<T> CountItem<T> for WithCount {
            type CItem = (usize, T);
            #[inline(always)]
            fn new(t: T) -> (usize, T) {
                (1, t)
            }
        }
        /// An iterator adaptor that may join together adjacent elements.
        ///
        /// See [`.coalesce()`](crate::Itertools::coalesce) for more information.
        pub type Coalesce<I, F> = CoalesceBy<I, F, NoCount>;
        impl<F, Item, T> CoalescePredicate<Item, T> for F
        where
            F: FnMut(T, Item) -> Result<T, (T, T)>,
        {
            fn coalesce_pair(&mut self, t: T, item: Item) -> Result<T, (T, T)> {
                self(t, item)
            }
        }
        /// Create a new `Coalesce`.
        pub fn coalesce<I, F>(iter: I, f: F) -> Coalesce<I, F>
        where
            I: Iterator,
        {
            Coalesce { last: None, iter, f }
        }
        /// An iterator adaptor that removes repeated duplicates, determining equality using a comparison function.
        ///
        /// See [`.dedup_by()`](crate::Itertools::dedup_by) or [`.dedup()`](crate::Itertools::dedup) for more information.
        pub type DedupBy<I, Pred> = CoalesceBy<I, DedupPred2CoalescePred<Pred>, NoCount>;
        pub struct DedupPred2CoalescePred<DP>(DP);
        #[automatically_derived]
        impl<DP: ::core::clone::Clone> ::core::clone::Clone
        for DedupPred2CoalescePred<DP> {
            #[inline]
            fn clone(&self) -> DedupPred2CoalescePred<DP> {
                DedupPred2CoalescePred(::core::clone::Clone::clone(&self.0))
            }
        }
        impl<DP> fmt::Debug for DedupPred2CoalescePred<DP> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("DedupPred2CoalescePred").finish()
            }
        }
        pub trait DedupPredicate<T> {
            fn dedup_pair(&mut self, a: &T, b: &T) -> bool;
        }
        impl<DP, T> CoalescePredicate<T, T> for DedupPred2CoalescePred<DP>
        where
            DP: DedupPredicate<T>,
        {
            fn coalesce_pair(&mut self, t: T, item: T) -> Result<T, (T, T)> {
                if self.0.dedup_pair(&t, &item) { Ok(t) } else { Err((t, item)) }
            }
        }
        pub struct DedupEq;
        #[automatically_derived]
        impl ::core::clone::Clone for DedupEq {
            #[inline]
            fn clone(&self) -> DedupEq {
                DedupEq
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for DedupEq {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "DedupEq")
            }
        }
        impl<T: PartialEq> DedupPredicate<T> for DedupEq {
            fn dedup_pair(&mut self, a: &T, b: &T) -> bool {
                a == b
            }
        }
        impl<T, F: FnMut(&T, &T) -> bool> DedupPredicate<T> for F {
            fn dedup_pair(&mut self, a: &T, b: &T) -> bool {
                self(a, b)
            }
        }
        /// Create a new `DedupBy`.
        pub fn dedup_by<I, Pred>(iter: I, dedup_pred: Pred) -> DedupBy<I, Pred>
        where
            I: Iterator,
        {
            DedupBy {
                last: None,
                iter,
                f: DedupPred2CoalescePred(dedup_pred),
            }
        }
        /// An iterator adaptor that removes repeated duplicates.
        ///
        /// See [`.dedup()`](crate::Itertools::dedup) for more information.
        pub type Dedup<I> = DedupBy<I, DedupEq>;
        /// Create a new `Dedup`.
        pub fn dedup<I>(iter: I) -> Dedup<I>
        where
            I: Iterator,
        {
            dedup_by(iter, DedupEq)
        }
        /// An iterator adaptor that removes repeated duplicates, while keeping a count of how many
        /// repeated elements were present. This will determine equality using a comparison function.
        ///
        /// See [`.dedup_by_with_count()`](crate::Itertools::dedup_by_with_count) or
        /// [`.dedup_with_count()`](crate::Itertools::dedup_with_count) for more information.
        pub type DedupByWithCount<I, Pred> = CoalesceBy<
            I,
            DedupPredWithCount2CoalescePred<Pred>,
            WithCount,
        >;
        pub struct DedupPredWithCount2CoalescePred<DP>(DP);
        #[automatically_derived]
        impl<DP: ::core::clone::Clone> ::core::clone::Clone
        for DedupPredWithCount2CoalescePred<DP> {
            #[inline]
            fn clone(&self) -> DedupPredWithCount2CoalescePred<DP> {
                DedupPredWithCount2CoalescePred(::core::clone::Clone::clone(&self.0))
            }
        }
        #[automatically_derived]
        impl<DP: ::core::fmt::Debug> ::core::fmt::Debug
        for DedupPredWithCount2CoalescePred<DP> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "DedupPredWithCount2CoalescePred",
                    &&self.0,
                )
            }
        }
        impl<DP, T> CoalescePredicate<T, (usize, T)>
        for DedupPredWithCount2CoalescePred<DP>
        where
            DP: DedupPredicate<T>,
        {
            fn coalesce_pair(
                &mut self,
                (c, t): (usize, T),
                item: T,
            ) -> Result<(usize, T), ((usize, T), (usize, T))> {
                if self.0.dedup_pair(&t, &item) {
                    Ok((c + 1, t))
                } else {
                    Err(((c, t), (1, item)))
                }
            }
        }
        /// An iterator adaptor that removes repeated duplicates, while keeping a count of how many
        /// repeated elements were present.
        ///
        /// See [`.dedup_with_count()`](crate::Itertools::dedup_with_count) for more information.
        pub type DedupWithCount<I> = DedupByWithCount<I, DedupEq>;
        /// Create a new `DedupByWithCount`.
        pub fn dedup_by_with_count<I, Pred>(
            iter: I,
            dedup_pred: Pred,
        ) -> DedupByWithCount<I, Pred>
        where
            I: Iterator,
        {
            DedupByWithCount {
                last: None,
                iter,
                f: DedupPredWithCount2CoalescePred(dedup_pred),
            }
        }
        /// Create a new `DedupWithCount`.
        pub fn dedup_with_count<I>(iter: I) -> DedupWithCount<I>
        where
            I: Iterator,
        {
            dedup_by_with_count(iter, DedupEq)
        }
    }
    pub(crate) mod map {
        use std::iter::FromIterator;
        use std::marker::PhantomData;
        #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
        pub struct MapSpecialCase<I, F> {
            pub(crate) iter: I,
            pub(crate) f: F,
        }
        #[automatically_derived]
        impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
        for MapSpecialCase<I, F> {
            #[inline]
            fn clone(&self) -> MapSpecialCase<I, F> {
                MapSpecialCase {
                    iter: ::core::clone::Clone::clone(&self.iter),
                    f: ::core::clone::Clone::clone(&self.f),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::fmt::Debug, F: ::core::fmt::Debug> ::core::fmt::Debug
        for MapSpecialCase<I, F> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "MapSpecialCase",
                    "iter",
                    &self.iter,
                    "f",
                    &&self.f,
                )
            }
        }
        pub trait MapSpecialCaseFn<T> {
            type Out;
            fn call(&mut self, t: T) -> Self::Out;
        }
        impl<I, R> Iterator for MapSpecialCase<I, R>
        where
            I: Iterator,
            R: MapSpecialCaseFn<I::Item>,
        {
            type Item = R::Out;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(|i| self.f.call(i))
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
            fn fold<Acc, Fold>(self, init: Acc, mut fold_f: Fold) -> Acc
            where
                Fold: FnMut(Acc, Self::Item) -> Acc,
            {
                let mut f = self.f;
                self.iter.fold(init, move |acc, v| fold_f(acc, f.call(v)))
            }
            fn collect<C>(self) -> C
            where
                C: FromIterator<Self::Item>,
            {
                let mut f = self.f;
                self.iter.map(move |v| f.call(v)).collect()
            }
        }
        impl<I, R> DoubleEndedIterator for MapSpecialCase<I, R>
        where
            I: DoubleEndedIterator,
            R: MapSpecialCaseFn<I::Item>,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back().map(|i| self.f.call(i))
            }
        }
        impl<I, R> ExactSizeIterator for MapSpecialCase<I, R>
        where
            I: ExactSizeIterator,
            R: MapSpecialCaseFn<I::Item>,
        {}
        /// An iterator adapter to apply a transformation within a nested `Result::Ok`.
        ///
        /// See [`.map_ok()`](crate::Itertools::map_ok) for more information.
        pub type MapOk<I, F> = MapSpecialCase<I, MapSpecialCaseFnOk<F>>;
        impl<F, T, U, E> MapSpecialCaseFn<Result<T, E>> for MapSpecialCaseFnOk<F>
        where
            F: FnMut(T) -> U,
        {
            type Out = Result<U, E>;
            fn call(&mut self, t: Result<T, E>) -> Self::Out {
                t.map(|v| self.0(v))
            }
        }
        pub struct MapSpecialCaseFnOk<F>(F);
        #[automatically_derived]
        impl<F: ::core::clone::Clone> ::core::clone::Clone for MapSpecialCaseFnOk<F> {
            #[inline]
            fn clone(&self) -> MapSpecialCaseFnOk<F> {
                MapSpecialCaseFnOk(::core::clone::Clone::clone(&self.0))
            }
        }
        impl<F> std::fmt::Debug for MapSpecialCaseFnOk<F> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("MapSpecialCaseFnOk").finish()
            }
        }
        /// Create a new `MapOk` iterator.
        pub fn map_ok<I, F, T, U, E>(iter: I, f: F) -> MapOk<I, F>
        where
            I: Iterator<Item = Result<T, E>>,
            F: FnMut(T) -> U,
        {
            MapSpecialCase {
                iter,
                f: MapSpecialCaseFnOk(f),
            }
        }
        /// An iterator adapter to apply `Into` conversion to each element.
        ///
        /// See [`.map_into()`](crate::Itertools::map_into) for more information.
        pub type MapInto<I, R> = MapSpecialCase<I, MapSpecialCaseFnInto<R>>;
        impl<T: Into<U>, U> MapSpecialCaseFn<T> for MapSpecialCaseFnInto<U> {
            type Out = U;
            fn call(&mut self, t: T) -> Self::Out {
                t.into()
            }
        }
        pub struct MapSpecialCaseFnInto<U>(PhantomData<U>);
        impl<U> std::fmt::Debug for MapSpecialCaseFnInto<U> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("MapSpecialCaseFnInto").field("0", &self.0).finish()
            }
        }
        impl<U> Clone for MapSpecialCaseFnInto<U> {
            #[inline]
            fn clone(&self) -> Self {
                Self(PhantomData)
            }
        }
        /// Create a new [`MapInto`] iterator.
        pub fn map_into<I, R>(iter: I) -> MapInto<I, R> {
            MapSpecialCase {
                iter,
                f: MapSpecialCaseFnInto(PhantomData),
            }
        }
    }
    mod multi_product {
        use Option::{self as State, None as ProductEnded, Some as ProductInProgress};
        use Option::{self as CurrentItems, None as NotYetPopulated, Some as Populated};
        use alloc::vec::Vec;
        use crate::size_hint;
        /// An iterator adaptor that iterates over the cartesian product of
        /// multiple iterators of type `I`.
        ///
        /// An iterator element type is `Vec<I::Item>`.
        ///
        /// See [`.multi_cartesian_product()`](crate::Itertools::multi_cartesian_product)
        /// for more information.
        #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
        pub struct MultiProduct<I>(
            State<MultiProductInner<I>>,
        )
        where
            I: Iterator + Clone,
            I::Item: Clone;
        #[automatically_derived]
        impl<I: ::core::clone::Clone> ::core::clone::Clone for MultiProduct<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            #[inline]
            fn clone(&self) -> MultiProduct<I> {
                MultiProduct(::core::clone::Clone::clone(&self.0))
            }
        }
        /// Internals for `MultiProduct`.
        struct MultiProductInner<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            /// Holds the iterators.
            iters: Vec<MultiProductIter<I>>,
            /// Not populated at the beginning then it holds the current item of each iterator.
            cur: CurrentItems<Vec<I::Item>>,
        }
        #[automatically_derived]
        impl<I: ::core::clone::Clone> ::core::clone::Clone for MultiProductInner<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
            I::Item: ::core::clone::Clone,
        {
            #[inline]
            fn clone(&self) -> MultiProductInner<I> {
                MultiProductInner {
                    iters: ::core::clone::Clone::clone(&self.iters),
                    cur: ::core::clone::Clone::clone(&self.cur),
                }
            }
        }
        impl<I> std::fmt::Debug for MultiProduct<I>
        where
            I: Iterator + Clone + std::fmt::Debug,
            I::Item: Clone + std::fmt::Debug,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("MultiProduct").field("0", &self.0).finish()
            }
        }
        impl<I> std::fmt::Debug for MultiProductInner<I>
        where
            I: Iterator + Clone + std::fmt::Debug,
            I::Item: Clone + std::fmt::Debug,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("MultiProductInner")
                    .field("iters", &self.iters)
                    .field("cur", &self.cur)
                    .finish()
            }
        }
        /// Create a new cartesian product iterator over an arbitrary number
        /// of iterators of the same type.
        ///
        /// Iterator element is of type `Vec<H::Item::Item>`.
        pub fn multi_cartesian_product<H>(
            iters: H,
        ) -> MultiProduct<<H::Item as IntoIterator>::IntoIter>
        where
            H: Iterator,
            H::Item: IntoIterator,
            <H::Item as IntoIterator>::IntoIter: Clone,
            <H::Item as IntoIterator>::Item: Clone,
        {
            let inner = MultiProductInner {
                iters: iters.map(|i| MultiProductIter::new(i.into_iter())).collect(),
                cur: NotYetPopulated,
            };
            MultiProduct(ProductInProgress(inner))
        }
        /// Holds the state of a single iterator within a `MultiProduct`.
        struct MultiProductIter<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            iter: I,
            iter_orig: I,
        }
        #[automatically_derived]
        impl<I: ::core::clone::Clone> ::core::clone::Clone for MultiProductIter<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            #[inline]
            fn clone(&self) -> MultiProductIter<I> {
                MultiProductIter {
                    iter: ::core::clone::Clone::clone(&self.iter),
                    iter_orig: ::core::clone::Clone::clone(&self.iter_orig),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::fmt::Debug> ::core::fmt::Debug for MultiProductIter<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "MultiProductIter",
                    "iter",
                    &self.iter,
                    "iter_orig",
                    &&self.iter_orig,
                )
            }
        }
        impl<I> MultiProductIter<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            fn new(iter: I) -> Self {
                Self {
                    iter: iter.clone(),
                    iter_orig: iter,
                }
            }
        }
        impl<I> Iterator for MultiProduct<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {
            type Item = Vec<I::Item>;
            fn next(&mut self) -> Option<Self::Item> {
                let inner = self.0.as_mut()?;
                match &mut inner.cur {
                    Populated(values) => {
                        if true {
                            if !!inner.iters.is_empty() {
                                ::core::panicking::panic(
                                    "assertion failed: !inner.iters.is_empty()",
                                )
                            }
                        }
                        for (iter, item) in inner
                            .iters
                            .iter_mut()
                            .zip(values.iter_mut())
                            .rev()
                        {
                            if let Some(new) = iter.iter.next() {
                                *item = new;
                                return Some(values.clone());
                            } else {
                                iter.iter = iter.iter_orig.clone();
                                *item = iter.iter.next().unwrap();
                            }
                        }
                        self.0 = ProductEnded;
                        None
                    }
                    NotYetPopulated => {
                        let next: Option<Vec<_>> = inner
                            .iters
                            .iter_mut()
                            .map(|i| i.iter.next())
                            .collect();
                        if next.is_none() || inner.iters.is_empty() {
                            self.0 = ProductEnded;
                        } else {
                            inner.cur.clone_from(&next);
                        }
                        next
                    }
                }
            }
            fn count(self) -> usize {
                match self.0 {
                    ProductEnded => 0,
                    ProductInProgress(
                        MultiProductInner { iters, cur: NotYetPopulated },
                    ) => {
                        iters
                            .into_iter()
                            .map(|iter| iter.iter_orig.count())
                            .try_fold(
                                1,
                                |product, count| {
                                    if count == 0 { None } else { Some(product * count) }
                                },
                            )
                            .unwrap_or_default()
                    }
                    ProductInProgress(MultiProductInner { iters, cur: Populated(_) }) => {
                        iters
                            .into_iter()
                            .fold(
                                0,
                                |mut acc, iter| {
                                    if acc != 0 {
                                        acc *= iter.iter_orig.count();
                                    }
                                    acc + iter.iter.count()
                                },
                            )
                    }
                }
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                match &self.0 {
                    ProductEnded => (0, Some(0)),
                    ProductInProgress(
                        MultiProductInner { iters, cur: NotYetPopulated },
                    ) => {
                        iters
                            .iter()
                            .map(|iter| iter.iter_orig.size_hint())
                            .fold((1, Some(1)), size_hint::mul)
                    }
                    ProductInProgress(MultiProductInner { iters, cur: Populated(_) }) => {
                        if let [first, tail @ ..] = &iters[..] {
                            tail.iter()
                                .fold(
                                    first.iter.size_hint(),
                                    |mut sh, iter| {
                                        sh = size_hint::mul(sh, iter.iter_orig.size_hint());
                                        size_hint::add(sh, iter.iter.size_hint())
                                    },
                                )
                        } else {
                            ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            )
                        }
                    }
                }
            }
            fn last(self) -> Option<Self::Item> {
                let MultiProductInner { iters, cur } = self.0?;
                if let Populated(values) = cur {
                    let mut count = iters.len();
                    let last = iters
                        .into_iter()
                        .zip(values)
                        .map(|(i, value)| {
                            i.iter
                                .last()
                                .unwrap_or_else(|| {
                                    count -= 1;
                                    value
                                })
                        })
                        .collect();
                    if count == 0 { None } else { Some(last) }
                } else {
                    iters.into_iter().map(|i| i.iter.last()).collect()
                }
            }
        }
        impl<I> std::iter::FusedIterator for MultiProduct<I>
        where
            I: Iterator + Clone,
            I::Item: Clone,
        {}
    }
    pub use self::coalesce::*;
    pub use self::map::{map_into, map_ok, MapInto, MapOk};
    pub use self::multi_product::*;
    use crate::size_hint::{self, SizeHint};
    use std::fmt;
    use std::iter::{Enumerate, FromIterator, Fuse, FusedIterator};
    use std::marker::PhantomData;
    /// An iterator adaptor that alternates elements from two iterators until both
    /// run out.
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.interleave()`](crate::Itertools::interleave) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Interleave<I, J> {
        i: Fuse<I>,
        j: Fuse<J>,
        next_coming_from_j: bool,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, J: ::core::clone::Clone> ::core::clone::Clone
    for Interleave<I, J> {
        #[inline]
        fn clone(&self) -> Interleave<I, J> {
            Interleave {
                i: ::core::clone::Clone::clone(&self.i),
                j: ::core::clone::Clone::clone(&self.j),
                next_coming_from_j: ::core::clone::Clone::clone(&self.next_coming_from_j),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, J: ::core::fmt::Debug> ::core::fmt::Debug
    for Interleave<I, J> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Interleave",
                "i",
                &self.i,
                "j",
                &self.j,
                "next_coming_from_j",
                &&self.next_coming_from_j,
            )
        }
    }
    /// Create an iterator that interleaves elements in `i` and `j`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::interleave`](crate::Itertools::interleave).
    pub fn interleave<I, J>(
        i: I,
        j: J,
    ) -> Interleave<<I as IntoIterator>::IntoIter, <J as IntoIterator>::IntoIter>
    where
        I: IntoIterator,
        J: IntoIterator<Item = I::Item>,
    {
        Interleave {
            i: i.into_iter().fuse(),
            j: j.into_iter().fuse(),
            next_coming_from_j: false,
        }
    }
    impl<I, J> Iterator for Interleave<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.next_coming_from_j = !self.next_coming_from_j;
            if self.next_coming_from_j {
                match self.i.next() {
                    None => self.j.next(),
                    r => r,
                }
            } else {
                match self.j.next() {
                    None => self.i.next(),
                    r => r,
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add(self.i.size_hint(), self.j.size_hint())
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            let Self { mut i, mut j, next_coming_from_j } = self;
            if next_coming_from_j {
                match j.next() {
                    Some(y) => init = f(init, y),
                    None => return i.fold(init, f),
                }
            }
            let res = i
                .try_fold(
                    init,
                    |mut acc, x| {
                        acc = f(acc, x);
                        match j.next() {
                            Some(y) => Ok(f(acc, y)),
                            None => Err(acc),
                        }
                    },
                );
            match res {
                Ok(acc) => j.fold(acc, f),
                Err(acc) => i.fold(acc, f),
            }
        }
    }
    impl<I, J> FusedIterator for Interleave<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {}
    /// An iterator adaptor that alternates elements from the two iterators until
    /// one of them runs out.
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.interleave_shortest()`](crate::Itertools::interleave_shortest)
    /// for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct InterleaveShortest<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        i: I,
        j: J,
        next_coming_from_j: bool,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, J: ::core::clone::Clone> ::core::clone::Clone
    for InterleaveShortest<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        #[inline]
        fn clone(&self) -> InterleaveShortest<I, J> {
            InterleaveShortest {
                i: ::core::clone::Clone::clone(&self.i),
                j: ::core::clone::Clone::clone(&self.j),
                next_coming_from_j: ::core::clone::Clone::clone(&self.next_coming_from_j),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, J: ::core::fmt::Debug> ::core::fmt::Debug
    for InterleaveShortest<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "InterleaveShortest",
                "i",
                &self.i,
                "j",
                &self.j,
                "next_coming_from_j",
                &&self.next_coming_from_j,
            )
        }
    }
    /// Create a new `InterleaveShortest` iterator.
    pub fn interleave_shortest<I, J>(i: I, j: J) -> InterleaveShortest<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        InterleaveShortest {
            i,
            j,
            next_coming_from_j: false,
        }
    }
    impl<I, J> Iterator for InterleaveShortest<I, J>
    where
        I: Iterator,
        J: Iterator<Item = I::Item>,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let e = if self.next_coming_from_j { self.j.next() } else { self.i.next() };
            if e.is_some() {
                self.next_coming_from_j = !self.next_coming_from_j;
            }
            e
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (curr_hint, next_hint) = {
                let i_hint = self.i.size_hint();
                let j_hint = self.j.size_hint();
                if self.next_coming_from_j { (j_hint, i_hint) } else { (i_hint, j_hint) }
            };
            let (curr_lower, curr_upper) = curr_hint;
            let (next_lower, next_upper) = next_hint;
            let (combined_lower, combined_upper) = size_hint::mul_scalar(
                size_hint::min(curr_hint, next_hint),
                2,
            );
            let lower = if curr_lower > next_lower {
                combined_lower + 1
            } else {
                combined_lower
            };
            let upper = {
                let extra_elem = match (curr_upper, next_upper) {
                    (_, None) => false,
                    (None, Some(_)) => true,
                    (Some(curr_max), Some(next_max)) => curr_max > next_max,
                };
                if extra_elem {
                    combined_upper.and_then(|x| x.checked_add(1))
                } else {
                    combined_upper
                }
            };
            (lower, upper)
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            let Self { mut i, mut j, next_coming_from_j } = self;
            if next_coming_from_j {
                match j.next() {
                    Some(y) => init = f(init, y),
                    None => return init,
                }
            }
            let res = i
                .try_fold(
                    init,
                    |mut acc, x| {
                        acc = f(acc, x);
                        match j.next() {
                            Some(y) => Ok(f(acc, y)),
                            None => Err(acc),
                        }
                    },
                );
            match res {
                Ok(val) => val,
                Err(val) => val,
            }
        }
    }
    impl<I, J> FusedIterator for InterleaveShortest<I, J>
    where
        I: FusedIterator,
        J: FusedIterator<Item = I::Item>,
    {}
    /// An iterator adaptor that allows putting back a single
    /// item to the front of the iterator.
    ///
    /// Iterator element type is `I::Item`.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct PutBack<I>
    where
        I: Iterator,
    {
        top: Option<I::Item>,
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for PutBack<I>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> PutBack<I> {
            PutBack {
                top: ::core::clone::Clone::clone(&self.top),
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for PutBack<I>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PutBack",
                "top",
                &self.top,
                "iter",
                &&self.iter,
            )
        }
    }
    /// Create an iterator where you can put back a single item
    pub fn put_back<I>(iterable: I) -> PutBack<I::IntoIter>
    where
        I: IntoIterator,
    {
        PutBack {
            top: None,
            iter: iterable.into_iter(),
        }
    }
    impl<I> PutBack<I>
    where
        I: Iterator,
    {
        /// put back value `value` (builder method)
        pub fn with_value(mut self, value: I::Item) -> Self {
            self.put_back(value);
            self
        }
        /// Split the `PutBack` into its parts.
        #[inline]
        pub fn into_parts(self) -> (Option<I::Item>, I) {
            let Self { top, iter } = self;
            (top, iter)
        }
        /// Put back a single value to the front of the iterator.
        ///
        /// If a value is already in the put back slot, it is returned.
        #[inline]
        pub fn put_back(&mut self, x: I::Item) -> Option<I::Item> {
            self.top.replace(x)
        }
    }
    impl<I> Iterator for PutBack<I>
    where
        I: Iterator,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match self.top {
                None => self.iter.next(),
                ref mut some => some.take(),
            }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.iter.size_hint(), self.top.is_some() as usize)
        }
        fn count(self) -> usize {
            self.iter.count() + (self.top.is_some() as usize)
        }
        fn last(self) -> Option<Self::Item> {
            self.iter.last().or(self.top)
        }
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            match self.top {
                None => self.iter.nth(n),
                ref mut some => {
                    if n == 0 {
                        some.take()
                    } else {
                        *some = None;
                        self.iter.nth(n - 1)
                    }
                }
            }
        }
        fn all<G>(&mut self, mut f: G) -> bool
        where
            G: FnMut(Self::Item) -> bool,
        {
            if let Some(elt) = self.top.take() {
                if !f(elt) {
                    return false;
                }
            }
            self.iter.all(f)
        }
        fn fold<Acc, G>(mut self, init: Acc, mut f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut accum = init;
            if let Some(elt) = self.top.take() {
                accum = f(accum, elt);
            }
            self.iter.fold(accum, f)
        }
    }
    /// An iterator adaptor that iterates over the cartesian product of
    /// the element sets of two iterators `I` and `J`.
    ///
    /// Iterator element type is `(I::Item, J::Item)`.
    ///
    /// See [`.cartesian_product()`](crate::Itertools::cartesian_product) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Product<I, J>
    where
        I: Iterator,
    {
        a: I,
        /// `a_cur` is `None` while no item have been taken out of `a` (at definition).
        /// Then `a_cur` will be `Some(Some(item))` until `a` is exhausted,
        /// in which case `a_cur` will be `Some(None)`.
        a_cur: Option<Option<I::Item>>,
        b: J,
        b_orig: J,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, J: ::core::fmt::Debug> ::core::fmt::Debug
    for Product<I, J>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "Product",
                "a",
                &self.a,
                "a_cur",
                &self.a_cur,
                "b",
                &self.b,
                "b_orig",
                &&self.b_orig,
            )
        }
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, J: ::core::clone::Clone> ::core::clone::Clone
    for Product<I, J>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Product<I, J> {
            Product {
                a: ::core::clone::Clone::clone(&self.a),
                a_cur: ::core::clone::Clone::clone(&self.a_cur),
                b: ::core::clone::Clone::clone(&self.b),
                b_orig: ::core::clone::Clone::clone(&self.b_orig),
            }
        }
    }
    /// Create a new cartesian product iterator
    ///
    /// Iterator element type is `(I::Item, J::Item)`.
    pub fn cartesian_product<I, J>(i: I, j: J) -> Product<I, J>
    where
        I: Iterator,
        J: Clone + Iterator,
        I::Item: Clone,
    {
        Product {
            a_cur: None,
            a: i,
            b: j.clone(),
            b_orig: j,
        }
    }
    impl<I, J> Iterator for Product<I, J>
    where
        I: Iterator,
        J: Clone + Iterator,
        I::Item: Clone,
    {
        type Item = (I::Item, J::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let Self { a, a_cur, b, b_orig } = self;
            let elt_b = match b.next() {
                None => {
                    *b = b_orig.clone();
                    match b.next() {
                        None => return None,
                        Some(x) => {
                            *a_cur = Some(a.next());
                            x
                        }
                    }
                }
                Some(x) => x,
            };
            a_cur.get_or_insert_with(|| a.next()).as_ref().map(|a| (a.clone(), elt_b))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let mut sh = size_hint::mul(self.a.size_hint(), self.b_orig.size_hint());
            if #[allow(non_exhaustive_omitted_patterns)]
            match self.a_cur {
                Some(Some(_)) => true,
                _ => false,
            } {
                sh = size_hint::add(sh, self.b.size_hint());
            }
            sh
        }
        fn fold<Acc, G>(self, mut accum: Acc, mut f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            let Self { mut a, a_cur, mut b, b_orig } = self;
            if let Some(mut elt_a) = a_cur.unwrap_or_else(|| a.next()) {
                loop {
                    accum = b.fold(accum, |acc, elt| f(acc, (elt_a.clone(), elt)));
                    if let Some(next_elt_a) = a.next() {
                        b = b_orig.clone();
                        elt_a = next_elt_a;
                    } else {
                        break;
                    }
                }
            }
            accum
        }
    }
    impl<I, J> FusedIterator for Product<I, J>
    where
        I: FusedIterator,
        J: Clone + FusedIterator,
        I::Item: Clone,
    {}
    /// A “meta iterator adaptor”. Its closure receives a reference to the iterator
    /// and may pick off as many elements as it likes, to produce the next iterator element.
    ///
    /// Iterator element type is `X` if the return type of `F` is `Option<X>`.
    ///
    /// See [`.batching()`](crate::Itertools::batching) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Batching<I, F> {
        f: F,
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for Batching<I, F> {
        #[inline]
        fn clone(&self) -> Batching<I, F> {
            Batching {
                f: ::core::clone::Clone::clone(&self.f),
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    impl<I, F> fmt::Debug for Batching<I, F>
    where
        I: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Batching").field("iter", &self.iter).finish()
        }
    }
    /// Create a new Batching iterator.
    pub fn batching<I, F>(iter: I, f: F) -> Batching<I, F> {
        Batching { f, iter }
    }
    impl<B, F, I> Iterator for Batching<I, F>
    where
        I: Iterator,
        F: FnMut(&mut I) -> Option<B>,
    {
        type Item = B;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            (self.f)(&mut self.iter)
        }
    }
    /// An iterator adaptor that borrows from a `Clone`-able iterator
    /// to only pick off elements while the predicate returns `true`.
    ///
    /// See [`.take_while_ref()`](crate::Itertools::take_while_ref) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct TakeWhileRef<'a, I: 'a, F> {
        iter: &'a mut I,
        f: F,
    }
    impl<I, F> fmt::Debug for TakeWhileRef<'_, I, F>
    where
        I: Iterator + fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("TakeWhileRef").field("iter", &self.iter).finish()
        }
    }
    /// Create a new `TakeWhileRef` from a reference to clonable iterator.
    pub fn take_while_ref<I, F>(iter: &mut I, f: F) -> TakeWhileRef<I, F>
    where
        I: Iterator + Clone,
    {
        TakeWhileRef { iter, f }
    }
    impl<I, F> Iterator for TakeWhileRef<'_, I, F>
    where
        I: Iterator + Clone,
        F: FnMut(&I::Item) -> bool,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let old = self.iter.clone();
            match self.iter.next() {
                None => None,
                Some(elt) => {
                    if (self.f)(&elt) {
                        Some(elt)
                    } else {
                        *self.iter = old;
                        None
                    }
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
    }
    /// An iterator adaptor that filters `Option<A>` iterator elements
    /// and produces `A`. Stops on the first `None` encountered.
    ///
    /// See [`.while_some()`](crate::Itertools::while_some) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct WhileSome<I> {
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for WhileSome<I> {
        #[inline]
        fn clone(&self) -> WhileSome<I> {
            WhileSome {
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for WhileSome<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "WhileSome",
                "iter",
                &&self.iter,
            )
        }
    }
    /// Create a new `WhileSome<I>`.
    pub fn while_some<I>(iter: I) -> WhileSome<I> {
        WhileSome { iter }
    }
    impl<I, A> Iterator for WhileSome<I>
    where
        I: Iterator<Item = Option<A>>,
    {
        type Item = A;
        fn next(&mut self) -> Option<Self::Item> {
            match self.iter.next() {
                None | Some(None) => None,
                Some(elt) => elt,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
        fn fold<B, F>(mut self, acc: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let res = self
                .iter
                .try_fold(
                    acc,
                    |acc, item| match item {
                        Some(item) => Ok(f(acc, item)),
                        None => Err(acc),
                    },
                );
            match res {
                Ok(val) => val,
                Err(val) => val,
            }
        }
    }
    /// An iterator to iterate through all combinations in a `Clone`-able iterator that produces tuples
    /// of a specific size.
    ///
    /// See [`.tuple_combinations()`](crate::Itertools::tuple_combinations) for more
    /// information.
    #[must_use = "this iterator adaptor is not lazy but does nearly nothing unless consumed"]
    pub struct TupleCombinations<I, T>
    where
        I: Iterator,
        T: HasCombination<I>,
    {
        iter: T::Combination,
        _mi: PhantomData<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, T: ::core::clone::Clone> ::core::clone::Clone
    for TupleCombinations<I, T>
    where
        I: Iterator,
        T: HasCombination<I>,
        T::Combination: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> TupleCombinations<I, T> {
            TupleCombinations {
                iter: ::core::clone::Clone::clone(&self.iter),
                _mi: ::core::clone::Clone::clone(&self._mi),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, T: ::core::fmt::Debug> ::core::fmt::Debug
    for TupleCombinations<I, T>
    where
        I: Iterator,
        T: HasCombination<I>,
        T::Combination: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TupleCombinations",
                "iter",
                &self.iter,
                "_mi",
                &&self._mi,
            )
        }
    }
    pub trait HasCombination<I>: Sized {
        type Combination: From<I> + Iterator<Item = Self>;
    }
    /// Create a new `TupleCombinations` from a clonable iterator.
    pub fn tuple_combinations<T, I>(iter: I) -> TupleCombinations<I, T>
    where
        I: Iterator + Clone,
        I::Item: Clone,
        T: HasCombination<I>,
    {
        TupleCombinations {
            iter: T::Combination::from(iter),
            _mi: PhantomData,
        }
    }
    impl<I, T> Iterator for TupleCombinations<I, T>
    where
        I: Iterator,
        T: HasCombination<I>,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
        fn size_hint(&self) -> SizeHint {
            self.iter.size_hint()
        }
        fn count(self) -> usize {
            self.iter.count()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.fold(init, f)
        }
    }
    impl<I, T> FusedIterator for TupleCombinations<I, T>
    where
        I: FusedIterator,
        T: HasCombination<I>,
    {}
    pub struct Tuple1Combination<I> {
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for Tuple1Combination<I> {
        #[inline]
        fn clone(&self) -> Tuple1Combination<I> {
            Tuple1Combination {
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for Tuple1Combination<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Tuple1Combination",
                "iter",
                &&self.iter,
            )
        }
    }
    impl<I> From<I> for Tuple1Combination<I> {
        fn from(iter: I) -> Self {
            Self { iter }
        }
    }
    impl<I: Iterator> Iterator for Tuple1Combination<I> {
        type Item = (I::Item,);
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next().map(|x| (x,))
        }
        fn size_hint(&self) -> SizeHint {
            self.iter.size_hint()
        }
        fn count(self) -> usize {
            self.iter.count()
        }
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            self.iter.map(|x| (x,)).fold(init, f)
        }
    }
    impl<I: Iterator> HasCombination<I> for (I::Item,) {
        type Combination = Tuple1Combination<I>;
    }
    pub struct Tuple2Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple1Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple2Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple2Combination<I> {
            Tuple2Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple2Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple2Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple2Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple2Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple2Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a,)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a,)| (z, a))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + 0);
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + 0);
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A);
            type PrevTuple<A> = (A,);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a,)| (z.clone(), a)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple1Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple2Combination<Fuse<I>>;
    }
    pub struct Tuple3Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple2Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple3Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple3Combination<I> {
            Tuple3Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple3Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple3Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple3Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple3Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple3Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a, b)| (z, a, b))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + 0));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + 0));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A);
            type PrevTuple<A> = (A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b)| (z.clone(), a, b)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple2Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple3Combination<Fuse<I>>;
    }
    pub struct Tuple4Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple3Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple4Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple4Combination<I> {
            Tuple4Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple4Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple4Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple4Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple4Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple4Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a, b, c)| (z, a, b, c))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + 0)));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + 0)));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A);
            type PrevTuple<A> = (A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c)| (z.clone(), a, b, c)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple3Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple4Combination<Fuse<I>>;
    }
    pub struct Tuple5Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple4Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple5Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple5Combination<I> {
            Tuple5Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple5Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple5Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple5Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple5Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple5Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a, b, c, d)| (z, a, b, c, d))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + 0))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + 0))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d)| (z.clone(), a, b, c, d)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple4Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple5Combination<Fuse<I>>;
    }
    pub struct Tuple6Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple5Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple6Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple6Combination<I> {
            Tuple6Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple6Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple6Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple6Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple6Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple6Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a, b, c, d, e)| (z, a, b, c, d, e))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + 0)))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + 0)))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e)| (z.clone(), a, b, c, d, e)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple5Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple6Combination<Fuse<I>>;
    }
    pub struct Tuple7Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple6Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple7Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple7Combination<I> {
            Tuple7Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple7Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple7Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple7Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple7Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple7Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c.next().map(|(a, b, c, d, e, f)| (z, a, b, c, d, e, f))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f)| (z.clone(), a, b, c, d, e, f)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple6Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple7Combination<Fuse<I>>;
    }
    pub struct Tuple8Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple7Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple8Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple8Combination<I> {
            Tuple8Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple8Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple8Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple8Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple8Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple8Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f, g)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f, g))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c
                            .next()
                            .map(|(a, b, c, d, e, f, g)| (z, a, b, c, d, e, f, g))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f, g)| (z.clone(), a, b, c, d, e, f, g)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple7Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple8Combination<Fuse<I>>;
    }
    pub struct Tuple9Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple8Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple9Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple9Combination<I> {
            Tuple9Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple9Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple9Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple9Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple9Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple9Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f, g, h)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f, g, h))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c
                            .next()
                            .map(|(a, b, c, d, e, f, g, h)| (z, a, b, c, d, e, f, g, h))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f, g, h)| (z.clone(), a, b, c, d, e, f, g, h)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple8Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple9Combination<Fuse<I>>;
    }
    pub struct Tuple10Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple9Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple10Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple10Combination<I> {
            Tuple10Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple10Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple10Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple10Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple10Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple10Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f, g, h, i)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f, g, h, i))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c
                            .next()
                            .map(|(a, b, c, d, e, f, g, h, i)| (
                                z,
                                a,
                                b,
                                c,
                                d,
                                e,
                                f,
                                g,
                                h,
                                i,
                            ))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f, g, h, i)| (z.clone(), a, b, c, d, e, f, g, h, i)
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple9Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple10Combination<Fuse<I>>;
    }
    pub struct Tuple11Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple10Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple11Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple11Combination<I> {
            Tuple11Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple11Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple11Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple11Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple11Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple11Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f, g, h, i, j)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f, g, h, i, j))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c
                            .next()
                            .map(|(a, b, c, d, e, f, g, h, i, j)| (
                                z,
                                a,
                                b,
                                c,
                                d,
                                e,
                                f,
                                g,
                                h,
                                i,
                                j,
                            ))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1
                + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1
                + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f, g, h, i, j)| (
                    z.clone(),
                    a,
                    b,
                    c,
                    d,
                    e,
                    f,
                    g,
                    h,
                    i,
                    j,
                )
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple10Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple11Combination<Fuse<I>>;
    }
    pub struct Tuple12Combination<I: Iterator> {
        item: Option<I::Item>,
        iter: I,
        c: Tuple11Combination<I>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone
    for Tuple12Combination<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuple12Combination<I> {
            Tuple12Combination {
                item: ::core::clone::Clone::clone(&self.item),
                iter: ::core::clone::Clone::clone(&self.iter),
                c: ::core::clone::Clone::clone(&self.c),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for Tuple12Combination<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Tuple12Combination",
                "item",
                &self.item,
                "iter",
                &self.iter,
                "c",
                &&self.c,
            )
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple12Combination<I> {
        fn from(mut iter: I) -> Self {
            Self {
                item: iter.next(),
                iter: iter.clone(),
                c: iter.into(),
            }
        }
    }
    impl<I: Iterator + Clone> From<I> for Tuple12Combination<Fuse<I>> {
        fn from(iter: I) -> Self {
            Self::from(iter.fuse())
        }
    }
    impl<I, A> Iterator for Tuple12Combination<I>
    where
        I: Iterator<Item = A> + Clone,
        A: Clone,
    {
        type Item = (A, A, A, A, A, A, A, A, A, A, A, A);
        fn next(&mut self) -> Option<Self::Item> {
            if let Some((a, b, c, d, e, f, g, h, i, j, k)) = self.c.next() {
                let z = self.item.clone().unwrap();
                Some((z, a, b, c, d, e, f, g, h, i, j, k))
            } else {
                self.item = self.iter.next();
                self.item
                    .clone()
                    .and_then(|z| {
                        self.c = self.iter.clone().into();
                        self.c
                            .next()
                            .map(|(a, b, c, d, e, f, g, h, i, j, k)| (
                                z,
                                a,
                                b,
                                c,
                                d,
                                e,
                                f,
                                g,
                                h,
                                i,
                                j,
                                k,
                            ))
                    })
            }
        }
        fn size_hint(&self) -> SizeHint {
            const K: usize = 1
                + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))))));
            let (mut n_min, mut n_max) = self.iter.size_hint();
            n_min = checked_binomial(n_min, K).unwrap_or(usize::MAX);
            n_max = n_max.and_then(|n| checked_binomial(n, K));
            size_hint::add(self.c.size_hint(), (n_min, n_max))
        }
        fn count(self) -> usize {
            const K: usize = 1
                + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))))));
            let n = self.iter.count();
            checked_binomial(n, K).unwrap() + self.c.count()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            type CurrTuple<A> = (A, A, A, A, A, A, A, A, A, A, A, A);
            type PrevTuple<A> = (A, A, A, A, A, A, A, A, A, A, A);
            fn map_fn<A: Clone>(z: &A) -> impl FnMut(PrevTuple<A>) -> CurrTuple<A> + '_ {
                move |(a, b, c, d, e, f, g, h, i, j, k)| (
                    z.clone(),
                    a,
                    b,
                    c,
                    d,
                    e,
                    f,
                    g,
                    h,
                    i,
                    j,
                    k,
                )
            }
            let Self { c, item, mut iter } = self;
            if let Some(z) = item.as_ref() {
                init = c.map(map_fn::<A>(z)).fold(init, &mut f);
            }
            while let Some(z) = iter.next() {
                let c: Tuple11Combination<I> = iter.clone().into();
                init = c.map(map_fn::<A>(&z)).fold(init, &mut f);
            }
            init
        }
    }
    impl<I, A> HasCombination<I> for (A, A, A, A, A, A, A, A, A, A, A, A)
    where
        I: Iterator<Item = A> + Clone,
        I::Item: Clone,
    {
        type Combination = Tuple12Combination<Fuse<I>>;
    }
    pub(crate) fn checked_binomial(mut n: usize, mut k: usize) -> Option<usize> {
        if n < k {
            return Some(0);
        }
        k = (n - k).min(k);
        let mut c = 1;
        for i in 1..=k {
            c = (c / i).checked_mul(n)?.checked_add((c % i).checked_mul(n)? / i)?;
            n -= 1;
        }
        Some(c)
    }
    extern crate test;
    #[rustc_test_marker = "adaptors::test_checked_binomial"]
    #[doc(hidden)]
    pub const test_checked_binomial: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("adaptors::test_checked_binomial"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/adaptors/mod.rs",
            start_line: 855usize,
            start_col: 4usize,
            end_line: 855usize,
            end_col: 25usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(test_checked_binomial()),
        ),
    };
    fn test_checked_binomial() {
        const LIMIT: usize = 500;
        let mut row = ::alloc::vec::from_elem(Some(0), LIMIT + 1);
        row[0] = Some(1);
        for n in 0..=LIMIT {
            for k in 0..=LIMIT {
                match (&row[k], &checked_binomial(n, k)) {
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
            row = std::iter::once(Some(1))
                .chain((1..=LIMIT).map(|k| row[k - 1]?.checked_add(row[k]?)))
                .collect();
        }
    }
    /// An iterator adapter to filter values within a nested `Result::Ok`.
    ///
    /// See [`.filter_ok()`](crate::Itertools::filter_ok) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct FilterOk<I, F> {
        iter: I,
        f: F,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for FilterOk<I, F> {
        #[inline]
        fn clone(&self) -> FilterOk<I, F> {
            FilterOk {
                iter: ::core::clone::Clone::clone(&self.iter),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<I, F> fmt::Debug for FilterOk<I, F>
    where
        I: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FilterOk").field("iter", &self.iter).finish()
        }
    }
    /// Create a new `FilterOk` iterator.
    pub fn filter_ok<I, F, T, E>(iter: I, f: F) -> FilterOk<I, F>
    where
        I: Iterator<Item = Result<T, E>>,
        F: FnMut(&T) -> bool,
    {
        FilterOk { iter, f }
    }
    impl<I, F, T, E> Iterator for FilterOk<I, F>
    where
        I: Iterator<Item = Result<T, E>>,
        F: FnMut(&T) -> bool,
    {
        type Item = Result<T, E>;
        fn next(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter
                .find(|res| match res {
                    Ok(t) => f(t),
                    _ => true,
                })
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
        fn fold<Acc, Fold>(self, init: Acc, fold_f: Fold) -> Acc
        where
            Fold: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut f = self.f;
            self.iter
                .filter(|v| v.as_ref().map(&mut f).unwrap_or(true))
                .fold(init, fold_f)
        }
        fn collect<C>(self) -> C
        where
            C: FromIterator<Self::Item>,
        {
            let mut f = self.f;
            self.iter.filter(|v| v.as_ref().map(&mut f).unwrap_or(true)).collect()
        }
    }
    impl<I, F, T, E> DoubleEndedIterator for FilterOk<I, F>
    where
        I: DoubleEndedIterator<Item = Result<T, E>>,
        F: FnMut(&T) -> bool,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter
                .rfind(|res| match res {
                    Ok(t) => f(t),
                    _ => true,
                })
        }
        fn rfold<Acc, Fold>(self, init: Acc, fold_f: Fold) -> Acc
        where
            Fold: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut f = self.f;
            self.iter
                .filter(|v| v.as_ref().map(&mut f).unwrap_or(true))
                .rfold(init, fold_f)
        }
    }
    impl<I, F, T, E> FusedIterator for FilterOk<I, F>
    where
        I: FusedIterator<Item = Result<T, E>>,
        F: FnMut(&T) -> bool,
    {}
    /// An iterator adapter to filter and apply a transformation on values within a nested `Result::Ok`.
    ///
    /// See [`.filter_map_ok()`](crate::Itertools::filter_map_ok) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct FilterMapOk<I, F> {
        iter: I,
        f: F,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for FilterMapOk<I, F> {
        #[inline]
        fn clone(&self) -> FilterMapOk<I, F> {
            FilterMapOk {
                iter: ::core::clone::Clone::clone(&self.iter),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<I, F> fmt::Debug for FilterMapOk<I, F>
    where
        I: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FilterMapOk").field("iter", &self.iter).finish()
        }
    }
    fn transpose_result<T, E>(result: Result<Option<T>, E>) -> Option<Result<T, E>> {
        match result {
            Ok(Some(v)) => Some(Ok(v)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
    /// Create a new `FilterOk` iterator.
    pub fn filter_map_ok<I, F, T, U, E>(iter: I, f: F) -> FilterMapOk<I, F>
    where
        I: Iterator<Item = Result<T, E>>,
        F: FnMut(T) -> Option<U>,
    {
        FilterMapOk { iter, f }
    }
    impl<I, F, T, U, E> Iterator for FilterMapOk<I, F>
    where
        I: Iterator<Item = Result<T, E>>,
        F: FnMut(T) -> Option<U>,
    {
        type Item = Result<U, E>;
        fn next(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter
                .find_map(|res| match res {
                    Ok(t) => f(t).map(Ok),
                    Err(e) => Some(Err(e)),
                })
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
        fn fold<Acc, Fold>(self, init: Acc, fold_f: Fold) -> Acc
        where
            Fold: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut f = self.f;
            self.iter.filter_map(|v| transpose_result(v.map(&mut f))).fold(init, fold_f)
        }
        fn collect<C>(self) -> C
        where
            C: FromIterator<Self::Item>,
        {
            let mut f = self.f;
            self.iter.filter_map(|v| transpose_result(v.map(&mut f))).collect()
        }
    }
    impl<I, F, T, U, E> DoubleEndedIterator for FilterMapOk<I, F>
    where
        I: DoubleEndedIterator<Item = Result<T, E>>,
        F: FnMut(T) -> Option<U>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter
                .by_ref()
                .rev()
                .find_map(|res| match res {
                    Ok(t) => f(t).map(Ok),
                    Err(e) => Some(Err(e)),
                })
        }
        fn rfold<Acc, Fold>(self, init: Acc, fold_f: Fold) -> Acc
        where
            Fold: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut f = self.f;
            self.iter.filter_map(|v| transpose_result(v.map(&mut f))).rfold(init, fold_f)
        }
    }
    impl<I, F, T, U, E> FusedIterator for FilterMapOk<I, F>
    where
        I: FusedIterator<Item = Result<T, E>>,
        F: FnMut(T) -> Option<U>,
    {}
    /// An iterator adapter to get the positions of each element that matches a predicate.
    ///
    /// See [`.positions()`](crate::Itertools::positions) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Positions<I, F> {
        iter: Enumerate<I>,
        f: F,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for Positions<I, F> {
        #[inline]
        fn clone(&self) -> Positions<I, F> {
            Positions {
                iter: ::core::clone::Clone::clone(&self.iter),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<I, F> fmt::Debug for Positions<I, F>
    where
        I: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Positions").field("iter", &self.iter).finish()
        }
    }
    /// Create a new `Positions` iterator.
    pub fn positions<I, F>(iter: I, f: F) -> Positions<I, F>
    where
        I: Iterator,
        F: FnMut(I::Item) -> bool,
    {
        let iter = iter.enumerate();
        Positions { iter, f }
    }
    impl<I, F> Iterator for Positions<I, F>
    where
        I: Iterator,
        F: FnMut(I::Item) -> bool,
    {
        type Item = usize;
        fn next(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter.find_map(|(count, val)| f(val).then_some(count))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
        fn fold<B, G>(self, init: B, mut func: G) -> B
        where
            G: FnMut(B, Self::Item) -> B,
        {
            let mut f = self.f;
            self.iter
                .fold(
                    init,
                    |mut acc, (count, val)| {
                        if f(val) {
                            acc = func(acc, count);
                        }
                        acc
                    },
                )
        }
    }
    impl<I, F> DoubleEndedIterator for Positions<I, F>
    where
        I: DoubleEndedIterator + ExactSizeIterator,
        F: FnMut(I::Item) -> bool,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let f = &mut self.f;
            self.iter.by_ref().rev().find_map(|(count, val)| f(val).then_some(count))
        }
        fn rfold<B, G>(self, init: B, mut func: G) -> B
        where
            G: FnMut(B, Self::Item) -> B,
        {
            let mut f = self.f;
            self.iter
                .rfold(
                    init,
                    |mut acc, (count, val)| {
                        if f(val) {
                            acc = func(acc, count);
                        }
                        acc
                    },
                )
        }
    }
    impl<I, F> FusedIterator for Positions<I, F>
    where
        I: FusedIterator,
        F: FnMut(I::Item) -> bool,
    {}
    /// An iterator adapter to apply a mutating function to each element before yielding it.
    ///
    /// See [`.update()`](crate::Itertools::update) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Update<I, F> {
        iter: I,
        f: F,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for Update<I, F> {
        #[inline]
        fn clone(&self) -> Update<I, F> {
            Update {
                iter: ::core::clone::Clone::clone(&self.iter),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<I, F> fmt::Debug for Update<I, F>
    where
        I: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Update").field("iter", &self.iter).finish()
        }
    }
    /// Create a new `Update` iterator.
    pub fn update<I, F>(iter: I, f: F) -> Update<I, F>
    where
        I: Iterator,
        F: FnMut(&mut I::Item),
    {
        Update { iter, f }
    }
    impl<I, F> Iterator for Update<I, F>
    where
        I: Iterator,
        F: FnMut(&mut I::Item),
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(mut v) = self.iter.next() {
                (self.f)(&mut v);
                Some(v)
            } else {
                None
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }
        fn fold<Acc, G>(self, init: Acc, mut g: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            let mut f = self.f;
            self.iter
                .fold(
                    init,
                    move |acc, mut v| {
                        f(&mut v);
                        g(acc, v)
                    },
                )
        }
        fn collect<C>(self) -> C
        where
            C: FromIterator<Self::Item>,
        {
            let mut f = self.f;
            self.iter
                .map(move |mut v| {
                    f(&mut v);
                    v
                })
                .collect()
        }
    }
    impl<I, F> ExactSizeIterator for Update<I, F>
    where
        I: ExactSizeIterator,
        F: FnMut(&mut I::Item),
    {}
    impl<I, F> DoubleEndedIterator for Update<I, F>
    where
        I: DoubleEndedIterator,
        F: FnMut(&mut I::Item),
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            if let Some(mut v) = self.iter.next_back() {
                (self.f)(&mut v);
                Some(v)
            } else {
                None
            }
        }
    }
    impl<I, F> FusedIterator for Update<I, F>
    where
        I: FusedIterator,
        F: FnMut(&mut I::Item),
    {}
}
mod either_or_both {
    use core::ops::{Deref, DerefMut};
    use crate::EitherOrBoth::*;
    use either::Either;
    /// Value that either holds a single A or B, or both.
    pub enum EitherOrBoth<A, B = A> {
        /// Both values are present.
        Both(A, B),
        /// Only the left value of type `A` is present.
        Left(A),
        /// Only the right value of type `B` is present.
        Right(B),
    }
    #[automatically_derived]
    impl<A: ::core::clone::Clone, B: ::core::clone::Clone> ::core::clone::Clone
    for EitherOrBoth<A, B> {
        #[inline]
        fn clone(&self) -> EitherOrBoth<A, B> {
            match self {
                EitherOrBoth::Both(__self_0, __self_1) => {
                    EitherOrBoth::Both(
                        ::core::clone::Clone::clone(__self_0),
                        ::core::clone::Clone::clone(__self_1),
                    )
                }
                EitherOrBoth::Left(__self_0) => {
                    EitherOrBoth::Left(::core::clone::Clone::clone(__self_0))
                }
                EitherOrBoth::Right(__self_0) => {
                    EitherOrBoth::Right(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[automatically_derived]
    impl<A, B> ::core::marker::StructuralPartialEq for EitherOrBoth<A, B> {}
    #[automatically_derived]
    impl<A: ::core::cmp::PartialEq, B: ::core::cmp::PartialEq> ::core::cmp::PartialEq
    for EitherOrBoth<A, B> {
        #[inline]
        fn eq(&self, other: &EitherOrBoth<A, B>) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        EitherOrBoth::Both(__self_0, __self_1),
                        EitherOrBoth::Both(__arg1_0, __arg1_1),
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    (EitherOrBoth::Left(__self_0), EitherOrBoth::Left(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (EitherOrBoth::Right(__self_0), EitherOrBoth::Right(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl<A: ::core::cmp::Eq, B: ::core::cmp::Eq> ::core::cmp::Eq for EitherOrBoth<A, B> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<A>;
            let _: ::core::cmp::AssertParamIsEq<B>;
        }
    }
    #[automatically_derived]
    impl<A: ::core::hash::Hash, B: ::core::hash::Hash> ::core::hash::Hash
    for EitherOrBoth<A, B> {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            ::core::hash::Hash::hash(&__self_discr, state);
            match self {
                EitherOrBoth::Both(__self_0, __self_1) => {
                    ::core::hash::Hash::hash(__self_0, state);
                    ::core::hash::Hash::hash(__self_1, state)
                }
                EitherOrBoth::Left(__self_0) => ::core::hash::Hash::hash(__self_0, state),
                EitherOrBoth::Right(__self_0) => {
                    ::core::hash::Hash::hash(__self_0, state)
                }
            }
        }
    }
    #[automatically_derived]
    impl<A: ::core::fmt::Debug, B: ::core::fmt::Debug> ::core::fmt::Debug
    for EitherOrBoth<A, B> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                EitherOrBoth::Both(__self_0, __self_1) => {
                    ::core::fmt::Formatter::debug_tuple_field2_finish(
                        f,
                        "Both",
                        __self_0,
                        &__self_1,
                    )
                }
                EitherOrBoth::Left(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Left",
                        &__self_0,
                    )
                }
                EitherOrBoth::Right(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Right",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl<A, B> EitherOrBoth<A, B> {
        /// If `Left`, or `Both`, return true. Otherwise, return false.
        pub fn has_left(&self) -> bool {
            self.as_ref().left().is_some()
        }
        /// If `Right`, or `Both`, return true, otherwise, return false.
        pub fn has_right(&self) -> bool {
            self.as_ref().right().is_some()
        }
        /// If `Left`, return true. Otherwise, return false.
        /// Exclusive version of [`has_left`](EitherOrBoth::has_left).
        pub fn is_left(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                Left(_) => true,
                _ => false,
            }
        }
        /// If `Right`, return true. Otherwise, return false.
        /// Exclusive version of [`has_right`](EitherOrBoth::has_right).
        pub fn is_right(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                Right(_) => true,
                _ => false,
            }
        }
        /// If `Both`, return true. Otherwise, return false.
        pub fn is_both(&self) -> bool {
            self.as_ref().both().is_some()
        }
        /// If `Left`, or `Both`, return `Some` with the left value. Otherwise, return `None`.
        pub fn left(self) -> Option<A> {
            match self {
                Left(left) | Both(left, _) => Some(left),
                _ => None,
            }
        }
        /// If `Right`, or `Both`, return `Some` with the right value. Otherwise, return `None`.
        pub fn right(self) -> Option<B> {
            match self {
                Right(right) | Both(_, right) => Some(right),
                _ => None,
            }
        }
        /// Return tuple of options corresponding to the left and right value respectively
        ///
        /// If `Left` return `(Some(..), None)`, if `Right` return `(None,Some(..))`, else return
        /// `(Some(..),Some(..))`
        pub fn left_and_right(self) -> (Option<A>, Option<B>) {
            self.map_any(Some, Some).or_default()
        }
        /// If `Left`, return `Some` with the left value. If `Right` or `Both`, return `None`.
        ///
        /// # Examples
        ///
        /// ```
        /// // On the `Left` variant.
        /// # use itertools::{EitherOrBoth, EitherOrBoth::{Left, Right, Both}};
        /// let x: EitherOrBoth<_, ()> = Left("bonjour");
        /// assert_eq!(x.just_left(), Some("bonjour"));
        ///
        /// // On the `Right` variant.
        /// let x: EitherOrBoth<(), _> = Right("hola");
        /// assert_eq!(x.just_left(), None);
        ///
        /// // On the `Both` variant.
        /// let x = Both("bonjour", "hola");
        /// assert_eq!(x.just_left(), None);
        /// ```
        pub fn just_left(self) -> Option<A> {
            match self {
                Left(left) => Some(left),
                _ => None,
            }
        }
        /// If `Right`, return `Some` with the right value. If `Left` or `Both`, return `None`.
        ///
        /// # Examples
        ///
        /// ```
        /// // On the `Left` variant.
        /// # use itertools::{EitherOrBoth::{Left, Right, Both}, EitherOrBoth};
        /// let x: EitherOrBoth<_, ()> = Left("auf wiedersehen");
        /// assert_eq!(x.just_left(), Some("auf wiedersehen"));
        ///
        /// // On the `Right` variant.
        /// let x: EitherOrBoth<(), _> = Right("adios");
        /// assert_eq!(x.just_left(), None);
        ///
        /// // On the `Both` variant.
        /// let x = Both("auf wiedersehen", "adios");
        /// assert_eq!(x.just_left(), None);
        /// ```
        pub fn just_right(self) -> Option<B> {
            match self {
                Right(right) => Some(right),
                _ => None,
            }
        }
        /// If `Both`, return `Some` containing the left and right values. Otherwise, return `None`.
        pub fn both(self) -> Option<(A, B)> {
            match self {
                Both(a, b) => Some((a, b)),
                _ => None,
            }
        }
        /// If `Left` or `Both`, return the left value. Otherwise, convert the right value and return it.
        pub fn into_left(self) -> A
        where
            B: Into<A>,
        {
            match self {
                Left(a) | Both(a, _) => a,
                Right(b) => b.into(),
            }
        }
        /// If `Right` or `Both`, return the right value. Otherwise, convert the left value and return it.
        pub fn into_right(self) -> B
        where
            A: Into<B>,
        {
            match self {
                Right(b) | Both(_, b) => b,
                Left(a) => a.into(),
            }
        }
        /// Converts from `&EitherOrBoth<A, B>` to `EitherOrBoth<&A, &B>`.
        pub fn as_ref(&self) -> EitherOrBoth<&A, &B> {
            match *self {
                Left(ref left) => Left(left),
                Right(ref right) => Right(right),
                Both(ref left, ref right) => Both(left, right),
            }
        }
        /// Converts from `&mut EitherOrBoth<A, B>` to `EitherOrBoth<&mut A, &mut B>`.
        pub fn as_mut(&mut self) -> EitherOrBoth<&mut A, &mut B> {
            match *self {
                Left(ref mut left) => Left(left),
                Right(ref mut right) => Right(right),
                Both(ref mut left, ref mut right) => Both(left, right),
            }
        }
        /// Converts from `&EitherOrBoth<A, B>` to `EitherOrBoth<&_, &_>` using the [`Deref`] trait.
        pub fn as_deref(&self) -> EitherOrBoth<&A::Target, &B::Target>
        where
            A: Deref,
            B: Deref,
        {
            match *self {
                Left(ref left) => Left(left),
                Right(ref right) => Right(right),
                Both(ref left, ref right) => Both(left, right),
            }
        }
        /// Converts from `&mut EitherOrBoth<A, B>` to `EitherOrBoth<&mut _, &mut _>` using the [`DerefMut`] trait.
        pub fn as_deref_mut(&mut self) -> EitherOrBoth<&mut A::Target, &mut B::Target>
        where
            A: DerefMut,
            B: DerefMut,
        {
            match *self {
                Left(ref mut left) => Left(left),
                Right(ref mut right) => Right(right),
                Both(ref mut left, ref mut right) => Both(left, right),
            }
        }
        /// Convert `EitherOrBoth<A, B>` to `EitherOrBoth<B, A>`.
        pub fn flip(self) -> EitherOrBoth<B, A> {
            match self {
                Left(a) => Right(a),
                Right(b) => Left(b),
                Both(a, b) => Both(b, a),
            }
        }
        /// Apply the function `f` on the value `a` in `Left(a)` or `Both(a, b)` variants. If it is
        /// present rewrapping the result in `self`'s original variant.
        pub fn map_left<F, M>(self, f: F) -> EitherOrBoth<M, B>
        where
            F: FnOnce(A) -> M,
        {
            match self {
                Both(a, b) => Both(f(a), b),
                Left(a) => Left(f(a)),
                Right(b) => Right(b),
            }
        }
        /// Apply the function `f` on the value `b` in `Right(b)` or `Both(a, b)` variants.
        /// If it is present rewrapping the result in `self`'s original variant.
        pub fn map_right<F, M>(self, f: F) -> EitherOrBoth<A, M>
        where
            F: FnOnce(B) -> M,
        {
            match self {
                Left(a) => Left(a),
                Right(b) => Right(f(b)),
                Both(a, b) => Both(a, f(b)),
            }
        }
        /// Apply the functions `f` and `g` on the value `a` and `b` respectively;
        /// found in `Left(a)`, `Right(b)`, or `Both(a, b)` variants.
        /// The Result is rewrapped `self`'s original variant.
        pub fn map_any<F, L, G, R>(self, f: F, g: G) -> EitherOrBoth<L, R>
        where
            F: FnOnce(A) -> L,
            G: FnOnce(B) -> R,
        {
            match self {
                Left(a) => Left(f(a)),
                Right(b) => Right(g(b)),
                Both(a, b) => Both(f(a), g(b)),
            }
        }
        /// Apply the function `f` on the value `a` in `Left(a)` or `Both(a, _)` variants if it is
        /// present.
        pub fn left_and_then<F, L>(self, f: F) -> EitherOrBoth<L, B>
        where
            F: FnOnce(A) -> EitherOrBoth<L, B>,
        {
            match self {
                Left(a) | Both(a, _) => f(a),
                Right(b) => Right(b),
            }
        }
        /// Apply the function `f` on the value `b`
        /// in `Right(b)` or `Both(_, b)` variants if it is present.
        pub fn right_and_then<F, R>(self, f: F) -> EitherOrBoth<A, R>
        where
            F: FnOnce(B) -> EitherOrBoth<A, R>,
        {
            match self {
                Left(a) => Left(a),
                Right(b) | Both(_, b) => f(b),
            }
        }
        /// Returns a tuple consisting of the `l` and `r` in `Both(l, r)`, if present.
        /// Otherwise, returns the wrapped value for the present element, and the supplied
        /// value for the other. The first (`l`) argument is used for a missing `Left`
        /// value. The second (`r`) argument is used for a missing `Right` value.
        ///
        /// Arguments passed to `or` are eagerly evaluated; if you are passing
        /// the result of a function call, it is recommended to use [`or_else`],
        /// which is lazily evaluated.
        ///
        /// [`or_else`]: EitherOrBoth::or_else
        ///
        /// # Examples
        ///
        /// ```
        /// # use itertools::EitherOrBoth;
        /// assert_eq!(EitherOrBoth::Both("tree", 1).or("stone", 5), ("tree", 1));
        /// assert_eq!(EitherOrBoth::Left("tree").or("stone", 5), ("tree", 5));
        /// assert_eq!(EitherOrBoth::Right(1).or("stone", 5), ("stone", 1));
        /// ```
        pub fn or(self, l: A, r: B) -> (A, B) {
            match self {
                Left(inner_l) => (inner_l, r),
                Right(inner_r) => (l, inner_r),
                Both(inner_l, inner_r) => (inner_l, inner_r),
            }
        }
        /// Returns a tuple consisting of the `l` and `r` in `Both(l, r)`, if present.
        /// Otherwise, returns the wrapped value for the present element, and the [`default`](Default::default)
        /// for the other.
        pub fn or_default(self) -> (A, B)
        where
            A: Default,
            B: Default,
        {
            match self {
                Left(l) => (l, B::default()),
                Right(r) => (A::default(), r),
                Both(l, r) => (l, r),
            }
        }
        /// Returns a tuple consisting of the `l` and `r` in `Both(l, r)`, if present.
        /// Otherwise, returns the wrapped value for the present element, and computes the
        /// missing value with the supplied closure. The first argument (`l`) is used for a
        /// missing `Left` value. The second argument (`r`) is used for a missing `Right` value.
        ///
        /// # Examples
        ///
        /// ```
        /// # use itertools::EitherOrBoth;
        /// let k = 10;
        /// assert_eq!(EitherOrBoth::Both("tree", 1).or_else(|| "stone", || 2 * k), ("tree", 1));
        /// assert_eq!(EitherOrBoth::Left("tree").or_else(|| "stone", || 2 * k), ("tree", 20));
        /// assert_eq!(EitherOrBoth::Right(1).or_else(|| "stone", || 2 * k), ("stone", 1));
        /// ```
        pub fn or_else<L: FnOnce() -> A, R: FnOnce() -> B>(self, l: L, r: R) -> (A, B) {
            match self {
                Left(inner_l) => (inner_l, r()),
                Right(inner_r) => (l(), inner_r),
                Both(inner_l, inner_r) => (inner_l, inner_r),
            }
        }
        /// Returns a mutable reference to the left value. If the left value is not present,
        /// it is replaced with `val`.
        pub fn left_or_insert(&mut self, val: A) -> &mut A {
            self.left_or_insert_with(|| val)
        }
        /// Returns a mutable reference to the right value. If the right value is not present,
        /// it is replaced with `val`.
        pub fn right_or_insert(&mut self, val: B) -> &mut B {
            self.right_or_insert_with(|| val)
        }
        /// If the left value is not present, replace it the value computed by the closure `f`.
        /// Returns a mutable reference to the now-present left value.
        pub fn left_or_insert_with<F>(&mut self, f: F) -> &mut A
        where
            F: FnOnce() -> A,
        {
            match self {
                Left(left) | Both(left, _) => left,
                Right(_) => self.insert_left(f()),
            }
        }
        /// If the right value is not present, replace it the value computed by the closure `f`.
        /// Returns a mutable reference to the now-present right value.
        pub fn right_or_insert_with<F>(&mut self, f: F) -> &mut B
        where
            F: FnOnce() -> B,
        {
            match self {
                Right(right) | Both(_, right) => right,
                Left(_) => self.insert_right(f()),
            }
        }
        /// Sets the `left` value of this instance, and returns a mutable reference to it.
        /// Does not affect the `right` value.
        ///
        /// # Examples
        /// ```
        /// # use itertools::{EitherOrBoth, EitherOrBoth::{Left, Right, Both}};
        ///
        /// // Overwriting a pre-existing value.
        /// let mut either: EitherOrBoth<_, ()> = Left(0_u32);
        /// assert_eq!(*either.insert_left(69), 69);
        ///
        /// // Inserting a second value.
        /// let mut either = Right("no");
        /// assert_eq!(*either.insert_left("yes"), "yes");
        /// assert_eq!(either, Both("yes", "no"));
        /// ```
        pub fn insert_left(&mut self, val: A) -> &mut A {
            match self {
                Left(left) | Both(left, _) => {
                    *left = val;
                    left
                }
                Right(right) => {
                    unsafe {
                        let right = std::ptr::read(right as *mut _);
                        std::ptr::write(self as *mut _, Both(val, right));
                    }
                    if let Both(left, _) = self {
                        left
                    } else {
                        unsafe { std::hint::unreachable_unchecked() }
                    }
                }
            }
        }
        /// Sets the `right` value of this instance, and returns a mutable reference to it.
        /// Does not affect the `left` value.
        ///
        /// # Examples
        /// ```
        /// # use itertools::{EitherOrBoth, EitherOrBoth::{Left, Both}};
        /// // Overwriting a pre-existing value.
        /// let mut either: EitherOrBoth<_, ()> = Left(0_u32);
        /// assert_eq!(*either.insert_left(69), 69);
        ///
        /// // Inserting a second value.
        /// let mut either = Left("what's");
        /// assert_eq!(*either.insert_right(9 + 10), 21 - 2);
        /// assert_eq!(either, Both("what's", 9+10));
        /// ```
        pub fn insert_right(&mut self, val: B) -> &mut B {
            match self {
                Right(right) | Both(_, right) => {
                    *right = val;
                    right
                }
                Left(left) => {
                    unsafe {
                        let left = std::ptr::read(left as *mut _);
                        std::ptr::write(self as *mut _, Both(left, val));
                    }
                    if let Both(_, right) = self {
                        right
                    } else {
                        unsafe { std::hint::unreachable_unchecked() }
                    }
                }
            }
        }
        /// Set `self` to `Both(..)`, containing the specified left and right values,
        /// and returns a mutable reference to those values.
        pub fn insert_both(&mut self, left: A, right: B) -> (&mut A, &mut B) {
            *self = Both(left, right);
            if let Both(left, right) = self {
                (left, right)
            } else {
                unsafe { std::hint::unreachable_unchecked() }
            }
        }
    }
    impl<T> EitherOrBoth<T, T> {
        /// Return either value of left, right, or apply a function `f` to both values if both are present.
        /// The input function has to return the same type as both Right and Left carry.
        ///
        /// This function can be used to preferrably extract the left resp. right value,
        /// but fall back to the other (i.e. right resp. left) if the preferred one is not present.
        ///
        /// # Examples
        /// ```
        /// # use itertools::EitherOrBoth;
        /// assert_eq!(EitherOrBoth::Both(3, 7).reduce(u32::max), 7);
        /// assert_eq!(EitherOrBoth::Left(3).reduce(u32::max), 3);
        /// assert_eq!(EitherOrBoth::Right(7).reduce(u32::max), 7);
        ///
        /// // Extract the left value if present, fall back to the right otherwise.
        /// assert_eq!(EitherOrBoth::Left("left").reduce(|l, _r| l), "left");
        /// assert_eq!(EitherOrBoth::Right("right").reduce(|l, _r| l), "right");
        /// assert_eq!(EitherOrBoth::Both("left", "right").reduce(|l, _r| l), "left");
        /// ```
        pub fn reduce<F>(self, f: F) -> T
        where
            F: FnOnce(T, T) -> T,
        {
            match self {
                Left(a) => a,
                Right(b) => b,
                Both(a, b) => f(a, b),
            }
        }
    }
    impl<A, B> From<EitherOrBoth<A, B>> for Option<Either<A, B>> {
        fn from(value: EitherOrBoth<A, B>) -> Self {
            match value {
                Left(l) => Some(Either::Left(l)),
                Right(r) => Some(Either::Right(r)),
                Both(..) => None,
            }
        }
    }
    impl<A, B> From<Either<A, B>> for EitherOrBoth<A, B> {
        fn from(either: Either<A, B>) -> Self {
            match either {
                Either::Left(l) => Left(l),
                Either::Right(l) => Right(l),
            }
        }
    }
}
pub use crate::either_or_both::EitherOrBoth;
#[doc(hidden)]
pub mod free {
    //! Free functions that create iterator adaptors or call iterator methods.
    //!
    //! The benefit of free functions is that they accept any [`IntoIterator`] as
    //! argument, so the resulting code may be easier to read.
    use std::fmt::Display;
    use std::iter::{self, Zip};
    type VecIntoIter<T> = alloc::vec::IntoIter<T>;
    use alloc::string::String;
    use crate::intersperse::{Intersperse, IntersperseWith};
    use crate::Itertools;
    pub use crate::adaptors::{interleave, put_back};
    pub use crate::kmerge_impl::kmerge;
    pub use crate::merge_join::{merge, merge_join_by};
    pub use crate::multipeek_impl::multipeek;
    pub use crate::peek_nth::peek_nth;
    pub use crate::put_back_n_impl::put_back_n;
    pub use crate::rciter_impl::rciter;
    pub use crate::zip_eq_impl::zip_eq;
    /// Iterate `iterable` with a particular value inserted between each element.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::intersperse`].
    ///
    /// ```
    /// use itertools::intersperse;
    ///
    /// itertools::assert_equal(intersperse(0..3, 8), vec![0, 8, 1, 8, 2]);
    /// ```
    pub fn intersperse<I>(iterable: I, element: I::Item) -> Intersperse<I::IntoIter>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Clone,
    {
        Itertools::intersperse(iterable.into_iter(), element)
    }
    /// Iterate `iterable` with a particular value created by a function inserted
    /// between each element.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::intersperse_with`].
    ///
    /// ```
    /// use itertools::intersperse_with;
    ///
    /// let mut i = 10;
    /// itertools::assert_equal(intersperse_with(0..3, || { i -= 1; i }), vec![0, 9, 1, 8, 2]);
    /// assert_eq!(i, 8);
    /// ```
    pub fn intersperse_with<I, F>(
        iterable: I,
        element: F,
    ) -> IntersperseWith<I::IntoIter, F>
    where
        I: IntoIterator,
        F: FnMut() -> I::Item,
    {
        Itertools::intersperse_with(iterable.into_iter(), element)
    }
    /// Iterate `iterable` with a running index.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::enumerate`].
    ///
    /// ```
    /// use itertools::enumerate;
    ///
    /// for (i, elt) in enumerate(&[1, 2, 3]) {
    ///     /* loop body */
    ///     # let _ = (i, elt);
    /// }
    /// ```
    pub fn enumerate<I>(iterable: I) -> iter::Enumerate<I::IntoIter>
    where
        I: IntoIterator,
    {
        iterable.into_iter().enumerate()
    }
    /// Iterate `iterable` in reverse.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::rev`].
    ///
    /// ```
    /// use itertools::rev;
    ///
    /// for elt in rev(&[1, 2, 3]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    pub fn rev<I>(iterable: I) -> iter::Rev<I::IntoIter>
    where
        I: IntoIterator,
        I::IntoIter: DoubleEndedIterator,
    {
        iterable.into_iter().rev()
    }
    /// Converts the arguments to iterators and zips them.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::zip`].
    ///
    /// ## Example
    ///
    /// ```
    /// use itertools::zip;
    ///
    /// let mut result: Vec<(i32, char)> = Vec::new();
    ///
    /// for (a, b) in zip(&[1, 2, 3, 4, 5], &['a', 'b', 'c']) {
    ///     result.push((*a, *b));
    /// }
    /// assert_eq!(result, vec![(1, 'a'),(2, 'b'),(3, 'c')]);
    /// ```
    #[deprecated(
        note = "Use [std::iter::zip](https://doc.rust-lang.org/std/iter/fn.zip.html) instead",
        since = "0.10.4"
    )]
    pub fn zip<I, J>(i: I, j: J) -> Zip<I::IntoIter, J::IntoIter>
    where
        I: IntoIterator,
        J: IntoIterator,
    {
        i.into_iter().zip(j)
    }
    /// Takes two iterables and creates a new iterator over both in sequence.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::chain`].
    ///
    /// ## Example
    /// ```
    /// use itertools::chain;
    ///
    /// let mut result:Vec<i32> = Vec::new();
    ///
    /// for element in chain(&[1, 2, 3], &[4]) {
    ///     result.push(*element);
    /// }
    /// assert_eq!(result, vec![1, 2, 3, 4]);
    /// ```
    pub fn chain<I, J>(
        i: I,
        j: J,
    ) -> iter::Chain<<I as IntoIterator>::IntoIter, <J as IntoIterator>::IntoIter>
    where
        I: IntoIterator,
        J: IntoIterator<Item = I::Item>,
    {
        i.into_iter().chain(j)
    }
    /// Create an iterator that clones each element from `&T` to `T`.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::cloned`].
    ///
    /// ```
    /// use itertools::cloned;
    ///
    /// assert_eq!(cloned(b"abc").next(), Some(b'a'));
    /// ```
    pub fn cloned<'a, I, T>(iterable: I) -> iter::Cloned<I::IntoIter>
    where
        I: IntoIterator<Item = &'a T>,
        T: Clone + 'a,
    {
        iterable.into_iter().cloned()
    }
    /// Perform a fold operation over the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::fold`].
    ///
    /// ```
    /// use itertools::fold;
    ///
    /// assert_eq!(fold(&[1., 2., 3.], 0., |a, &b| f32::max(a, b)), 3.);
    /// ```
    pub fn fold<I, B, F>(iterable: I, init: B, f: F) -> B
    where
        I: IntoIterator,
        F: FnMut(B, I::Item) -> B,
    {
        iterable.into_iter().fold(init, f)
    }
    /// Test whether the predicate holds for all elements in the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::all`].
    ///
    /// ```
    /// use itertools::all;
    ///
    /// assert!(all(&[1, 2, 3], |elt| *elt > 0));
    /// ```
    pub fn all<I, F>(iterable: I, f: F) -> bool
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> bool,
    {
        iterable.into_iter().all(f)
    }
    /// Test whether the predicate holds for any elements in the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::any`].
    ///
    /// ```
    /// use itertools::any;
    ///
    /// assert!(any(&[0, -1, 2], |elt| *elt > 0));
    /// ```
    pub fn any<I, F>(iterable: I, f: F) -> bool
    where
        I: IntoIterator,
        F: FnMut(I::Item) -> bool,
    {
        iterable.into_iter().any(f)
    }
    /// Return the maximum value of the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::max`].
    ///
    /// ```
    /// use itertools::max;
    ///
    /// assert_eq!(max(0..10), Some(9));
    /// ```
    pub fn max<I>(iterable: I) -> Option<I::Item>
    where
        I: IntoIterator,
        I::Item: Ord,
    {
        iterable.into_iter().max()
    }
    /// Return the minimum value of the iterable.
    ///
    /// [`IntoIterator`] enabled version of [`Iterator::min`].
    ///
    /// ```
    /// use itertools::min;
    ///
    /// assert_eq!(min(0..10), Some(0));
    /// ```
    pub fn min<I>(iterable: I) -> Option<I::Item>
    where
        I: IntoIterator,
        I::Item: Ord,
    {
        iterable.into_iter().min()
    }
    /// Combine all iterator elements into one `String`, separated by `sep`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::join`].
    ///
    /// ```
    /// use itertools::join;
    ///
    /// assert_eq!(join(&[1, 2, 3], ", "), "1, 2, 3");
    /// ```
    pub fn join<I>(iterable: I, sep: &str) -> String
    where
        I: IntoIterator,
        I::Item: Display,
    {
        iterable.into_iter().join(sep)
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::sorted`].
    ///
    /// ```
    /// use itertools::sorted;
    /// use itertools::assert_equal;
    ///
    /// assert_equal(sorted("rust".chars()), "rstu".chars());
    /// ```
    pub fn sorted<I>(iterable: I) -> VecIntoIter<I::Item>
    where
        I: IntoIterator,
        I::Item: Ord,
    {
        iterable.into_iter().sorted()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::sorted_unstable`].
    ///
    /// ```
    /// use itertools::sorted_unstable;
    /// use itertools::assert_equal;
    ///
    /// assert_equal(sorted_unstable("rust".chars()), "rstu".chars());
    /// ```
    pub fn sorted_unstable<I>(iterable: I) -> VecIntoIter<I::Item>
    where
        I: IntoIterator,
        I::Item: Ord,
    {
        iterable.into_iter().sorted_unstable()
    }
}
#[doc(inline)]
pub use crate::free::*;
mod combinations {
    use core::array;
    use core::borrow::BorrowMut;
    use std::fmt;
    use std::iter::FusedIterator;
    use super::lazy_buffer::LazyBuffer;
    use alloc::vec::Vec;
    use crate::adaptors::checked_binomial;
    /// Iterator for `Vec` valued combinations returned by [`.combinations()`](crate::Itertools::combinations)
    pub type Combinations<I> = CombinationsGeneric<I, Vec<usize>>;
    /// Iterator for const generic combinations returned by [`.array_combinations()`](crate::Itertools::array_combinations)
    pub type ArrayCombinations<I, const K: usize> = CombinationsGeneric<I, [usize; K]>;
    /// Create a new `Combinations` from a clonable iterator.
    pub fn combinations<I: Iterator>(iter: I, k: usize) -> Combinations<I>
    where
        I::Item: Clone,
    {
        Combinations::new(iter, (0..k).collect())
    }
    /// Create a new `ArrayCombinations` from a clonable iterator.
    pub fn array_combinations<I: Iterator, const K: usize>(
        iter: I,
    ) -> ArrayCombinations<I, K>
    where
        I::Item: Clone,
    {
        ArrayCombinations::new(iter, array::from_fn(|i| i))
    }
    /// An iterator to iterate through all the `k`-length combinations in an iterator.
    ///
    /// See [`.combinations()`](crate::Itertools::combinations) and [`.array_combinations()`](crate::Itertools::array_combinations) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct CombinationsGeneric<I: Iterator, Idx> {
        indices: Idx,
        pool: LazyBuffer<I>,
        first: bool,
    }
    /// A type holding indices of elements in a pool or buffer of items from an inner iterator
    /// and used to pick out different combinations in a generic way.
    pub trait PoolIndex<T>: BorrowMut<[usize]> {
        type Item;
        fn extract_item<I: Iterator<Item = T>>(&self, pool: &LazyBuffer<I>) -> Self::Item
        where
            T: Clone;
        fn len(&self) -> usize {
            self.borrow().len()
        }
    }
    impl<T> PoolIndex<T> for Vec<usize> {
        type Item = Vec<T>;
        fn extract_item<I: Iterator<Item = T>>(&self, pool: &LazyBuffer<I>) -> Vec<T>
        where
            T: Clone,
        {
            pool.get_at(self)
        }
    }
    impl<T, const K: usize> PoolIndex<T> for [usize; K] {
        type Item = [T; K];
        fn extract_item<I: Iterator<Item = T>>(&self, pool: &LazyBuffer<I>) -> [T; K]
        where
            T: Clone,
        {
            pool.get_array(*self)
        }
    }
    impl<I, Idx> Clone for CombinationsGeneric<I, Idx>
    where
        I: Iterator + Clone,
        I::Item: Clone,
        Idx: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                indices: self.indices.clone(),
                pool: self.pool.clone(),
                first: self.first.clone(),
            }
        }
    }
    impl<I, Idx> fmt::Debug for CombinationsGeneric<I, Idx>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug,
        Idx: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Combinations")
                .field("indices", &self.indices)
                .field("pool", &self.pool)
                .field("first", &self.first)
                .finish()
        }
    }
    impl<I: Iterator, Idx: PoolIndex<I::Item>> CombinationsGeneric<I, Idx> {
        /// Constructor with arguments the inner iterator and the initial state for the indices.
        fn new(iter: I, indices: Idx) -> Self {
            Self {
                indices,
                pool: LazyBuffer::new(iter),
                first: true,
            }
        }
        /// Returns the length of a combination produced by this iterator.
        #[inline]
        pub fn k(&self) -> usize {
            self.indices.len()
        }
        /// Returns the (current) length of the pool from which combination elements are
        /// selected. This value can change between invocations of [`next`](Combinations::next).
        #[inline]
        pub fn n(&self) -> usize {
            self.pool.len()
        }
        /// Returns a reference to the source pool.
        #[inline]
        pub(crate) fn src(&self) -> &LazyBuffer<I> {
            &self.pool
        }
        /// Return the length of the inner iterator and the count of remaining combinations.
        pub(crate) fn n_and_count(self) -> (usize, usize) {
            let Self { indices, pool, first } = self;
            let n = pool.count();
            (n, remaining_for(n, first, indices.borrow()).unwrap())
        }
        /// Initialises the iterator by filling a buffer with elements from the
        /// iterator. Returns true if there are no combinations, false otherwise.
        fn init(&mut self) -> bool {
            self.pool.prefill(self.k());
            let done = self.k() > self.n();
            if !done {
                self.first = false;
            }
            done
        }
        /// Increments indices representing the combination to advance to the next
        /// (in lexicographic order by increasing sequence) combination. For example
        /// if we have n=4 & k=2 then `[0, 1] -> [0, 2] -> [0, 3] -> [1, 2] -> ...`
        ///
        /// Returns true if we've run out of combinations, false otherwise.
        fn increment_indices(&mut self) -> bool {
            let indices = self.indices.borrow_mut();
            if indices.is_empty() {
                return true;
            }
            let mut i: usize = indices.len() - 1;
            if indices[i] == self.pool.len() - 1 {
                self.pool.get_next();
            }
            while indices[i] == i + self.pool.len() - indices.len() {
                if i > 0 {
                    i -= 1;
                } else {
                    return true;
                }
            }
            indices[i] += 1;
            for j in i + 1..indices.len() {
                indices[j] = indices[j - 1] + 1;
            }
            false
        }
        /// Returns the n-th item or the number of successful steps.
        pub(crate) fn try_nth(
            &mut self,
            n: usize,
        ) -> Result<<Self as Iterator>::Item, usize>
        where
            I: Iterator,
            I::Item: Clone,
        {
            let done = if self.first { self.init() } else { self.increment_indices() };
            if done {
                return Err(0);
            }
            for i in 0..n {
                if self.increment_indices() {
                    return Err(i + 1);
                }
            }
            Ok(self.indices.extract_item(&self.pool))
        }
    }
    impl<I, Idx> Iterator for CombinationsGeneric<I, Idx>
    where
        I: Iterator,
        I::Item: Clone,
        Idx: PoolIndex<I::Item>,
    {
        type Item = Idx::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let done = if self.first { self.init() } else { self.increment_indices() };
            if done {
                return None;
            }
            Some(self.indices.extract_item(&self.pool))
        }
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.try_nth(n).ok()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (mut low, mut upp) = self.pool.size_hint();
            low = remaining_for(low, self.first, self.indices.borrow())
                .unwrap_or(usize::MAX);
            upp = upp
                .and_then(|upp| remaining_for(upp, self.first, self.indices.borrow()));
            (low, upp)
        }
        #[inline]
        fn count(self) -> usize {
            self.n_and_count().1
        }
    }
    impl<I, Idx> FusedIterator for CombinationsGeneric<I, Idx>
    where
        I: Iterator,
        I::Item: Clone,
        Idx: PoolIndex<I::Item>,
    {}
    impl<I: Iterator> Combinations<I> {
        /// Resets this `Combinations` back to an initial state for combinations of length
        /// `k` over the same pool data source. If `k` is larger than the current length
        /// of the data pool an attempt is made to prefill the pool so that it holds `k`
        /// elements.
        pub(crate) fn reset(&mut self, k: usize) {
            self.first = true;
            if k < self.indices.len() {
                self.indices.truncate(k);
                for i in 0..k {
                    self.indices[i] = i;
                }
            } else {
                for i in 0..self.indices.len() {
                    self.indices[i] = i;
                }
                self.indices.extend(self.indices.len()..k);
                self.pool.prefill(k);
            }
        }
    }
    /// For a given size `n`, return the count of remaining combinations or None if it would overflow.
    fn remaining_for(n: usize, first: bool, indices: &[usize]) -> Option<usize> {
        let k = indices.len();
        if n < k {
            Some(0)
        } else if first {
            checked_binomial(n, k)
        } else {
            indices
                .iter()
                .enumerate()
                .try_fold(
                    0usize,
                    |sum, (i, n0)| {
                        sum.checked_add(checked_binomial(n - 1 - *n0, k - i)?)
                    },
                )
        }
    }
}
mod combinations_with_replacement {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use std::fmt;
    use std::iter::FusedIterator;
    use super::lazy_buffer::LazyBuffer;
    use crate::adaptors::checked_binomial;
    /// An iterator to iterate through all the `n`-length combinations in an iterator, with replacement.
    ///
    /// See [`.combinations_with_replacement()`](crate::Itertools::combinations_with_replacement)
    /// for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        indices: Box<[usize]>,
        pool: LazyBuffer<I>,
        first: bool,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> CombinationsWithReplacement<I> {
            CombinationsWithReplacement {
                indices: ::core::clone::Clone::clone(&self.indices),
                pool: ::core::clone::Clone::clone(&self.pool),
                first: ::core::clone::Clone::clone(&self.first),
            }
        }
    }
    impl<I> fmt::Debug for CombinationsWithReplacement<I>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug + Clone,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("CombinationsWithReplacement")
                .field("indices", &self.indices)
                .field("pool", &self.pool)
                .field("first", &self.first)
                .finish()
        }
    }
    /// Create a new `CombinationsWithReplacement` from a clonable iterator.
    pub fn combinations_with_replacement<I>(
        iter: I,
        k: usize,
    ) -> CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        let indices = ::alloc::vec::from_elem(0, k).into_boxed_slice();
        let pool: LazyBuffer<I> = LazyBuffer::new(iter);
        CombinationsWithReplacement {
            indices,
            pool,
            first: true,
        }
    }
    impl<I> CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        /// Increments indices representing the combination to advance to the next
        /// (in lexicographic order by increasing sequence) combination.
        ///
        /// Returns true if we've run out of combinations, false otherwise.
        fn increment_indices(&mut self) -> bool {
            self.pool.get_next();
            let mut increment = None;
            for (i, indices_int) in self.indices.iter().enumerate().rev() {
                if *indices_int < self.pool.len() - 1 {
                    increment = Some((i, indices_int + 1));
                    break;
                }
            }
            match increment {
                Some((increment_from, increment_value)) => {
                    self.indices[increment_from..].fill(increment_value);
                    false
                }
                None => true,
            }
        }
    }
    impl<I> Iterator for CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        type Item = Vec<I::Item>;
        fn next(&mut self) -> Option<Self::Item> {
            if self.first {
                if !(self.indices.is_empty() || self.pool.get_next()) {
                    return None;
                }
                self.first = false;
            } else if self.increment_indices() {
                return None;
            }
            Some(self.pool.get_at(&self.indices))
        }
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            if self.first {
                if !(self.indices.is_empty() || self.pool.get_next()) {
                    return None;
                }
                self.first = false;
            } else if self.increment_indices() {
                return None;
            }
            for _ in 0..n {
                if self.increment_indices() {
                    return None;
                }
            }
            Some(self.pool.get_at(&self.indices))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (mut low, mut upp) = self.pool.size_hint();
            low = remaining_for(low, self.first, &self.indices).unwrap_or(usize::MAX);
            upp = upp.and_then(|upp| remaining_for(upp, self.first, &self.indices));
            (low, upp)
        }
        fn count(self) -> usize {
            let Self { indices, pool, first } = self;
            let n = pool.count();
            remaining_for(n, first, &indices).unwrap()
        }
    }
    impl<I> FusedIterator for CombinationsWithReplacement<I>
    where
        I: Iterator,
        I::Item: Clone,
    {}
    /// For a given size `n`, return the count of remaining combinations with replacement or None if it would overflow.
    fn remaining_for(n: usize, first: bool, indices: &[usize]) -> Option<usize> {
        let count = |n: usize, k: usize| {
            let positions = if n == 0 {
                k.saturating_sub(1)
            } else {
                (n - 1).checked_add(k)?
            };
            checked_binomial(positions, k)
        };
        let k = indices.len();
        if first {
            count(n, k)
        } else {
            indices
                .iter()
                .enumerate()
                .try_fold(
                    0usize,
                    |sum, (i, n0)| { sum.checked_add(count(n - 1 - *n0, k - i)?) },
                )
        }
    }
}
mod concat_impl {
    /// Combine all an iterator's elements into one element by using [`Extend`].
    ///
    /// [`IntoIterator`]-enabled version of [`Itertools::concat`](crate::Itertools::concat).
    ///
    /// This combinator will extend the first item with each of the rest of the
    /// items of the iterator. If the iterator is empty, the default value of
    /// `I::Item` is returned.
    ///
    /// ```rust
    /// use itertools::concat;
    ///
    /// let input = vec![vec![1], vec![2, 3], vec![4, 5, 6]];
    /// assert_eq!(concat(input), vec![1, 2, 3, 4, 5, 6]);
    /// ```
    pub fn concat<I>(iterable: I) -> I::Item
    where
        I: IntoIterator,
        I::Item: Extend<<<I as IntoIterator>::Item as IntoIterator>::Item> + IntoIterator
            + Default,
    {
        iterable
            .into_iter()
            .reduce(|mut a, b| {
                a.extend(b);
                a
            })
            .unwrap_or_default()
    }
}
mod cons_tuples_impl {
    use crate::adaptors::map::{MapSpecialCase, MapSpecialCaseFn};
    #[allow(non_snake_case)]
    impl<K, L, X> MapSpecialCaseFn<((K, L), X)> for ConsTuplesFn {
        type Out = (K, L, X);
        fn call(&mut self, ((K, L), X): ((K, L), X)) -> Self::Out {
            (K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<J, K, L, X> MapSpecialCaseFn<((J, K, L), X)> for ConsTuplesFn {
        type Out = (J, K, L, X);
        fn call(&mut self, ((J, K, L), X): ((J, K, L), X)) -> Self::Out {
            (J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<I, J, K, L, X> MapSpecialCaseFn<((I, J, K, L), X)> for ConsTuplesFn {
        type Out = (I, J, K, L, X);
        fn call(&mut self, ((I, J, K, L), X): ((I, J, K, L), X)) -> Self::Out {
            (I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<H, I, J, K, L, X> MapSpecialCaseFn<((H, I, J, K, L), X)> for ConsTuplesFn {
        type Out = (H, I, J, K, L, X);
        fn call(&mut self, ((H, I, J, K, L), X): ((H, I, J, K, L), X)) -> Self::Out {
            (H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<G, H, I, J, K, L, X> MapSpecialCaseFn<((G, H, I, J, K, L), X)>
    for ConsTuplesFn {
        type Out = (G, H, I, J, K, L, X);
        fn call(
            &mut self,
            ((G, H, I, J, K, L), X): ((G, H, I, J, K, L), X),
        ) -> Self::Out {
            (G, H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<F, G, H, I, J, K, L, X> MapSpecialCaseFn<((F, G, H, I, J, K, L), X)>
    for ConsTuplesFn {
        type Out = (F, G, H, I, J, K, L, X);
        fn call(
            &mut self,
            ((F, G, H, I, J, K, L), X): ((F, G, H, I, J, K, L), X),
        ) -> Self::Out {
            (F, G, H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<E, F, G, H, I, J, K, L, X> MapSpecialCaseFn<((E, F, G, H, I, J, K, L), X)>
    for ConsTuplesFn {
        type Out = (E, F, G, H, I, J, K, L, X);
        fn call(
            &mut self,
            ((E, F, G, H, I, J, K, L), X): ((E, F, G, H, I, J, K, L), X),
        ) -> Self::Out {
            (E, F, G, H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<D, E, F, G, H, I, J, K, L, X> MapSpecialCaseFn<((D, E, F, G, H, I, J, K, L), X)>
    for ConsTuplesFn {
        type Out = (D, E, F, G, H, I, J, K, L, X);
        fn call(
            &mut self,
            ((D, E, F, G, H, I, J, K, L), X): ((D, E, F, G, H, I, J, K, L), X),
        ) -> Self::Out {
            (D, E, F, G, H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        X,
    > MapSpecialCaseFn<((C, D, E, F, G, H, I, J, K, L), X)> for ConsTuplesFn {
        type Out = (C, D, E, F, G, H, I, J, K, L, X);
        fn call(
            &mut self,
            ((C, D, E, F, G, H, I, J, K, L), X): ((C, D, E, F, G, H, I, J, K, L), X),
        ) -> Self::Out {
            (C, D, E, F, G, H, I, J, K, L, X)
        }
    }
    #[allow(non_snake_case)]
    impl<
        B,
        C,
        D,
        E,
        F,
        G,
        H,
        I,
        J,
        K,
        L,
        X,
    > MapSpecialCaseFn<((B, C, D, E, F, G, H, I, J, K, L), X)> for ConsTuplesFn {
        type Out = (B, C, D, E, F, G, H, I, J, K, L, X);
        fn call(
            &mut self,
            (
                (B, C, D, E, F, G, H, I, J, K, L),
                X,
            ): ((B, C, D, E, F, G, H, I, J, K, L), X),
        ) -> Self::Out {
            (B, C, D, E, F, G, H, I, J, K, L, X)
        }
    }
    pub struct ConsTuplesFn;
    #[automatically_derived]
    impl ::core::fmt::Debug for ConsTuplesFn {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "ConsTuplesFn")
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ConsTuplesFn {
        #[inline]
        fn clone(&self) -> ConsTuplesFn {
            ConsTuplesFn
        }
    }
    /// An iterator that maps an iterator of tuples like
    /// `((A, B), C)` to an iterator of `(A, B, C)`.
    ///
    /// Used by the `iproduct!()` macro.
    pub type ConsTuples<I> = MapSpecialCase<I, ConsTuplesFn>;
    /// Create an iterator that maps for example iterators of
    /// `((A, B), C)` to `(A, B, C)`.
    pub fn cons_tuples<I>(iterable: I) -> ConsTuples<I::IntoIter>
    where
        I: IntoIterator,
    {
        ConsTuples {
            iter: iterable.into_iter(),
            f: ConsTuplesFn,
        }
    }
}
mod diff {
    //! "Diff"ing iterators for caching elements to sequential collections without requiring the new
    //! elements' iterator to be `Clone`.
    //!
    //! [`Diff`] (produced by the [`diff_with`] function)
    //! describes the difference between two non-`Clone` iterators `I` and `J` after breaking ASAP from
    //! a lock-step comparison.
    use std::fmt;
    use crate::free::put_back;
    use crate::structs::PutBack;
    /// A type returned by the [`diff_with`] function.
    ///
    /// `Diff` represents the way in which the elements yielded by the iterator `I` differ to some
    /// iterator `J`.
    pub enum Diff<I, J>
    where
        I: Iterator,
        J: Iterator,
    {
        /// The index of the first non-matching element along with both iterator's remaining elements
        /// starting with the first mis-match.
        FirstMismatch(usize, PutBack<I>, PutBack<J>),
        /// The total number of elements that were in `J` along with the remaining elements of `I`.
        Shorter(usize, PutBack<I>),
        /// The total number of elements that were in `I` along with the remaining elements of `J`.
        Longer(usize, PutBack<J>),
    }
    impl<I, J> fmt::Debug for Diff<I, J>
    where
        I: Iterator,
        J: Iterator,
        PutBack<I>: fmt::Debug,
        PutBack<J>: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::FirstMismatch(idx, i, j) => {
                    f.debug_tuple("FirstMismatch").field(idx).field(i).field(j).finish()
                }
                Self::Shorter(idx, i) => {
                    f.debug_tuple("Shorter").field(idx).field(i).finish()
                }
                Self::Longer(idx, j) => {
                    f.debug_tuple("Longer").field(idx).field(j).finish()
                }
            }
        }
    }
    impl<I, J> Clone for Diff<I, J>
    where
        I: Iterator,
        J: Iterator,
        PutBack<I>: Clone,
        PutBack<J>: Clone,
    {
        fn clone(&self) -> Self {
            match self {
                Self::FirstMismatch(idx, i, j) => {
                    Self::FirstMismatch(*idx, i.clone(), j.clone())
                }
                Self::Shorter(idx, i) => Self::Shorter(*idx, i.clone()),
                Self::Longer(idx, j) => Self::Longer(*idx, j.clone()),
            }
        }
    }
    /// Compares every element yielded by both `i` and `j` with the given function in lock-step and
    /// returns a [`Diff`] which describes how `j` differs from `i`.
    ///
    /// If the number of elements yielded by `j` is less than the number of elements yielded by `i`,
    /// the number of `j` elements yielded will be returned along with `i`'s remaining elements as
    /// `Diff::Shorter`.
    ///
    /// If the two elements of a step differ, the index of those elements along with the remaining
    /// elements of both `i` and `j` are returned as `Diff::FirstMismatch`.
    ///
    /// If `i` becomes exhausted before `j` becomes exhausted, the number of elements in `i` along with
    /// the remaining `j` elements will be returned as `Diff::Longer`.
    pub fn diff_with<I, J, F>(
        i: I,
        j: J,
        mut is_equal: F,
    ) -> Option<Diff<I::IntoIter, J::IntoIter>>
    where
        I: IntoIterator,
        J: IntoIterator,
        F: FnMut(&I::Item, &J::Item) -> bool,
    {
        let mut i = i.into_iter();
        let mut j = j.into_iter();
        let mut idx = 0;
        while let Some(i_elem) = i.next() {
            match j.next() {
                None => return Some(Diff::Shorter(idx, put_back(i).with_value(i_elem))),
                Some(j_elem) => {
                    if !is_equal(&i_elem, &j_elem) {
                        let remaining_i = put_back(i).with_value(i_elem);
                        let remaining_j = put_back(j).with_value(j_elem);
                        return Some(Diff::FirstMismatch(idx, remaining_i, remaining_j));
                    }
                }
            }
            idx += 1;
        }
        j.next().map(|j_elem| Diff::Longer(idx, put_back(j).with_value(j_elem)))
    }
}
mod duplicates_impl {
    use std::hash::Hash;
    mod private {
        use std::collections::HashMap;
        use std::fmt;
        use std::hash::Hash;
        #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
        pub struct DuplicatesBy<I: Iterator, Key, F> {
            pub(crate) iter: I,
            pub(crate) meta: Meta<Key, F>,
        }
        #[automatically_derived]
        impl<
            I: ::core::clone::Clone + Iterator,
            Key: ::core::clone::Clone,
            F: ::core::clone::Clone,
        > ::core::clone::Clone for DuplicatesBy<I, Key, F> {
            #[inline]
            fn clone(&self) -> DuplicatesBy<I, Key, F> {
                DuplicatesBy {
                    iter: ::core::clone::Clone::clone(&self.iter),
                    meta: ::core::clone::Clone::clone(&self.meta),
                }
            }
        }
        impl<I, V, F> fmt::Debug for DuplicatesBy<I, V, F>
        where
            I: Iterator + fmt::Debug,
            V: fmt::Debug + Hash + Eq,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("DuplicatesBy")
                    .field("iter", &self.iter)
                    .field("meta.used", &self.meta.used)
                    .finish()
            }
        }
        impl<I: Iterator, Key: Eq + Hash, F> DuplicatesBy<I, Key, F> {
            pub(crate) fn new(iter: I, key_method: F) -> Self {
                Self {
                    iter,
                    meta: Meta {
                        used: HashMap::new(),
                        pending: 0,
                        key_method,
                    },
                }
            }
        }
        pub struct Meta<Key, F> {
            used: HashMap<Key, bool>,
            pending: usize,
            key_method: F,
        }
        #[automatically_derived]
        impl<Key: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
        for Meta<Key, F> {
            #[inline]
            fn clone(&self) -> Meta<Key, F> {
                Meta {
                    used: ::core::clone::Clone::clone(&self.used),
                    pending: ::core::clone::Clone::clone(&self.pending),
                    key_method: ::core::clone::Clone::clone(&self.key_method),
                }
            }
        }
        impl<Key, F> Meta<Key, F>
        where
            Key: Eq + Hash,
        {
            /// Takes an item and returns it back to the caller if it's the second time we see it.
            /// Otherwise the item is consumed and None is returned
            #[inline(always)]
            fn filter<I>(&mut self, item: I) -> Option<I>
            where
                F: KeyMethod<Key, I>,
            {
                let kv = self.key_method.make(item);
                match self.used.get_mut(kv.key_ref()) {
                    None => {
                        self.used.insert(kv.key(), false);
                        self.pending += 1;
                        None
                    }
                    Some(true) => None,
                    Some(produced) => {
                        *produced = true;
                        self.pending -= 1;
                        Some(kv.value())
                    }
                }
            }
        }
        impl<I, Key, F> Iterator for DuplicatesBy<I, Key, F>
        where
            I: Iterator,
            Key: Eq + Hash,
            F: KeyMethod<Key, I::Item>,
        {
            type Item = I::Item;
            fn next(&mut self) -> Option<Self::Item> {
                let Self { iter, meta } = self;
                iter.find_map(|v| meta.filter(v))
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let (_, hi) = self.iter.size_hint();
                let hi = hi
                    .map(|hi| {
                        if hi <= self.meta.pending {
                            hi
                        } else {
                            self.meta.pending + (hi - self.meta.pending) / 2
                        }
                    });
                (0, hi)
            }
        }
        impl<I, Key, F> DoubleEndedIterator for DuplicatesBy<I, Key, F>
        where
            I: DoubleEndedIterator,
            Key: Eq + Hash,
            F: KeyMethod<Key, I::Item>,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                let Self { iter, meta } = self;
                iter.rev().find_map(|v| meta.filter(v))
            }
        }
        /// A keying method for use with `DuplicatesBy`
        pub trait KeyMethod<K, V> {
            type Container: KeyXorValue<K, V>;
            fn make(&mut self, value: V) -> Self::Container;
        }
        /// Apply the identity function to elements before checking them for equality.
        pub struct ById;
        #[automatically_derived]
        impl ::core::fmt::Debug for ById {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "ById")
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for ById {
            #[inline]
            fn clone(&self) -> ById {
                ById
            }
        }
        impl<V> KeyMethod<V, V> for ById {
            type Container = JustValue<V>;
            fn make(&mut self, v: V) -> Self::Container {
                JustValue(v)
            }
        }
        /// Apply a user-supplied function to elements before checking them for equality.
        pub struct ByFn<F>(pub(crate) F);
        #[automatically_derived]
        impl<F: ::core::clone::Clone> ::core::clone::Clone for ByFn<F> {
            #[inline]
            fn clone(&self) -> ByFn<F> {
                ByFn(::core::clone::Clone::clone(&self.0))
            }
        }
        impl<F> fmt::Debug for ByFn<F> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.debug_struct("ByFn").finish()
            }
        }
        impl<K, V, F> KeyMethod<K, V> for ByFn<F>
        where
            F: FnMut(&V) -> K,
        {
            type Container = KeyValue<K, V>;
            fn make(&mut self, v: V) -> Self::Container {
                KeyValue((self.0)(&v), v)
            }
        }
        pub trait KeyXorValue<K, V> {
            fn key_ref(&self) -> &K;
            fn key(self) -> K;
            fn value(self) -> V;
        }
        pub struct KeyValue<K, V>(K, V);
        #[automatically_derived]
        impl<K: ::core::fmt::Debug, V: ::core::fmt::Debug> ::core::fmt::Debug
        for KeyValue<K, V> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field2_finish(
                    f,
                    "KeyValue",
                    &self.0,
                    &&self.1,
                )
            }
        }
        impl<K, V> KeyXorValue<K, V> for KeyValue<K, V> {
            fn key_ref(&self) -> &K {
                &self.0
            }
            fn key(self) -> K {
                self.0
            }
            fn value(self) -> V {
                self.1
            }
        }
        pub struct JustValue<V>(V);
        #[automatically_derived]
        impl<V: ::core::fmt::Debug> ::core::fmt::Debug for JustValue<V> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "JustValue",
                    &&self.0,
                )
            }
        }
        impl<V> KeyXorValue<V, V> for JustValue<V> {
            fn key_ref(&self) -> &V {
                &self.0
            }
            fn key(self) -> V {
                self.0
            }
            fn value(self) -> V {
                self.0
            }
        }
    }
    /// An iterator adapter to filter for duplicate elements.
    ///
    /// See [`.duplicates_by()`](crate::Itertools::duplicates_by) for more information.
    pub type DuplicatesBy<I, V, F> = private::DuplicatesBy<I, V, private::ByFn<F>>;
    /// Create a new `DuplicatesBy` iterator.
    pub fn duplicates_by<I, Key, F>(iter: I, f: F) -> DuplicatesBy<I, Key, F>
    where
        Key: Eq + Hash,
        F: FnMut(&I::Item) -> Key,
        I: Iterator,
    {
        DuplicatesBy::new(iter, private::ByFn(f))
    }
    /// An iterator adapter to filter out duplicate elements.
    ///
    /// See [`.duplicates()`](crate::Itertools::duplicates) for more information.
    pub type Duplicates<I> = private::DuplicatesBy<
        I,
        <I as Iterator>::Item,
        private::ById,
    >;
    /// Create a new `Duplicates` iterator.
    pub fn duplicates<I>(iter: I) -> Duplicates<I>
    where
        I: Iterator,
        I::Item: Eq + Hash,
    {
        Duplicates::new(iter, private::ById)
    }
}
mod exactly_one_err {
    use std::error::Error;
    use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
    use std::iter::ExactSizeIterator;
    use either::Either;
    use crate::size_hint;
    /// Iterator returned for the error case of `Itertools::exactly_one()`
    /// This iterator yields exactly the same elements as the input iterator.
    ///
    /// During the execution of `exactly_one` the iterator must be mutated.  This wrapper
    /// effectively "restores" the state of the input iterator when it's handed back.
    ///
    /// This is very similar to `PutBackN` except this iterator only supports 0-2 elements and does not
    /// use a `Vec`.
    pub struct ExactlyOneError<I>
    where
        I: Iterator,
    {
        first_two: Option<Either<[I::Item; 2], I::Item>>,
        inner: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for ExactlyOneError<I>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> ExactlyOneError<I> {
            ExactlyOneError {
                first_two: ::core::clone::Clone::clone(&self.first_two),
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    impl<I> ExactlyOneError<I>
    where
        I: Iterator,
    {
        /// Creates a new `ExactlyOneErr` iterator.
        pub(crate) fn new(
            first_two: Option<Either<[I::Item; 2], I::Item>>,
            inner: I,
        ) -> Self {
            Self { first_two, inner }
        }
        fn additional_len(&self) -> usize {
            match self.first_two {
                Some(Either::Left(_)) => 2,
                Some(Either::Right(_)) => 1,
                None => 0,
            }
        }
    }
    impl<I> Iterator for ExactlyOneError<I>
    where
        I: Iterator,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            match self.first_two.take() {
                Some(Either::Left([first, second])) => {
                    self.first_two = Some(Either::Right(second));
                    Some(first)
                }
                Some(Either::Right(second)) => Some(second),
                None => self.inner.next(),
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.inner.size_hint(), self.additional_len())
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            match self.first_two {
                Some(Either::Left([first, second])) => {
                    init = f(init, first);
                    init = f(init, second);
                }
                Some(Either::Right(second)) => init = f(init, second),
                None => {}
            }
            self.inner.fold(init, f)
        }
    }
    impl<I> ExactSizeIterator for ExactlyOneError<I>
    where
        I: ExactSizeIterator,
    {}
    impl<I> Display for ExactlyOneError<I>
    where
        I: Iterator,
    {
        fn fmt(&self, f: &mut Formatter) -> FmtResult {
            let additional = self.additional_len();
            if additional > 0 {
                f.write_fmt(
                    format_args!("got at least 2 elements when exactly one was expected"),
                )
            } else {
                f.write_fmt(
                    format_args!("got zero elements when exactly one was expected"),
                )
            }
        }
    }
    impl<I> Debug for ExactlyOneError<I>
    where
        I: Iterator + Debug,
        I::Item: Debug,
    {
        fn fmt(&self, f: &mut Formatter) -> FmtResult {
            let mut dbg = f.debug_struct("ExactlyOneError");
            match &self.first_two {
                Some(Either::Left([first, second])) => {
                    dbg.field("first", first).field("second", second);
                }
                Some(Either::Right(second)) => {
                    dbg.field("second", second);
                }
                None => {}
            }
            dbg.field("inner", &self.inner).finish()
        }
    }
    impl<I> Error for ExactlyOneError<I>
    where
        I: Iterator + Debug,
        I::Item: Debug,
    {}
}
mod extrema_set {
    use alloc::{vec, vec::Vec};
    use std::cmp::Ordering;
    /// Implementation guts for `min_set`, `min_set_by`, and `min_set_by_key`.
    pub fn min_set_impl<I, K, F, Compare>(
        mut it: I,
        mut key_for: F,
        mut compare: Compare,
    ) -> Vec<I::Item>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> K,
        Compare: FnMut(&I::Item, &I::Item, &K, &K) -> Ordering,
    {
        match it.next() {
            None => Vec::new(),
            Some(element) => {
                let mut current_key = key_for(&element);
                let mut result = <[_]>::into_vec(::alloc::boxed::box_new([element]));
                it.for_each(|element| {
                    let key = key_for(&element);
                    match compare(&element, &result[0], &key, &current_key) {
                        Ordering::Less => {
                            result.clear();
                            result.push(element);
                            current_key = key;
                        }
                        Ordering::Equal => {
                            result.push(element);
                        }
                        Ordering::Greater => {}
                    }
                });
                result
            }
        }
    }
    /// Implementation guts for `ax_set`, `max_set_by`, and `max_set_by_key`.
    pub fn max_set_impl<I, K, F, Compare>(
        it: I,
        key_for: F,
        mut compare: Compare,
    ) -> Vec<I::Item>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> K,
        Compare: FnMut(&I::Item, &I::Item, &K, &K) -> Ordering,
    {
        min_set_impl(
            it,
            key_for,
            |it1, it2, key1, key2| { compare(it2, it1, key2, key1) },
        )
    }
}
mod flatten_ok {
    use crate::size_hint;
    use std::{fmt, iter::{DoubleEndedIterator, FusedIterator}};
    pub fn flatten_ok<I, T, E>(iter: I) -> FlattenOk<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
        T: IntoIterator,
    {
        FlattenOk {
            iter,
            inner_front: None,
            inner_back: None,
        }
    }
    /// An iterator adaptor that flattens `Result::Ok` values and
    /// allows `Result::Err` values through unchanged.
    ///
    /// See [`.flatten_ok()`](crate::Itertools::flatten_ok) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct FlattenOk<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
        T: IntoIterator,
    {
        iter: I,
        inner_front: Option<T::IntoIter>,
        inner_back: Option<T::IntoIter>,
    }
    impl<I, T, E> Iterator for FlattenOk<I, T, E>
    where
        I: Iterator<Item = Result<T, E>>,
        T: IntoIterator,
    {
        type Item = Result<T::Item, E>;
        fn next(&mut self) -> Option<Self::Item> {
            loop {
                if let Some(inner) = &mut self.inner_front {
                    if let Some(item) = inner.next() {
                        return Some(Ok(item));
                    }
                    self.inner_front = None;
                }
                match self.iter.next() {
                    Some(Ok(ok)) => self.inner_front = Some(ok.into_iter()),
                    Some(Err(e)) => return Some(Err(e)),
                    None => {
                        if let Some(inner) = &mut self.inner_back {
                            if let Some(item) = inner.next() {
                                return Some(Ok(item));
                            }
                            self.inner_back = None;
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let mut acc = match self.inner_front {
                Some(x) => x.fold(init, |a, o| f(a, Ok(o))),
                None => init,
            };
            acc = self
                .iter
                .fold(
                    acc,
                    |acc, x| match x {
                        Ok(it) => it.into_iter().fold(acc, |a, o| f(a, Ok(o))),
                        Err(e) => f(acc, Err(e)),
                    },
                );
            match self.inner_back {
                Some(x) => x.fold(acc, |a, o| f(a, Ok(o))),
                None => acc,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let inner_hint = |inner: &Option<T::IntoIter>| {
                inner.as_ref().map(Iterator::size_hint).unwrap_or((0, Some(0)))
            };
            let inner_front = inner_hint(&self.inner_front);
            let inner_back = inner_hint(&self.inner_back);
            let outer = match self.iter.size_hint() {
                (0, Some(0)) => (0, Some(0)),
                _ => (0, None),
            };
            size_hint::add(size_hint::add(inner_front, inner_back), outer)
        }
    }
    impl<I, T, E> DoubleEndedIterator for FlattenOk<I, T, E>
    where
        I: DoubleEndedIterator<Item = Result<T, E>>,
        T: IntoIterator,
        T::IntoIter: DoubleEndedIterator,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            loop {
                if let Some(inner) = &mut self.inner_back {
                    if let Some(item) = inner.next_back() {
                        return Some(Ok(item));
                    }
                    self.inner_back = None;
                }
                match self.iter.next_back() {
                    Some(Ok(ok)) => self.inner_back = Some(ok.into_iter()),
                    Some(Err(e)) => return Some(Err(e)),
                    None => {
                        if let Some(inner) = &mut self.inner_front {
                            if let Some(item) = inner.next_back() {
                                return Some(Ok(item));
                            }
                            self.inner_front = None;
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        fn rfold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let mut acc = match self.inner_back {
                Some(x) => x.rfold(init, |a, o| f(a, Ok(o))),
                None => init,
            };
            acc = self
                .iter
                .rfold(
                    acc,
                    |acc, x| match x {
                        Ok(it) => it.into_iter().rfold(acc, |a, o| f(a, Ok(o))),
                        Err(e) => f(acc, Err(e)),
                    },
                );
            match self.inner_front {
                Some(x) => x.rfold(acc, |a, o| f(a, Ok(o))),
                None => acc,
            }
        }
    }
    impl<I, T, E> Clone for FlattenOk<I, T, E>
    where
        I: Iterator<Item = Result<T, E>> + Clone,
        T: IntoIterator,
        T::IntoIter: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                iter: self.iter.clone(),
                inner_front: self.inner_front.clone(),
                inner_back: self.inner_back.clone(),
            }
        }
    }
    impl<I, T, E> fmt::Debug for FlattenOk<I, T, E>
    where
        I: Iterator<Item = Result<T, E>> + fmt::Debug,
        T: IntoIterator,
        T::IntoIter: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FlattenOk")
                .field("iter", &self.iter)
                .field("inner_front", &self.inner_front)
                .field("inner_back", &self.inner_back)
                .finish()
        }
    }
    /// Only the iterator being flattened needs to implement [`FusedIterator`].
    impl<I, T, E> FusedIterator for FlattenOk<I, T, E>
    where
        I: FusedIterator<Item = Result<T, E>>,
        T: IntoIterator,
    {}
}
mod format {
    use std::cell::Cell;
    use std::fmt;
    /// Format all iterator elements lazily, separated by `sep`.
    ///
    /// The format value can only be formatted once, after that the iterator is
    /// exhausted.
    ///
    /// See [`.format_with()`](crate::Itertools::format_with) for more information.
    pub struct FormatWith<'a, I, F> {
        sep: &'a str,
        /// `FormatWith` uses interior mutability because `Display::fmt` takes `&self`.
        inner: Cell<Option<(I, F)>>,
    }
    /// Format all iterator elements lazily, separated by `sep`.
    ///
    /// The format value can only be formatted once, after that the iterator is
    /// exhausted.
    ///
    /// See [`.format()`](crate::Itertools::format)
    /// for more information.
    pub struct Format<'a, I> {
        sep: &'a str,
        /// `Format` uses interior mutability because `Display::fmt` takes `&self`.
        inner: Cell<Option<I>>,
    }
    pub fn new_format<I, F>(iter: I, separator: &str, f: F) -> FormatWith<'_, I, F>
    where
        I: Iterator,
        F: FnMut(
            I::Item,
            &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result,
        ) -> fmt::Result,
    {
        FormatWith {
            sep: separator,
            inner: Cell::new(Some((iter, f))),
        }
    }
    pub fn new_format_default<I>(iter: I, separator: &str) -> Format<'_, I>
    where
        I: Iterator,
    {
        Format {
            sep: separator,
            inner: Cell::new(Some(iter)),
        }
    }
    impl<I, F> fmt::Display for FormatWith<'_, I, F>
    where
        I: Iterator,
        F: FnMut(
            I::Item,
            &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result,
        ) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let (mut iter, mut format) = match self.inner.take() {
                Some(t) => t,
                None => {
                    ::std::rt::begin_panic("FormatWith: was already formatted once");
                }
            };
            if let Some(fst) = iter.next() {
                format(fst, &mut |disp: &dyn fmt::Display| disp.fmt(f))?;
                iter.try_for_each(|elt| {
                    if !self.sep.is_empty() {
                        f.write_str(self.sep)?;
                    }
                    format(elt, &mut |disp: &dyn fmt::Display| disp.fmt(f))
                })?;
            }
            Ok(())
        }
    }
    impl<I, F> fmt::Debug for FormatWith<'_, I, F>
    where
        I: Iterator,
        F: FnMut(
            I::Item,
            &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result,
        ) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl<I> Format<'_, I>
    where
        I: Iterator,
    {
        fn format(
            &self,
            f: &mut fmt::Formatter,
            cb: fn(&I::Item, &mut fmt::Formatter) -> fmt::Result,
        ) -> fmt::Result {
            let mut iter = match self.inner.take() {
                Some(t) => t,
                None => {
                    ::std::rt::begin_panic("Format: was already formatted once");
                }
            };
            if let Some(fst) = iter.next() {
                cb(&fst, f)?;
                iter.try_for_each(|elt| {
                    if !self.sep.is_empty() {
                        f.write_str(self.sep)?;
                    }
                    cb(&elt, f)
                })?;
            }
            Ok(())
        }
    }
    impl<'a, I> fmt::Display for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::Display,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::Display::fmt)
        }
    }
    impl<'a, I> fmt::Debug for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::Debug::fmt)
        }
    }
    impl<'a, I> fmt::UpperExp for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::UpperExp,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::UpperExp::fmt)
        }
    }
    impl<'a, I> fmt::LowerExp for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::LowerExp,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::LowerExp::fmt)
        }
    }
    impl<'a, I> fmt::UpperHex for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::UpperHex,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::UpperHex::fmt)
        }
    }
    impl<'a, I> fmt::LowerHex for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::LowerHex,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::LowerHex::fmt)
        }
    }
    impl<'a, I> fmt::Octal for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::Octal,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::Octal::fmt)
        }
    }
    impl<'a, I> fmt::Binary for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::Binary,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::Binary::fmt)
        }
    }
    impl<'a, I> fmt::Pointer for Format<'a, I>
    where
        I: Iterator,
        I::Item: fmt::Pointer,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.format(f, fmt::Pointer::fmt)
        }
    }
    impl<I, F> Clone for FormatWith<'_, I, F>
    where
        (I, F): Clone,
    {
        fn clone(&self) -> Self {
            struct PutBackOnDrop<'r, 'a, I, F> {
                into: &'r FormatWith<'a, I, F>,
                inner: Option<(I, F)>,
            }
            impl<I, F> Drop for PutBackOnDrop<'_, '_, I, F> {
                fn drop(&mut self) {
                    self.into.inner.set(self.inner.take())
                }
            }
            let pbod = PutBackOnDrop {
                inner: self.inner.take(),
                into: self,
            };
            Self {
                inner: Cell::new(pbod.inner.clone()),
                sep: self.sep,
            }
        }
    }
    impl<I> Clone for Format<'_, I>
    where
        I: Clone,
    {
        fn clone(&self) -> Self {
            struct PutBackOnDrop<'r, 'a, I> {
                into: &'r Format<'a, I>,
                inner: Option<I>,
            }
            impl<I> Drop for PutBackOnDrop<'_, '_, I> {
                fn drop(&mut self) {
                    self.into.inner.set(self.inner.take())
                }
            }
            let pbod = PutBackOnDrop {
                inner: self.inner.take(),
                into: self,
            };
            Self {
                inner: Cell::new(pbod.inner.clone()),
                sep: self.sep,
            }
        }
    }
}
mod group_map {
    use std::collections::HashMap;
    use std::hash::Hash;
    use std::iter::Iterator;
    /// Return a `HashMap` of keys mapped to a list of their corresponding values.
    ///
    /// See [`.into_group_map()`](crate::Itertools::into_group_map)
    /// for more information.
    pub fn into_group_map<I, K, V>(iter: I) -> HashMap<K, Vec<V>>
    where
        I: Iterator<Item = (K, V)>,
        K: Hash + Eq,
    {
        let mut lookup = HashMap::new();
        iter.for_each(|(key, val)| {
            lookup.entry(key).or_insert_with(Vec::new).push(val);
        });
        lookup
    }
    pub fn into_group_map_by<I, K, V, F>(iter: I, mut f: F) -> HashMap<K, Vec<V>>
    where
        I: Iterator<Item = V>,
        K: Hash + Eq,
        F: FnMut(&V) -> K,
    {
        into_group_map(iter.map(|v| (f(&v), v)))
    }
}
mod groupbylazy {
    use alloc::vec::{self, Vec};
    use std::cell::{Cell, RefCell};
    /// A trait to unify `FnMut` for `ChunkBy` with the chunk key in `IntoChunks`
    trait KeyFunction<A> {
        type Key;
        fn call_mut(&mut self, arg: A) -> Self::Key;
    }
    impl<A, K, F> KeyFunction<A> for F
    where
        F: FnMut(A) -> K + ?Sized,
    {
        type Key = K;
        #[inline]
        fn call_mut(&mut self, arg: A) -> Self::Key {
            (*self)(arg)
        }
    }
    /// `ChunkIndex` acts like the grouping key function for `IntoChunks`
    struct ChunkIndex {
        size: usize,
        index: usize,
        key: usize,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ChunkIndex {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "ChunkIndex",
                "size",
                &self.size,
                "index",
                &self.index,
                "key",
                &&self.key,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ChunkIndex {
        #[inline]
        fn clone(&self) -> ChunkIndex {
            ChunkIndex {
                size: ::core::clone::Clone::clone(&self.size),
                index: ::core::clone::Clone::clone(&self.index),
                key: ::core::clone::Clone::clone(&self.key),
            }
        }
    }
    impl ChunkIndex {
        #[inline(always)]
        fn new(size: usize) -> Self {
            Self { size, index: 0, key: 0 }
        }
    }
    impl<A> KeyFunction<A> for ChunkIndex {
        type Key = usize;
        #[inline(always)]
        fn call_mut(&mut self, _arg: A) -> Self::Key {
            if self.index == self.size {
                self.key += 1;
                self.index = 0;
            }
            self.index += 1;
            self.key
        }
    }
    struct GroupInner<K, I, F>
    where
        I: Iterator,
    {
        key: F,
        iter: I,
        current_key: Option<K>,
        current_elt: Option<I::Item>,
        /// flag set if iterator is exhausted
        done: bool,
        /// Index of group we are currently buffering or visiting
        top_group: usize,
        /// Least index for which we still have elements buffered
        oldest_buffered_group: usize,
        /// Group index for `buffer[0]` -- the slots
        /// `bottom_group..oldest_buffered_group` are unused and will be erased when
        /// that range is large enough.
        bottom_group: usize,
        /// Buffered groups, from `bottom_group` (index 0) to `top_group`.
        buffer: Vec<vec::IntoIter<I::Item>>,
        /// index of last group iter that was dropped,
        /// `usize::MAX` initially when no group was dropped
        dropped_group: usize,
    }
    #[automatically_derived]
    impl<
        K: ::core::clone::Clone,
        I: ::core::clone::Clone,
        F: ::core::clone::Clone,
    > ::core::clone::Clone for GroupInner<K, I, F>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> GroupInner<K, I, F> {
            GroupInner {
                key: ::core::clone::Clone::clone(&self.key),
                iter: ::core::clone::Clone::clone(&self.iter),
                current_key: ::core::clone::Clone::clone(&self.current_key),
                current_elt: ::core::clone::Clone::clone(&self.current_elt),
                done: ::core::clone::Clone::clone(&self.done),
                top_group: ::core::clone::Clone::clone(&self.top_group),
                oldest_buffered_group: ::core::clone::Clone::clone(
                    &self.oldest_buffered_group,
                ),
                bottom_group: ::core::clone::Clone::clone(&self.bottom_group),
                buffer: ::core::clone::Clone::clone(&self.buffer),
                dropped_group: ::core::clone::Clone::clone(&self.dropped_group),
            }
        }
    }
    impl<K, I, F> GroupInner<K, I, F>
    where
        I: Iterator,
        F: for<'a> KeyFunction<&'a I::Item, Key = K>,
        K: PartialEq,
    {
        /// `client`: Index of group that requests next element
        #[inline(always)]
        fn step(&mut self, client: usize) -> Option<I::Item> {
            if client < self.oldest_buffered_group {
                None
            } else if client < self.top_group
                || (client == self.top_group
                    && self.buffer.len() > self.top_group - self.bottom_group)
            {
                self.lookup_buffer(client)
            } else if self.done {
                None
            } else if self.top_group == client {
                self.step_current()
            } else {
                self.step_buffering(client)
            }
        }
        #[inline(never)]
        fn lookup_buffer(&mut self, client: usize) -> Option<I::Item> {
            let bufidx = client - self.bottom_group;
            if client < self.oldest_buffered_group {
                return None;
            }
            let elt = self.buffer.get_mut(bufidx).and_then(|queue| queue.next());
            if elt.is_none() && client == self.oldest_buffered_group {
                self.oldest_buffered_group += 1;
                while self
                    .buffer
                    .get(self.oldest_buffered_group - self.bottom_group)
                    .map_or(false, |buf| buf.len() == 0)
                {
                    self.oldest_buffered_group += 1;
                }
                let nclear = self.oldest_buffered_group - self.bottom_group;
                if nclear > 0 && nclear >= self.buffer.len() / 2 {
                    let mut i = 0;
                    self.buffer
                        .retain(|buf| {
                            i += 1;
                            if true {
                                if !(buf.len() == 0 || i > nclear) {
                                    ::core::panicking::panic(
                                        "assertion failed: buf.len() == 0 || i > nclear",
                                    )
                                }
                            }
                            i > nclear
                        });
                    self.bottom_group = self.oldest_buffered_group;
                }
            }
            elt
        }
        /// Take the next element from the iterator, and set the done
        /// flag if exhausted. Must not be called after done.
        #[inline(always)]
        fn next_element(&mut self) -> Option<I::Item> {
            if true {
                if !!self.done {
                    ::core::panicking::panic("assertion failed: !self.done")
                }
            }
            match self.iter.next() {
                None => {
                    self.done = true;
                    None
                }
                otherwise => otherwise,
            }
        }
        #[inline(never)]
        fn step_buffering(&mut self, client: usize) -> Option<I::Item> {
            if true {
                if !(self.top_group + 1 == client) {
                    ::core::panicking::panic(
                        "assertion failed: self.top_group + 1 == client",
                    )
                }
            }
            let mut group = Vec::new();
            if let Some(elt) = self.current_elt.take() {
                if self.top_group != self.dropped_group {
                    group.push(elt);
                }
            }
            let mut first_elt = None;
            while let Some(elt) = self.next_element() {
                let key = self.key.call_mut(&elt);
                match self.current_key.take() {
                    None => {}
                    Some(old_key) => {
                        if old_key != key {
                            self.current_key = Some(key);
                            first_elt = Some(elt);
                            break;
                        }
                    }
                }
                self.current_key = Some(key);
                if self.top_group != self.dropped_group {
                    group.push(elt);
                }
            }
            if self.top_group != self.dropped_group {
                self.push_next_group(group);
            }
            if first_elt.is_some() {
                self.top_group += 1;
                if true {
                    if !(self.top_group == client) {
                        ::core::panicking::panic(
                            "assertion failed: self.top_group == client",
                        )
                    }
                }
            }
            first_elt
        }
        fn push_next_group(&mut self, group: Vec<I::Item>) {
            while self.top_group - self.bottom_group > self.buffer.len() {
                if self.buffer.is_empty() {
                    self.bottom_group += 1;
                    self.oldest_buffered_group += 1;
                } else {
                    self.buffer.push(Vec::new().into_iter());
                }
            }
            self.buffer.push(group.into_iter());
            if true {
                if !(self.top_group + 1 - self.bottom_group == self.buffer.len()) {
                    ::core::panicking::panic(
                        "assertion failed: self.top_group + 1 - self.bottom_group == self.buffer.len()",
                    )
                }
            }
        }
        /// This is the immediate case, where we use no buffering
        #[inline]
        fn step_current(&mut self) -> Option<I::Item> {
            if true {
                if !!self.done {
                    ::core::panicking::panic("assertion failed: !self.done")
                }
            }
            if let elt @ Some(..) = self.current_elt.take() {
                return elt;
            }
            match self.next_element() {
                None => None,
                Some(elt) => {
                    let key = self.key.call_mut(&elt);
                    match self.current_key.take() {
                        None => {}
                        Some(old_key) => {
                            if old_key != key {
                                self.current_key = Some(key);
                                self.current_elt = Some(elt);
                                self.top_group += 1;
                                return None;
                            }
                        }
                    }
                    self.current_key = Some(key);
                    Some(elt)
                }
            }
        }
        /// Request the just started groups' key.
        ///
        /// `client`: Index of group
        ///
        /// **Panics** if no group key is available.
        fn group_key(&mut self, client: usize) -> K {
            if true {
                if !!self.done {
                    ::core::panicking::panic("assertion failed: !self.done")
                }
            }
            if true {
                if !(client == self.top_group) {
                    ::core::panicking::panic(
                        "assertion failed: client == self.top_group",
                    )
                }
            }
            if true {
                if !self.current_key.is_some() {
                    ::core::panicking::panic(
                        "assertion failed: self.current_key.is_some()",
                    )
                }
            }
            if true {
                if !self.current_elt.is_none() {
                    ::core::panicking::panic(
                        "assertion failed: self.current_elt.is_none()",
                    )
                }
            }
            let old_key = self.current_key.take().unwrap();
            if let Some(elt) = self.next_element() {
                let key = self.key.call_mut(&elt);
                if old_key != key {
                    self.top_group += 1;
                }
                self.current_key = Some(key);
                self.current_elt = Some(elt);
            }
            old_key
        }
    }
    impl<K, I, F> GroupInner<K, I, F>
    where
        I: Iterator,
    {
        /// Called when a group is dropped
        fn drop_group(&mut self, client: usize) {
            if self.dropped_group == !0 || client > self.dropped_group {
                self.dropped_group = client;
            }
        }
    }
    #[deprecated(note = "Use `ChunkBy` instead", since = "0.13.0")]
    /// See [`ChunkBy`](crate::structs::ChunkBy).
    pub type GroupBy<K, I, F> = ChunkBy<K, I, F>;
    /// `ChunkBy` is the storage for the lazy grouping operation.
    ///
    /// If the groups are consumed in their original order, or if each
    /// group is dropped without keeping it around, then `ChunkBy` uses
    /// no allocations. It needs allocations only if several group iterators
    /// are alive at the same time.
    ///
    /// This type implements [`IntoIterator`] (it is **not** an iterator
    /// itself), because the group iterators need to borrow from this
    /// value. It should be stored in a local variable or temporary and
    /// iterated.
    ///
    /// See [`.chunk_by()`](crate::Itertools::chunk_by) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct ChunkBy<K, I, F>
    where
        I: Iterator,
    {
        inner: RefCell<GroupInner<K, I, F>>,
        index: Cell<usize>,
    }
    /// Create a new
    pub fn new<K, J, F>(iter: J, f: F) -> ChunkBy<K, J::IntoIter, F>
    where
        J: IntoIterator,
        F: FnMut(&J::Item) -> K,
    {
        ChunkBy {
            inner: RefCell::new(GroupInner {
                key: f,
                iter: iter.into_iter(),
                current_key: None,
                current_elt: None,
                done: false,
                top_group: 0,
                oldest_buffered_group: 0,
                bottom_group: 0,
                buffer: Vec::new(),
                dropped_group: !0,
            }),
            index: Cell::new(0),
        }
    }
    impl<K, I, F> ChunkBy<K, I, F>
    where
        I: Iterator,
    {
        /// `client`: Index of group that requests next element
        fn step(&self, client: usize) -> Option<I::Item>
        where
            F: FnMut(&I::Item) -> K,
            K: PartialEq,
        {
            self.inner.borrow_mut().step(client)
        }
        /// `client`: Index of group
        fn drop_group(&self, client: usize) {
            self.inner.borrow_mut().drop_group(client);
        }
    }
    impl<'a, K, I, F> IntoIterator for &'a ChunkBy<K, I, F>
    where
        I: Iterator,
        I::Item: 'a,
        F: FnMut(&I::Item) -> K,
        K: PartialEq,
    {
        type Item = (K, Group<'a, K, I, F>);
        type IntoIter = Groups<'a, K, I, F>;
        fn into_iter(self) -> Self::IntoIter {
            Groups { parent: self }
        }
    }
    /// An iterator that yields the Group iterators.
    ///
    /// Iterator element type is `(K, Group)`:
    /// the group's key `K` and the group's iterator.
    ///
    /// See [`.chunk_by()`](crate::Itertools::chunk_by) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Groups<'a, K, I, F>
    where
        I: Iterator + 'a,
        I::Item: 'a,
        K: 'a,
        F: 'a,
    {
        parent: &'a ChunkBy<K, I, F>,
    }
    impl<'a, K, I, F> Iterator for Groups<'a, K, I, F>
    where
        I: Iterator,
        I::Item: 'a,
        F: FnMut(&I::Item) -> K,
        K: PartialEq,
    {
        type Item = (K, Group<'a, K, I, F>);
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let index = self.parent.index.get();
            self.parent.index.set(index + 1);
            let inner = &mut *self.parent.inner.borrow_mut();
            inner
                .step(index)
                .map(|elt| {
                    let key = inner.group_key(index);
                    (
                        key,
                        Group {
                            parent: self.parent,
                            index,
                            first: Some(elt),
                        },
                    )
                })
        }
    }
    /// An iterator for the elements in a single group.
    ///
    /// Iterator element type is `I::Item`.
    pub struct Group<'a, K, I, F>
    where
        I: Iterator + 'a,
        I::Item: 'a,
        K: 'a,
        F: 'a,
    {
        parent: &'a ChunkBy<K, I, F>,
        index: usize,
        first: Option<I::Item>,
    }
    impl<'a, K, I, F> Drop for Group<'a, K, I, F>
    where
        I: Iterator,
        I::Item: 'a,
    {
        fn drop(&mut self) {
            self.parent.drop_group(self.index);
        }
    }
    impl<'a, K, I, F> Iterator for Group<'a, K, I, F>
    where
        I: Iterator,
        I::Item: 'a,
        F: FnMut(&I::Item) -> K,
        K: PartialEq,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let elt @ Some(..) = self.first.take() {
                return elt;
            }
            self.parent.step(self.index)
        }
    }
    /// Create a new
    pub fn new_chunks<J>(iter: J, size: usize) -> IntoChunks<J::IntoIter>
    where
        J: IntoIterator,
    {
        IntoChunks {
            inner: RefCell::new(GroupInner {
                key: ChunkIndex::new(size),
                iter: iter.into_iter(),
                current_key: None,
                current_elt: None,
                done: false,
                top_group: 0,
                oldest_buffered_group: 0,
                bottom_group: 0,
                buffer: Vec::new(),
                dropped_group: !0,
            }),
            index: Cell::new(0),
        }
    }
    /// `ChunkLazy` is the storage for a lazy chunking operation.
    ///
    /// `IntoChunks` behaves just like `ChunkBy`: it is iterable, and
    /// it only buffers if several chunk iterators are alive at the same time.
    ///
    /// This type implements [`IntoIterator`] (it is **not** an iterator
    /// itself), because the chunk iterators need to borrow from this
    /// value. It should be stored in a local variable or temporary and
    /// iterated.
    ///
    /// Iterator element type is `Chunk`, each chunk's iterator.
    ///
    /// See [`.chunks()`](crate::Itertools::chunks) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct IntoChunks<I>
    where
        I: Iterator,
    {
        inner: RefCell<GroupInner<usize, I, ChunkIndex>>,
        index: Cell<usize>,
    }
    impl<I> Clone for IntoChunks<I>
    where
        I: Clone + Iterator,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                index: self.index.clone(),
            }
        }
    }
    impl<I> IntoChunks<I>
    where
        I: Iterator,
    {
        /// `client`: Index of chunk that requests next element
        fn step(&self, client: usize) -> Option<I::Item> {
            self.inner.borrow_mut().step(client)
        }
        /// `client`: Index of chunk
        fn drop_group(&self, client: usize) {
            self.inner.borrow_mut().drop_group(client);
        }
    }
    impl<'a, I> IntoIterator for &'a IntoChunks<I>
    where
        I: Iterator,
        I::Item: 'a,
    {
        type Item = Chunk<'a, I>;
        type IntoIter = Chunks<'a, I>;
        fn into_iter(self) -> Self::IntoIter {
            Chunks { parent: self }
        }
    }
    /// An iterator that yields the Chunk iterators.
    ///
    /// Iterator element type is `Chunk`.
    ///
    /// See [`.chunks()`](crate::Itertools::chunks) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Chunks<'a, I>
    where
        I: Iterator + 'a,
        I::Item: 'a,
    {
        parent: &'a IntoChunks<I>,
    }
    #[automatically_derived]
    impl<'a, I: ::core::clone::Clone> ::core::clone::Clone for Chunks<'a, I>
    where
        I: Iterator + 'a,
        I::Item: 'a,
    {
        #[inline]
        fn clone(&self) -> Chunks<'a, I> {
            Chunks {
                parent: ::core::clone::Clone::clone(&self.parent),
            }
        }
    }
    impl<'a, I> Iterator for Chunks<'a, I>
    where
        I: Iterator,
        I::Item: 'a,
    {
        type Item = Chunk<'a, I>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let index = self.parent.index.get();
            self.parent.index.set(index + 1);
            let inner = &mut *self.parent.inner.borrow_mut();
            inner
                .step(index)
                .map(|elt| Chunk {
                    parent: self.parent,
                    index,
                    first: Some(elt),
                })
        }
    }
    /// An iterator for the elements in a single chunk.
    ///
    /// Iterator element type is `I::Item`.
    pub struct Chunk<'a, I>
    where
        I: Iterator + 'a,
        I::Item: 'a,
    {
        parent: &'a IntoChunks<I>,
        index: usize,
        first: Option<I::Item>,
    }
    impl<'a, I> Drop for Chunk<'a, I>
    where
        I: Iterator,
        I::Item: 'a,
    {
        fn drop(&mut self) {
            self.parent.drop_group(self.index);
        }
    }
    impl<'a, I> Iterator for Chunk<'a, I>
    where
        I: Iterator,
        I::Item: 'a,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let elt @ Some(..) = self.first.take() {
                return elt;
            }
            self.parent.step(self.index)
        }
    }
}
mod grouping_map {
    use crate::{
        adaptors::map::{MapSpecialCase, MapSpecialCaseFn},
        MinMaxResult,
    };
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::hash::Hash;
    use std::iter::Iterator;
    use std::ops::{Add, Mul};
    /// A wrapper to allow for an easy [`into_grouping_map_by`](crate::Itertools::into_grouping_map_by)
    pub type MapForGrouping<I, F> = MapSpecialCase<I, GroupingMapFn<F>>;
    pub struct GroupingMapFn<F>(F);
    #[automatically_derived]
    impl<F: ::core::clone::Clone> ::core::clone::Clone for GroupingMapFn<F> {
        #[inline]
        fn clone(&self) -> GroupingMapFn<F> {
            GroupingMapFn(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<F> std::fmt::Debug for GroupingMapFn<F> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("GroupingMapFn").finish()
        }
    }
    impl<V, K, F: FnMut(&V) -> K> MapSpecialCaseFn<V> for GroupingMapFn<F> {
        type Out = (K, V);
        fn call(&mut self, v: V) -> Self::Out {
            ((self.0)(&v), v)
        }
    }
    pub(crate) fn new_map_for_grouping<K, I: Iterator, F: FnMut(&I::Item) -> K>(
        iter: I,
        key_mapper: F,
    ) -> MapForGrouping<I, F> {
        MapSpecialCase {
            iter,
            f: GroupingMapFn(key_mapper),
        }
    }
    /// Creates a new `GroupingMap` from `iter`
    pub fn new<I, K, V>(iter: I) -> GroupingMap<I>
    where
        I: Iterator<Item = (K, V)>,
        K: Hash + Eq,
    {
        GroupingMap { iter }
    }
    /// `GroupingMapBy` is an intermediate struct for efficient group-and-fold operations.
    ///
    /// See [`GroupingMap`] for more informations.
    pub type GroupingMapBy<I, F> = GroupingMap<MapForGrouping<I, F>>;
    /// `GroupingMap` is an intermediate struct for efficient group-and-fold operations.
    /// It groups elements by their key and at the same time fold each group
    /// using some aggregating operation.
    ///
    /// No method on this struct performs temporary allocations.
    #[must_use = "GroupingMap is lazy and do nothing unless consumed"]
    pub struct GroupingMap<I> {
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for GroupingMap<I> {
        #[inline]
        fn clone(&self) -> GroupingMap<I> {
            GroupingMap {
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for GroupingMap<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "GroupingMap",
                "iter",
                &&self.iter,
            )
        }
    }
    impl<I, K, V> GroupingMap<I>
    where
        I: Iterator<Item = (K, V)>,
        K: Hash + Eq,
    {
        /// This is the generic way to perform any operation on a `GroupingMap`.
        /// It's suggested to use this method only to implement custom operations
        /// when the already provided ones are not enough.
        ///
        /// Groups elements from the `GroupingMap` source by key and applies `operation` to the elements
        /// of each group sequentially, passing the previously accumulated value, a reference to the key
        /// and the current element as arguments, and stores the results in an `HashMap`.
        ///
        /// The `operation` function is invoked on each element with the following parameters:
        ///  - the current value of the accumulator of the group if there is currently one;
        ///  - a reference to the key of the group this element belongs to;
        ///  - the element from the source being aggregated;
        ///
        /// If `operation` returns `Some(element)` then the accumulator is updated with `element`,
        /// otherwise the previous accumulation is discarded.
        ///
        /// Return a `HashMap` associating the key of each group with the result of aggregation of
        /// that group's elements. If the aggregation of the last element of a group discards the
        /// accumulator then there won't be an entry associated to that group's key.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let data = vec![2, 8, 5, 7, 9, 0, 4, 10];
        /// let lookup = data.into_iter()
        ///     .into_grouping_map_by(|&n| n % 4)
        ///     .aggregate(|acc, _key, val| {
        ///         if val == 0 || val == 10 {
        ///             None
        ///         } else {
        ///             Some(acc.unwrap_or(0) + val)
        ///         }
        ///     });
        ///
        /// assert_eq!(lookup[&0], 4);        // 0 resets the accumulator so only 4 is summed
        /// assert_eq!(lookup[&1], 5 + 9);
        /// assert_eq!(lookup.get(&2), None); // 10 resets the accumulator and nothing is summed afterward
        /// assert_eq!(lookup[&3], 7);
        /// assert_eq!(lookup.len(), 3);      // The final keys are only 0, 1 and 2
        /// ```
        pub fn aggregate<FO, R>(self, mut operation: FO) -> HashMap<K, R>
        where
            FO: FnMut(Option<R>, &K, V) -> Option<R>,
        {
            let mut destination_map = HashMap::new();
            self.iter
                .for_each(|(key, val)| {
                    let acc = destination_map.remove(&key);
                    if let Some(op_res) = operation(acc, &key, val) {
                        destination_map.insert(key, op_res);
                    }
                });
            destination_map
        }
        /// Groups elements from the `GroupingMap` source by key and applies `operation` to the elements
        /// of each group sequentially, passing the previously accumulated value, a reference to the key
        /// and the current element as arguments, and stores the results in a new map.
        ///
        /// `init` is called to obtain the initial value of each accumulator.
        ///
        /// `operation` is a function that is invoked on each element with the following parameters:
        ///  - the current value of the accumulator of the group;
        ///  - a reference to the key of the group this element belongs to;
        ///  - the element from the source being accumulated.
        ///
        /// Return a `HashMap` associating the key of each group with the result of folding that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// #[derive(Debug, Default)]
        /// struct Accumulator {
        ///   acc: usize,
        /// }
        ///
        /// let lookup = (1..=7)
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .fold_with(|_key, _val| Default::default(), |Accumulator { acc }, _key, val| {
        ///         let acc = acc + val;
        ///         Accumulator { acc }
        ///      });
        ///
        /// assert_eq!(lookup[&0].acc, 3 + 6);
        /// assert_eq!(lookup[&1].acc, 1 + 4 + 7);
        /// assert_eq!(lookup[&2].acc, 2 + 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn fold_with<FI, FO, R>(
            self,
            mut init: FI,
            mut operation: FO,
        ) -> HashMap<K, R>
        where
            FI: FnMut(&K, &V) -> R,
            FO: FnMut(R, &K, V) -> R,
        {
            self.aggregate(|acc, key, val| {
                let acc = acc.unwrap_or_else(|| init(key, &val));
                Some(operation(acc, key, val))
            })
        }
        /// Groups elements from the `GroupingMap` source by key and applies `operation` to the elements
        /// of each group sequentially, passing the previously accumulated value, a reference to the key
        /// and the current element as arguments, and stores the results in a new map.
        ///
        /// `init` is the value from which will be cloned the initial value of each accumulator.
        ///
        /// `operation` is a function that is invoked on each element with the following parameters:
        ///  - the current value of the accumulator of the group;
        ///  - a reference to the key of the group this element belongs to;
        ///  - the element from the source being accumulated.
        ///
        /// Return a `HashMap` associating the key of each group with the result of folding that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = (1..=7)
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .fold(0, |acc, _key, val| acc + val);
        ///
        /// assert_eq!(lookup[&0], 3 + 6);
        /// assert_eq!(lookup[&1], 1 + 4 + 7);
        /// assert_eq!(lookup[&2], 2 + 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn fold<FO, R>(self, init: R, operation: FO) -> HashMap<K, R>
        where
            R: Clone,
            FO: FnMut(R, &K, V) -> R,
        {
            self.fold_with(|_, _| init.clone(), operation)
        }
        /// Groups elements from the `GroupingMap` source by key and applies `operation` to the elements
        /// of each group sequentially, passing the previously accumulated value, a reference to the key
        /// and the current element as arguments, and stores the results in a new map.
        ///
        /// This is similar to [`fold`] but the initial value of the accumulator is the first element of the group.
        ///
        /// `operation` is a function that is invoked on each element with the following parameters:
        ///  - the current value of the accumulator of the group;
        ///  - a reference to the key of the group this element belongs to;
        ///  - the element from the source being accumulated.
        ///
        /// Return a `HashMap` associating the key of each group with the result of folding that group's elements.
        ///
        /// [`fold`]: GroupingMap::fold
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = (1..=7)
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .reduce(|acc, _key, val| acc + val);
        ///
        /// assert_eq!(lookup[&0], 3 + 6);
        /// assert_eq!(lookup[&1], 1 + 4 + 7);
        /// assert_eq!(lookup[&2], 2 + 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn reduce<FO>(self, mut operation: FO) -> HashMap<K, V>
        where
            FO: FnMut(V, &K, V) -> V,
        {
            self.aggregate(|acc, key, val| {
                Some(
                    match acc {
                        Some(acc) => operation(acc, key, val),
                        None => val,
                    },
                )
            })
        }
        /// See [`.reduce()`](GroupingMap::reduce).
        #[deprecated(note = "Use .reduce() instead", since = "0.13.0")]
        pub fn fold_first<FO>(self, operation: FO) -> HashMap<K, V>
        where
            FO: FnMut(V, &K, V) -> V,
        {
            self.reduce(operation)
        }
        /// Groups elements from the `GroupingMap` source by key and collects the elements of each group in
        /// an instance of `C`. The iteration order is preserved when inserting elements.
        ///
        /// Return a `HashMap` associating the key of each group with the collection containing that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        /// use std::collections::HashSet;
        ///
        /// let lookup = vec![0, 1, 2, 3, 4, 5, 6, 2, 3, 6].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .collect::<HashSet<_>>();
        ///
        /// assert_eq!(lookup[&0], vec![0, 3, 6].into_iter().collect::<HashSet<_>>());
        /// assert_eq!(lookup[&1], vec![1, 4].into_iter().collect::<HashSet<_>>());
        /// assert_eq!(lookup[&2], vec![2, 5].into_iter().collect::<HashSet<_>>());
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn collect<C>(self) -> HashMap<K, C>
        where
            C: Default + Extend<V>,
        {
            let mut destination_map = HashMap::new();
            self.iter
                .for_each(|(key, val)| {
                    destination_map
                        .entry(key)
                        .or_insert_with(C::default)
                        .extend(Some(val));
                });
            destination_map
        }
        /// Groups elements from the `GroupingMap` source by key and finds the maximum of each group.
        ///
        /// If several elements are equally maximum, the last element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .max();
        ///
        /// assert_eq!(lookup[&0], 12);
        /// assert_eq!(lookup[&1], 7);
        /// assert_eq!(lookup[&2], 8);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn max(self) -> HashMap<K, V>
        where
            V: Ord,
        {
            self.max_by(|_, v1, v2| V::cmp(v1, v2))
        }
        /// Groups elements from the `GroupingMap` source by key and finds the maximum of each group
        /// with respect to the specified comparison function.
        ///
        /// If several elements are equally maximum, the last element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .max_by(|_key, x, y| y.cmp(x));
        ///
        /// assert_eq!(lookup[&0], 3);
        /// assert_eq!(lookup[&1], 1);
        /// assert_eq!(lookup[&2], 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn max_by<F>(self, mut compare: F) -> HashMap<K, V>
        where
            F: FnMut(&K, &V, &V) -> Ordering,
        {
            self.reduce(|acc, key, val| match compare(key, &acc, &val) {
                Ordering::Less | Ordering::Equal => val,
                Ordering::Greater => acc,
            })
        }
        /// Groups elements from the `GroupingMap` source by key and finds the element of each group
        /// that gives the maximum from the specified function.
        ///
        /// If several elements are equally maximum, the last element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .max_by_key(|_key, &val| val % 4);
        ///
        /// assert_eq!(lookup[&0], 3);
        /// assert_eq!(lookup[&1], 7);
        /// assert_eq!(lookup[&2], 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn max_by_key<F, CK>(self, mut f: F) -> HashMap<K, V>
        where
            F: FnMut(&K, &V) -> CK,
            CK: Ord,
        {
            self.max_by(|key, v1, v2| f(key, v1).cmp(&f(key, v2)))
        }
        /// Groups elements from the `GroupingMap` source by key and finds the minimum of each group.
        ///
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .min();
        ///
        /// assert_eq!(lookup[&0], 3);
        /// assert_eq!(lookup[&1], 1);
        /// assert_eq!(lookup[&2], 5);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn min(self) -> HashMap<K, V>
        where
            V: Ord,
        {
            self.min_by(|_, v1, v2| V::cmp(v1, v2))
        }
        /// Groups elements from the `GroupingMap` source by key and finds the minimum of each group
        /// with respect to the specified comparison function.
        ///
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .min_by(|_key, x, y| y.cmp(x));
        ///
        /// assert_eq!(lookup[&0], 12);
        /// assert_eq!(lookup[&1], 7);
        /// assert_eq!(lookup[&2], 8);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn min_by<F>(self, mut compare: F) -> HashMap<K, V>
        where
            F: FnMut(&K, &V, &V) -> Ordering,
        {
            self.reduce(|acc, key, val| match compare(key, &acc, &val) {
                Ordering::Less | Ordering::Equal => acc,
                Ordering::Greater => val,
            })
        }
        /// Groups elements from the `GroupingMap` source by key and finds the element of each group
        /// that gives the minimum from the specified function.
        ///
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .min_by_key(|_key, &val| val % 4);
        ///
        /// assert_eq!(lookup[&0], 12);
        /// assert_eq!(lookup[&1], 4);
        /// assert_eq!(lookup[&2], 8);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn min_by_key<F, CK>(self, mut f: F) -> HashMap<K, V>
        where
            F: FnMut(&K, &V) -> CK,
            CK: Ord,
        {
            self.min_by(|key, v1, v2| f(key, v1).cmp(&f(key, v2)))
        }
        /// Groups elements from the `GroupingMap` source by key and find the maximum and minimum of
        /// each group.
        ///
        /// If several elements are equally maximum, the last element is picked.
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// See [`Itertools::minmax`](crate::Itertools::minmax) for the non-grouping version.
        ///
        /// Differences from the non grouping version:
        /// - It never produces a `MinMaxResult::NoElements`
        /// - It doesn't have any speedup
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum and maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        /// use itertools::MinMaxResult::{OneElement, MinMax};
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .minmax();
        ///
        /// assert_eq!(lookup[&0], MinMax(3, 12));
        /// assert_eq!(lookup[&1], MinMax(1, 7));
        /// assert_eq!(lookup[&2], OneElement(5));
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn minmax(self) -> HashMap<K, MinMaxResult<V>>
        where
            V: Ord,
        {
            self.minmax_by(|_, v1, v2| V::cmp(v1, v2))
        }
        /// Groups elements from the `GroupingMap` source by key and find the maximum and minimum of
        /// each group with respect to the specified comparison function.
        ///
        /// If several elements are equally maximum, the last element is picked.
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// It has the same differences from the non-grouping version as `minmax`.
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum and maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        /// use itertools::MinMaxResult::{OneElement, MinMax};
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .minmax_by(|_key, x, y| y.cmp(x));
        ///
        /// assert_eq!(lookup[&0], MinMax(12, 3));
        /// assert_eq!(lookup[&1], MinMax(7, 1));
        /// assert_eq!(lookup[&2], OneElement(5));
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn minmax_by<F>(self, mut compare: F) -> HashMap<K, MinMaxResult<V>>
        where
            F: FnMut(&K, &V, &V) -> Ordering,
        {
            self.aggregate(|acc, key, val| {
                Some(
                    match acc {
                        Some(MinMaxResult::OneElement(e)) => {
                            if compare(key, &val, &e) == Ordering::Less {
                                MinMaxResult::MinMax(val, e)
                            } else {
                                MinMaxResult::MinMax(e, val)
                            }
                        }
                        Some(MinMaxResult::MinMax(min, max)) => {
                            if compare(key, &val, &min) == Ordering::Less {
                                MinMaxResult::MinMax(val, max)
                            } else if compare(key, &val, &max) != Ordering::Less {
                                MinMaxResult::MinMax(min, val)
                            } else {
                                MinMaxResult::MinMax(min, max)
                            }
                        }
                        None => MinMaxResult::OneElement(val),
                        Some(MinMaxResult::NoElements) => {
                            ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            )
                        }
                    },
                )
            })
        }
        /// Groups elements from the `GroupingMap` source by key and find the elements of each group
        /// that gives the minimum and maximum from the specified function.
        ///
        /// If several elements are equally maximum, the last element is picked.
        /// If several elements are equally minimum, the first element is picked.
        ///
        /// It has the same differences from the non-grouping version as `minmax`.
        ///
        /// Returns a `HashMap` associating the key of each group with the minimum and maximum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        /// use itertools::MinMaxResult::{OneElement, MinMax};
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .minmax_by_key(|_key, &val| val % 4);
        ///
        /// assert_eq!(lookup[&0], MinMax(12, 3));
        /// assert_eq!(lookup[&1], MinMax(4, 7));
        /// assert_eq!(lookup[&2], OneElement(5));
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn minmax_by_key<F, CK>(self, mut f: F) -> HashMap<K, MinMaxResult<V>>
        where
            F: FnMut(&K, &V) -> CK,
            CK: Ord,
        {
            self.minmax_by(|key, v1, v2| f(key, v1).cmp(&f(key, v2)))
        }
        /// Groups elements from the `GroupingMap` source by key and sums them.
        ///
        /// This is just a shorthand for `self.reduce(|acc, _, val| acc + val)`.
        /// It is more limited than `Iterator::sum` since it doesn't use the `Sum` trait.
        ///
        /// Returns a `HashMap` associating the key of each group with the sum of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .sum();
        ///
        /// assert_eq!(lookup[&0], 3 + 9 + 12);
        /// assert_eq!(lookup[&1], 1 + 4 + 7);
        /// assert_eq!(lookup[&2], 5 + 8);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn sum(self) -> HashMap<K, V>
        where
            V: Add<V, Output = V>,
        {
            self.reduce(|acc, _, val| acc + val)
        }
        /// Groups elements from the `GroupingMap` source by key and multiply them.
        ///
        /// This is just a shorthand for `self.reduce(|acc, _, val| acc * val)`.
        /// It is more limited than `Iterator::product` since it doesn't use the `Product` trait.
        ///
        /// Returns a `HashMap` associating the key of each group with the product of that group's elements.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let lookup = vec![1, 3, 4, 5, 7, 8, 9, 12].into_iter()
        ///     .into_grouping_map_by(|&n| n % 3)
        ///     .product();
        ///
        /// assert_eq!(lookup[&0], 3 * 9 * 12);
        /// assert_eq!(lookup[&1], 1 * 4 * 7);
        /// assert_eq!(lookup[&2], 5 * 8);
        /// assert_eq!(lookup.len(), 3);
        /// ```
        pub fn product(self) -> HashMap<K, V>
        where
            V: Mul<V, Output = V>,
        {
            self.reduce(|acc, _, val| acc * val)
        }
    }
}
mod intersperse {
    use super::size_hint;
    use std::iter::{Fuse, FusedIterator};
    pub trait IntersperseElement<Item> {
        fn generate(&mut self) -> Item;
    }
    pub struct IntersperseElementSimple<Item>(Item);
    #[automatically_derived]
    impl<Item: ::core::fmt::Debug> ::core::fmt::Debug
    for IntersperseElementSimple<Item> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "IntersperseElementSimple",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl<Item: ::core::clone::Clone> ::core::clone::Clone
    for IntersperseElementSimple<Item> {
        #[inline]
        fn clone(&self) -> IntersperseElementSimple<Item> {
            IntersperseElementSimple(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<Item: Clone> IntersperseElement<Item> for IntersperseElementSimple<Item> {
        fn generate(&mut self) -> Item {
            self.0.clone()
        }
    }
    /// An iterator adaptor to insert a particular value
    /// between each element of the adapted iterator.
    ///
    /// Iterator element type is `I::Item`
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.intersperse()`](crate::Itertools::intersperse) for more information.
    pub type Intersperse<I> = IntersperseWith<
        I,
        IntersperseElementSimple<<I as Iterator>::Item>,
    >;
    /// Create a new Intersperse iterator
    pub fn intersperse<I>(iter: I, elt: I::Item) -> Intersperse<I>
    where
        I: Iterator,
    {
        intersperse_with(iter, IntersperseElementSimple(elt))
    }
    impl<Item, F: FnMut() -> Item> IntersperseElement<Item> for F {
        fn generate(&mut self) -> Item {
            self()
        }
    }
    /// An iterator adaptor to insert a particular value created by a function
    /// between each element of the adapted iterator.
    ///
    /// Iterator element type is `I::Item`
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.intersperse_with()`](crate::Itertools::intersperse_with) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct IntersperseWith<I, ElemF>
    where
        I: Iterator,
    {
        element: ElemF,
        iter: Fuse<I>,
        /// `peek` is None while no item have been taken out of `iter` (at definition).
        /// Then `peek` will alternatively be `Some(None)` and `Some(Some(item))`,
        /// where `None` indicates it's time to generate from `element` (unless `iter` is empty).
        peek: Option<Option<I::Item>>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, ElemF: ::core::clone::Clone> ::core::clone::Clone
    for IntersperseWith<I, ElemF>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> IntersperseWith<I, ElemF> {
            IntersperseWith {
                element: ::core::clone::Clone::clone(&self.element),
                iter: ::core::clone::Clone::clone(&self.iter),
                peek: ::core::clone::Clone::clone(&self.peek),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, ElemF: ::core::fmt::Debug> ::core::fmt::Debug
    for IntersperseWith<I, ElemF>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "IntersperseWith",
                "element",
                &self.element,
                "iter",
                &self.iter,
                "peek",
                &&self.peek,
            )
        }
    }
    /// Create a new `IntersperseWith` iterator
    pub fn intersperse_with<I, ElemF>(iter: I, elt: ElemF) -> IntersperseWith<I, ElemF>
    where
        I: Iterator,
    {
        IntersperseWith {
            peek: None,
            iter: iter.fuse(),
            element: elt,
        }
    }
    impl<I, ElemF> Iterator for IntersperseWith<I, ElemF>
    where
        I: Iterator,
        ElemF: IntersperseElement<I::Item>,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let Self { element, iter, peek } = self;
            match peek {
                Some(item @ Some(_)) => item.take(),
                Some(None) => {
                    match iter.next() {
                        new @ Some(_) => {
                            *peek = Some(new);
                            Some(element.generate())
                        }
                        None => None,
                    }
                }
                None => {
                    *peek = Some(None);
                    iter.next()
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let mut sh = self.iter.size_hint();
            sh = size_hint::add(sh, sh);
            match self.peek {
                Some(Some(_)) => size_hint::add_scalar(sh, 1),
                Some(None) => sh,
                None => size_hint::sub_scalar(sh, 1),
            }
        }
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let Self { mut element, mut iter, peek } = self;
            let mut accum = init;
            if let Some(x) = peek.unwrap_or_else(|| iter.next()) {
                accum = f(accum, x);
            }
            iter.fold(
                accum,
                |accum, x| {
                    let accum = f(accum, element.generate());
                    f(accum, x)
                },
            )
        }
    }
    impl<I, ElemF> FusedIterator for IntersperseWith<I, ElemF>
    where
        I: Iterator,
        ElemF: IntersperseElement<I::Item>,
    {}
}
mod iter_index {
    use core::iter::{Skip, Take};
    use core::ops::{
        Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    };
    mod private_iter_index {
        use core::ops;
        pub trait Sealed {}
        impl Sealed for ops::Range<usize> {}
        impl Sealed for ops::RangeInclusive<usize> {}
        impl Sealed for ops::RangeTo<usize> {}
        impl Sealed for ops::RangeToInclusive<usize> {}
        impl Sealed for ops::RangeFrom<usize> {}
        impl Sealed for ops::RangeFull {}
    }
    /// Used by [`Itertools::get`] to know which iterator
    /// to turn different ranges into.
    pub trait IteratorIndex<I>: private_iter_index::Sealed
    where
        I: Iterator,
    {
        /// The type returned for this type of index.
        type Output: Iterator<Item = I::Item>;
        /// Returns an adapted iterator for the current index.
        ///
        /// Prefer calling [`Itertools::get`] instead
        /// of calling this directly.
        fn index(self, from: I) -> Self::Output;
    }
    impl<I> IteratorIndex<I> for Range<usize>
    where
        I: Iterator,
    {
        type Output = Skip<Take<I>>;
        fn index(self, iter: I) -> Self::Output {
            iter.take(self.end).skip(self.start)
        }
    }
    impl<I> IteratorIndex<I> for RangeInclusive<usize>
    where
        I: Iterator,
    {
        type Output = Take<Skip<I>>;
        fn index(self, iter: I) -> Self::Output {
            let length = if *self.end() == usize::MAX {
                match (&*self.start(), &0) {
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
                self.end() - self.start() + 1
            } else {
                (self.end() + 1).saturating_sub(*self.start())
            };
            iter.skip(*self.start()).take(length)
        }
    }
    impl<I> IteratorIndex<I> for RangeTo<usize>
    where
        I: Iterator,
    {
        type Output = Take<I>;
        fn index(self, iter: I) -> Self::Output {
            iter.take(self.end)
        }
    }
    impl<I> IteratorIndex<I> for RangeToInclusive<usize>
    where
        I: Iterator,
    {
        type Output = Take<I>;
        fn index(self, iter: I) -> Self::Output {
            match (&self.end, &usize::MAX) {
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
            iter.take(self.end + 1)
        }
    }
    impl<I> IteratorIndex<I> for RangeFrom<usize>
    where
        I: Iterator,
    {
        type Output = Skip<I>;
        fn index(self, iter: I) -> Self::Output {
            iter.skip(self.start)
        }
    }
    impl<I> IteratorIndex<I> for RangeFull
    where
        I: Iterator,
    {
        type Output = I;
        fn index(self, iter: I) -> Self::Output {
            iter
        }
    }
    pub fn get<I, R>(iter: I, index: R) -> R::Output
    where
        I: IntoIterator,
        R: IteratorIndex<I::IntoIter>,
    {
        index.index(iter.into_iter())
    }
}
mod k_smallest {
    use alloc::vec::Vec;
    use core::cmp::Ordering;
    /// Consumes a given iterator, returning the minimum elements in **ascending** order.
    pub(crate) fn k_smallest_general<I, F>(
        iter: I,
        k: usize,
        mut comparator: F,
    ) -> Vec<I::Item>
    where
        I: Iterator,
        F: FnMut(&I::Item, &I::Item) -> Ordering,
    {
        /// Sift the element currently at `origin` away from the root until it is properly ordered.
        ///
        /// This will leave **larger** elements closer to the root of the heap.
        fn sift_down<T, F>(heap: &mut [T], is_less_than: &mut F, mut origin: usize)
        where
            F: FnMut(&T, &T) -> bool,
        {
            #[inline]
            fn children_of(n: usize) -> (usize, usize) {
                (2 * n + 1, 2 * n + 2)
            }
            while origin < heap.len() {
                let (left_idx, right_idx) = children_of(origin);
                if left_idx >= heap.len() {
                    return;
                }
                let replacement_idx = if right_idx < heap.len()
                    && is_less_than(&heap[left_idx], &heap[right_idx])
                {
                    right_idx
                } else {
                    left_idx
                };
                if is_less_than(&heap[origin], &heap[replacement_idx]) {
                    heap.swap(origin, replacement_idx);
                    origin = replacement_idx;
                } else {
                    return;
                }
            }
        }
        if k == 0 {
            iter.last();
            return Vec::new();
        }
        if k == 1 {
            return iter.min_by(comparator).into_iter().collect();
        }
        let mut iter = iter.fuse();
        let mut storage: Vec<I::Item> = iter.by_ref().take(k).collect();
        let mut is_less_than = move |a: &_, b: &_| comparator(a, b) == Ordering::Less;
        for i in (0..=(storage.len() / 2)).rev() {
            sift_down(&mut storage, &mut is_less_than, i);
        }
        iter.for_each(|val| {
            if true {
                match (&storage.len(), &k) {
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
            if is_less_than(&val, &storage[0]) {
                storage[0] = val;
                sift_down(&mut storage, &mut is_less_than, 0);
            }
        });
        let mut heap = &mut storage[..];
        while heap.len() > 1 {
            let last_idx = heap.len() - 1;
            heap.swap(0, last_idx);
            heap = &mut heap[..last_idx];
            sift_down(heap, &mut is_less_than, 0);
        }
        storage
    }
    pub(crate) fn k_smallest_relaxed_general<I, F>(
        iter: I,
        k: usize,
        mut comparator: F,
    ) -> Vec<I::Item>
    where
        I: Iterator,
        F: FnMut(&I::Item, &I::Item) -> Ordering,
    {
        if k == 0 {
            iter.last();
            return Vec::new();
        }
        let mut iter = iter.fuse();
        let mut buf = iter.by_ref().take(2 * k).collect::<Vec<_>>();
        if buf.len() < k {
            buf.sort_unstable_by(&mut comparator);
            return buf;
        }
        buf.select_nth_unstable_by(k - 1, &mut comparator);
        buf.truncate(k);
        iter.for_each(|val| {
            if comparator(&val, &buf[k - 1]) != Ordering::Less {
                return;
            }
            match (&buf.len(), &buf.capacity()) {
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
            buf.push(val);
            if buf.len() == 2 * k {
                buf.select_nth_unstable_by(k - 1, &mut comparator);
                buf.truncate(k);
            }
        });
        buf.sort_unstable_by(&mut comparator);
        buf.truncate(k);
        buf
    }
    #[inline]
    pub(crate) fn key_to_cmp<T, K, F>(mut key: F) -> impl FnMut(&T, &T) -> Ordering
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        move |a, b| key(a).cmp(&key(b))
    }
}
mod kmerge_impl {
    use crate::size_hint;
    use alloc::vec::Vec;
    use std::fmt;
    use std::iter::FusedIterator;
    use std::mem::replace;
    /// Head element and Tail iterator pair
    ///
    /// `PartialEq`, `Eq`, `PartialOrd` and `Ord` are implemented by comparing sequences based on
    /// first items (which are guaranteed to exist).
    ///
    /// The meanings of `PartialOrd` and `Ord` are reversed so as to turn the heap used in
    /// `KMerge` into a min-heap.
    struct HeadTail<I>
    where
        I: Iterator,
    {
        head: I::Item,
        tail: I,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for HeadTail<I>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "HeadTail",
                "head",
                &self.head,
                "tail",
                &&self.tail,
            )
        }
    }
    impl<I> HeadTail<I>
    where
        I: Iterator,
    {
        /// Constructs a `HeadTail` from an `Iterator`. Returns `None` if the `Iterator` is empty.
        fn new(mut it: I) -> Option<Self> {
            let head = it.next();
            head.map(|h| Self { head: h, tail: it })
        }
        /// Get the next element and update `head`, returning the old head in `Some`.
        ///
        /// Returns `None` when the tail is exhausted (only `head` then remains).
        fn next(&mut self) -> Option<I::Item> {
            if let Some(next) = self.tail.next() {
                Some(replace(&mut self.head, next))
            } else {
                None
            }
        }
        /// Hints at the size of the sequence, same as the `Iterator` method.
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.tail.size_hint(), 1)
        }
    }
    impl<I> Clone for HeadTail<I>
    where
        I: Iterator + Clone,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                head: self.head.clone(),
                tail: self.tail.clone(),
            }
        }
    }
    /// Make `data` a heap (min-heap w.r.t the sorting).
    fn heapify<T, S>(data: &mut [T], mut less_than: S)
    where
        S: FnMut(&T, &T) -> bool,
    {
        for i in (0..data.len() / 2).rev() {
            sift_down(data, i, &mut less_than);
        }
    }
    /// Sift down element at `index` (`heap` is a min-heap wrt the ordering)
    fn sift_down<T, S>(heap: &mut [T], index: usize, mut less_than: S)
    where
        S: FnMut(&T, &T) -> bool,
    {
        if true {
            if !(index <= heap.len()) {
                ::core::panicking::panic("assertion failed: index <= heap.len()")
            }
        }
        let mut pos = index;
        let mut child = 2 * pos + 1;
        while child + 1 < heap.len() {
            child += less_than(&heap[child + 1], &heap[child]) as usize;
            if !less_than(&heap[child], &heap[pos]) {
                return;
            }
            heap.swap(pos, child);
            pos = child;
            child = 2 * pos + 1;
        }
        if child + 1 == heap.len() && less_than(&heap[child], &heap[pos]) {
            heap.swap(pos, child);
        }
    }
    /// An iterator adaptor that merges an abitrary number of base iterators in ascending order.
    /// If all base iterators are sorted (ascending), the result is sorted.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.kmerge()`](crate::Itertools::kmerge) for more information.
    pub type KMerge<I> = KMergeBy<I, KMergeByLt>;
    pub trait KMergePredicate<T> {
        fn kmerge_pred(&mut self, a: &T, b: &T) -> bool;
    }
    pub struct KMergeByLt;
    #[automatically_derived]
    impl ::core::clone::Clone for KMergeByLt {
        #[inline]
        fn clone(&self) -> KMergeByLt {
            KMergeByLt
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for KMergeByLt {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "KMergeByLt")
        }
    }
    impl<T: PartialOrd> KMergePredicate<T> for KMergeByLt {
        fn kmerge_pred(&mut self, a: &T, b: &T) -> bool {
            a < b
        }
    }
    impl<T, F: FnMut(&T, &T) -> bool> KMergePredicate<T> for F {
        fn kmerge_pred(&mut self, a: &T, b: &T) -> bool {
            self(a, b)
        }
    }
    /// Create an iterator that merges elements of the contained iterators using
    /// the ordering function.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::kmerge`](crate::Itertools::kmerge).
    ///
    /// ```
    /// use itertools::kmerge;
    ///
    /// for elt in kmerge(vec![vec![0, 2, 4], vec![1, 3, 5], vec![6, 7]]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    pub fn kmerge<I>(iterable: I) -> KMerge<<I::Item as IntoIterator>::IntoIter>
    where
        I: IntoIterator,
        I::Item: IntoIterator,
        <<I as IntoIterator>::Item as IntoIterator>::Item: PartialOrd,
    {
        kmerge_by(iterable, KMergeByLt)
    }
    /// An iterator adaptor that merges an abitrary number of base iterators
    /// according to an ordering function.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.kmerge_by()`](crate::Itertools::kmerge_by) for more
    /// information.
    #[must_use = "this iterator adaptor is not lazy but does nearly nothing unless consumed"]
    pub struct KMergeBy<I, F>
    where
        I: Iterator,
    {
        heap: Vec<HeadTail<I>>,
        less_than: F,
    }
    impl<I, F> fmt::Debug for KMergeBy<I, F>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("KMergeBy").field("heap", &self.heap).finish()
        }
    }
    /// Create an iterator that merges elements of the contained iterators.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::kmerge_by`](crate::Itertools::kmerge_by).
    pub fn kmerge_by<I, F>(
        iterable: I,
        mut less_than: F,
    ) -> KMergeBy<<I::Item as IntoIterator>::IntoIter, F>
    where
        I: IntoIterator,
        I::Item: IntoIterator,
        F: KMergePredicate<<<I as IntoIterator>::Item as IntoIterator>::Item>,
    {
        let iter = iterable.into_iter();
        let (lower, _) = iter.size_hint();
        let mut heap: Vec<_> = Vec::with_capacity(lower);
        heap.extend(iter.filter_map(|it| HeadTail::new(it.into_iter())));
        heapify(&mut heap, |a, b| less_than.kmerge_pred(&a.head, &b.head));
        KMergeBy { heap, less_than }
    }
    impl<I, F> Clone for KMergeBy<I, F>
    where
        I: Iterator + Clone,
        I::Item: Clone,
        F: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                heap: self.heap.clone(),
                less_than: self.less_than.clone(),
            }
        }
    }
    impl<I, F> Iterator for KMergeBy<I, F>
    where
        I: Iterator,
        F: KMergePredicate<I::Item>,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            if self.heap.is_empty() {
                return None;
            }
            let result = if let Some(next) = self.heap[0].next() {
                next
            } else {
                self.heap.swap_remove(0).head
            };
            let less_than = &mut self.less_than;
            sift_down(
                &mut self.heap,
                0,
                |a, b| { less_than.kmerge_pred(&a.head, &b.head) },
            );
            Some(result)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.heap
                .iter()
                .map(|i| i.size_hint())
                .reduce(size_hint::add)
                .unwrap_or((0, Some(0)))
        }
    }
    impl<I, F> FusedIterator for KMergeBy<I, F>
    where
        I: Iterator,
        F: KMergePredicate<I::Item>,
    {}
}
mod lazy_buffer {
    use alloc::vec::Vec;
    use std::iter::Fuse;
    use std::ops::Index;
    use crate::size_hint::{self, SizeHint};
    pub struct LazyBuffer<I: Iterator> {
        it: Fuse<I>,
        buffer: Vec<I::Item>,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for LazyBuffer<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "LazyBuffer",
                "it",
                &self.it,
                "buffer",
                &&self.buffer,
            )
        }
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone for LazyBuffer<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> LazyBuffer<I> {
            LazyBuffer {
                it: ::core::clone::Clone::clone(&self.it),
                buffer: ::core::clone::Clone::clone(&self.buffer),
            }
        }
    }
    impl<I> LazyBuffer<I>
    where
        I: Iterator,
    {
        pub fn new(it: I) -> Self {
            Self {
                it: it.fuse(),
                buffer: Vec::new(),
            }
        }
        pub fn len(&self) -> usize {
            self.buffer.len()
        }
        pub fn size_hint(&self) -> SizeHint {
            size_hint::add_scalar(self.it.size_hint(), self.len())
        }
        pub fn count(self) -> usize {
            self.len() + self.it.count()
        }
        pub fn get_next(&mut self) -> bool {
            if let Some(x) = self.it.next() {
                self.buffer.push(x);
                true
            } else {
                false
            }
        }
        pub fn prefill(&mut self, len: usize) {
            let buffer_len = self.buffer.len();
            if len > buffer_len {
                let delta = len - buffer_len;
                self.buffer.extend(self.it.by_ref().take(delta));
            }
        }
    }
    impl<I> LazyBuffer<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        pub fn get_at(&self, indices: &[usize]) -> Vec<I::Item> {
            indices.iter().map(|i| self.buffer[*i].clone()).collect()
        }
        pub fn get_array<const K: usize>(&self, indices: [usize; K]) -> [I::Item; K] {
            indices.map(|i| self.buffer[i].clone())
        }
    }
    impl<I, J> Index<J> for LazyBuffer<I>
    where
        I: Iterator,
        I::Item: Sized,
        Vec<I::Item>: Index<J>,
    {
        type Output = <Vec<I::Item> as Index<J>>::Output;
        fn index(&self, index: J) -> &Self::Output {
            self.buffer.index(index)
        }
    }
}
mod merge_join {
    use std::cmp::Ordering;
    use std::fmt;
    use std::iter::{Fuse, FusedIterator};
    use std::marker::PhantomData;
    use either::Either;
    use super::adaptors::{put_back, PutBack};
    use crate::either_or_both::EitherOrBoth;
    use crate::size_hint::{self, SizeHint};
    pub struct MergeLte;
    #[automatically_derived]
    impl ::core::clone::Clone for MergeLte {
        #[inline]
        fn clone(&self) -> MergeLte {
            MergeLte
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for MergeLte {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "MergeLte")
        }
    }
    /// An iterator adaptor that merges the two base iterators in ascending order.
    /// If both base iterators are sorted (ascending), the result is sorted.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.merge()`](crate::Itertools::merge_by) for more information.
    pub type Merge<I, J> = MergeBy<I, J, MergeLte>;
    /// Create an iterator that merges elements in `i` and `j`.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::merge`](crate::Itertools::merge).
    ///
    /// ```
    /// use itertools::merge;
    ///
    /// for elt in merge(&[1, 2, 3], &[2, 3, 4]) {
    ///     /* loop body */
    ///     # let _ = elt;
    /// }
    /// ```
    pub fn merge<I, J>(
        i: I,
        j: J,
    ) -> Merge<<I as IntoIterator>::IntoIter, <J as IntoIterator>::IntoIter>
    where
        I: IntoIterator,
        J: IntoIterator<Item = I::Item>,
        I::Item: PartialOrd,
    {
        merge_by_new(i, j, MergeLte)
    }
    /// An iterator adaptor that merges the two base iterators in ascending order.
    /// If both base iterators are sorted (ascending), the result is sorted.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.merge_by()`](crate::Itertools::merge_by) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct MergeBy<I: Iterator, J: Iterator, F> {
        left: PutBack<Fuse<I>>,
        right: PutBack<Fuse<J>>,
        cmp_fn: F,
    }
    /// Create a `MergeBy` iterator.
    pub fn merge_by_new<I, J, F>(
        a: I,
        b: J,
        cmp: F,
    ) -> MergeBy<I::IntoIter, J::IntoIter, F>
    where
        I: IntoIterator,
        J: IntoIterator<Item = I::Item>,
    {
        MergeBy {
            left: put_back(a.into_iter().fuse()),
            right: put_back(b.into_iter().fuse()),
            cmp_fn: cmp,
        }
    }
    /// Return an iterator adaptor that merge-joins items from the two base iterators in ascending order.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::merge_join_by`].
    pub fn merge_join_by<I, J, F, T>(
        left: I,
        right: J,
        cmp_fn: F,
    ) -> MergeJoinBy<I::IntoIter, J::IntoIter, F>
    where
        I: IntoIterator,
        J: IntoIterator,
        F: FnMut(&I::Item, &J::Item) -> T,
    {
        MergeBy {
            left: put_back(left.into_iter().fuse()),
            right: put_back(right.into_iter().fuse()),
            cmp_fn: MergeFuncLR(cmp_fn, PhantomData),
        }
    }
    /// An iterator adaptor that merge-joins items from the two base iterators in ascending order.
    ///
    /// See [`.merge_join_by()`](crate::Itertools::merge_join_by) for more information.
    pub type MergeJoinBy<I, J, F> = MergeBy<
        I,
        J,
        MergeFuncLR<F, <F as FuncLR<<I as Iterator>::Item, <J as Iterator>::Item>>::T>,
    >;
    pub struct MergeFuncLR<F, T>(F, PhantomData<T>);
    #[automatically_derived]
    impl<F: ::core::clone::Clone, T: ::core::clone::Clone> ::core::clone::Clone
    for MergeFuncLR<F, T> {
        #[inline]
        fn clone(&self) -> MergeFuncLR<F, T> {
            MergeFuncLR(
                ::core::clone::Clone::clone(&self.0),
                ::core::clone::Clone::clone(&self.1),
            )
        }
    }
    #[automatically_derived]
    impl<F: ::core::fmt::Debug, T: ::core::fmt::Debug> ::core::fmt::Debug
    for MergeFuncLR<F, T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field2_finish(
                f,
                "MergeFuncLR",
                &self.0,
                &&self.1,
            )
        }
    }
    pub trait FuncLR<L, R> {
        type T;
    }
    impl<L, R, T, F: FnMut(&L, &R) -> T> FuncLR<L, R> for F {
        type T = T;
    }
    pub trait OrderingOrBool<L, R> {
        type MergeResult;
        fn left(left: L) -> Self::MergeResult;
        fn right(right: R) -> Self::MergeResult;
        fn merge(
            &mut self,
            left: L,
            right: R,
        ) -> (Option<Either<L, R>>, Self::MergeResult);
        fn size_hint(left: SizeHint, right: SizeHint) -> SizeHint;
    }
    impl<L, R, F: FnMut(&L, &R) -> Ordering> OrderingOrBool<L, R>
    for MergeFuncLR<F, Ordering> {
        type MergeResult = EitherOrBoth<L, R>;
        fn left(left: L) -> Self::MergeResult {
            EitherOrBoth::Left(left)
        }
        fn right(right: R) -> Self::MergeResult {
            EitherOrBoth::Right(right)
        }
        fn merge(
            &mut self,
            left: L,
            right: R,
        ) -> (Option<Either<L, R>>, Self::MergeResult) {
            match self.0(&left, &right) {
                Ordering::Equal => (None, EitherOrBoth::Both(left, right)),
                Ordering::Less => (Some(Either::Right(right)), EitherOrBoth::Left(left)),
                Ordering::Greater => {
                    (Some(Either::Left(left)), EitherOrBoth::Right(right))
                }
            }
        }
        fn size_hint(left: SizeHint, right: SizeHint) -> SizeHint {
            let (a_lower, a_upper) = left;
            let (b_lower, b_upper) = right;
            let lower = ::std::cmp::max(a_lower, b_lower);
            let upper = match (a_upper, b_upper) {
                (Some(x), Some(y)) => x.checked_add(y),
                _ => None,
            };
            (lower, upper)
        }
    }
    impl<L, R, F: FnMut(&L, &R) -> bool> OrderingOrBool<L, R> for MergeFuncLR<F, bool> {
        type MergeResult = Either<L, R>;
        fn left(left: L) -> Self::MergeResult {
            Either::Left(left)
        }
        fn right(right: R) -> Self::MergeResult {
            Either::Right(right)
        }
        fn merge(
            &mut self,
            left: L,
            right: R,
        ) -> (Option<Either<L, R>>, Self::MergeResult) {
            if self.0(&left, &right) {
                (Some(Either::Right(right)), Either::Left(left))
            } else {
                (Some(Either::Left(left)), Either::Right(right))
            }
        }
        fn size_hint(left: SizeHint, right: SizeHint) -> SizeHint {
            size_hint::add(left, right)
        }
    }
    impl<T, F: FnMut(&T, &T) -> bool> OrderingOrBool<T, T> for F {
        type MergeResult = T;
        fn left(left: T) -> Self::MergeResult {
            left
        }
        fn right(right: T) -> Self::MergeResult {
            right
        }
        fn merge(
            &mut self,
            left: T,
            right: T,
        ) -> (Option<Either<T, T>>, Self::MergeResult) {
            if self(&left, &right) {
                (Some(Either::Right(right)), left)
            } else {
                (Some(Either::Left(left)), right)
            }
        }
        fn size_hint(left: SizeHint, right: SizeHint) -> SizeHint {
            size_hint::add(left, right)
        }
    }
    impl<T: PartialOrd> OrderingOrBool<T, T> for MergeLte {
        type MergeResult = T;
        fn left(left: T) -> Self::MergeResult {
            left
        }
        fn right(right: T) -> Self::MergeResult {
            right
        }
        fn merge(
            &mut self,
            left: T,
            right: T,
        ) -> (Option<Either<T, T>>, Self::MergeResult) {
            if left <= right {
                (Some(Either::Right(right)), left)
            } else {
                (Some(Either::Left(left)), right)
            }
        }
        fn size_hint(left: SizeHint, right: SizeHint) -> SizeHint {
            size_hint::add(left, right)
        }
    }
    impl<I, J, F> Clone for MergeBy<I, J, F>
    where
        I: Iterator,
        J: Iterator,
        PutBack<Fuse<I>>: Clone,
        PutBack<Fuse<J>>: Clone,
        F: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                left: self.left.clone(),
                right: self.right.clone(),
                cmp_fn: self.cmp_fn.clone(),
            }
        }
    }
    impl<I, J, F> fmt::Debug for MergeBy<I, J, F>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug,
        J: Iterator + fmt::Debug,
        J::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("MergeBy")
                .field("left", &self.left)
                .field("right", &self.right)
                .finish()
        }
    }
    impl<I, J, F> Iterator for MergeBy<I, J, F>
    where
        I: Iterator,
        J: Iterator,
        F: OrderingOrBool<I::Item, J::Item>,
    {
        type Item = F::MergeResult;
        fn next(&mut self) -> Option<Self::Item> {
            match (self.left.next(), self.right.next()) {
                (None, None) => None,
                (Some(left), None) => Some(F::left(left)),
                (None, Some(right)) => Some(F::right(right)),
                (Some(left), Some(right)) => {
                    let (not_next, next) = self.cmp_fn.merge(left, right);
                    match not_next {
                        Some(Either::Left(l)) => {
                            self.left.put_back(l);
                        }
                        Some(Either::Right(r)) => {
                            self.right.put_back(r);
                        }
                        None => {}
                    }
                    Some(next)
                }
            }
        }
        fn fold<B, G>(mut self, init: B, mut f: G) -> B
        where
            Self: Sized,
            G: FnMut(B, Self::Item) -> B,
        {
            let mut acc = init;
            let mut left = self.left.next();
            let mut right = self.right.next();
            loop {
                match (left, right) {
                    (Some(l), Some(r)) => {
                        match self.cmp_fn.merge(l, r) {
                            (Some(Either::Right(r)), x) => {
                                acc = f(acc, x);
                                left = self.left.next();
                                right = Some(r);
                            }
                            (Some(Either::Left(l)), x) => {
                                acc = f(acc, x);
                                left = Some(l);
                                right = self.right.next();
                            }
                            (None, x) => {
                                acc = f(acc, x);
                                left = self.left.next();
                                right = self.right.next();
                            }
                        }
                    }
                    (Some(l), None) => {
                        self.left.put_back(l);
                        acc = self.left.fold(acc, |acc, x| f(acc, F::left(x)));
                        break;
                    }
                    (None, Some(r)) => {
                        self.right.put_back(r);
                        acc = self.right.fold(acc, |acc, x| f(acc, F::right(x)));
                        break;
                    }
                    (None, None) => {
                        break;
                    }
                }
            }
            acc
        }
        fn size_hint(&self) -> SizeHint {
            F::size_hint(self.left.size_hint(), self.right.size_hint())
        }
        fn nth(&mut self, mut n: usize) -> Option<Self::Item> {
            loop {
                if n == 0 {
                    break self.next();
                }
                n -= 1;
                match (self.left.next(), self.right.next()) {
                    (None, None) => break None,
                    (Some(_left), None) => break self.left.nth(n).map(F::left),
                    (None, Some(_right)) => break self.right.nth(n).map(F::right),
                    (Some(left), Some(right)) => {
                        let (not_next, _) = self.cmp_fn.merge(left, right);
                        match not_next {
                            Some(Either::Left(l)) => {
                                self.left.put_back(l);
                            }
                            Some(Either::Right(r)) => {
                                self.right.put_back(r);
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    }
    impl<I, J, F> FusedIterator for MergeBy<I, J, F>
    where
        I: Iterator,
        J: Iterator,
        F: OrderingOrBool<I::Item, J::Item>,
    {}
}
mod minmax {
    /// `MinMaxResult` is an enum returned by `minmax`.
    ///
    /// See [`.minmax()`](crate::Itertools::minmax) for more detail.
    pub enum MinMaxResult<T> {
        /// Empty iterator
        NoElements,
        /// Iterator with one element, so the minimum and maximum are the same
        OneElement(T),
        /// More than one element in the iterator, the first element is not larger
        /// than the second
        MinMax(T, T),
    }
    #[automatically_derived]
    impl<T: ::core::marker::Copy> ::core::marker::Copy for MinMaxResult<T> {}
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for MinMaxResult<T> {
        #[inline]
        fn clone(&self) -> MinMaxResult<T> {
            match self {
                MinMaxResult::NoElements => MinMaxResult::NoElements,
                MinMaxResult::OneElement(__self_0) => {
                    MinMaxResult::OneElement(::core::clone::Clone::clone(__self_0))
                }
                MinMaxResult::MinMax(__self_0, __self_1) => {
                    MinMaxResult::MinMax(
                        ::core::clone::Clone::clone(__self_0),
                        ::core::clone::Clone::clone(__self_1),
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl<T> ::core::marker::StructuralPartialEq for MinMaxResult<T> {}
    #[automatically_derived]
    impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for MinMaxResult<T> {
        #[inline]
        fn eq(&self, other: &MinMaxResult<T>) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        MinMaxResult::OneElement(__self_0),
                        MinMaxResult::OneElement(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    (
                        MinMaxResult::MinMax(__self_0, __self_1),
                        MinMaxResult::MinMax(__arg1_0, __arg1_1),
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    _ => true,
                }
        }
    }
    #[automatically_derived]
    impl<T: ::core::cmp::Eq> ::core::cmp::Eq for MinMaxResult<T> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<T>;
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for MinMaxResult<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                MinMaxResult::NoElements => {
                    ::core::fmt::Formatter::write_str(f, "NoElements")
                }
                MinMaxResult::OneElement(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "OneElement",
                        &__self_0,
                    )
                }
                MinMaxResult::MinMax(__self_0, __self_1) => {
                    ::core::fmt::Formatter::debug_tuple_field2_finish(
                        f,
                        "MinMax",
                        __self_0,
                        &__self_1,
                    )
                }
            }
        }
    }
    impl<T: Clone> MinMaxResult<T> {
        /// `into_option` creates an `Option` of type `(T, T)`. The returned `Option`
        /// has variant `None` if and only if the `MinMaxResult` has variant
        /// `NoElements`. Otherwise `Some((x, y))` is returned where `x <= y`.
        /// If the `MinMaxResult` has variant `OneElement(x)`, performing this
        /// operation will make one clone of `x`.
        ///
        /// # Examples
        ///
        /// ```
        /// use itertools::MinMaxResult::{self, NoElements, OneElement, MinMax};
        ///
        /// let r: MinMaxResult<i32> = NoElements;
        /// assert_eq!(r.into_option(), None);
        ///
        /// let r = OneElement(1);
        /// assert_eq!(r.into_option(), Some((1, 1)));
        ///
        /// let r = MinMax(1, 2);
        /// assert_eq!(r.into_option(), Some((1, 2)));
        /// ```
        pub fn into_option(self) -> Option<(T, T)> {
            match self {
                Self::NoElements => None,
                Self::OneElement(x) => Some((x.clone(), x)),
                Self::MinMax(x, y) => Some((x, y)),
            }
        }
    }
    /// Implementation guts for `minmax` and `minmax_by_key`.
    pub fn minmax_impl<I, K, F, L>(
        mut it: I,
        mut key_for: F,
        mut lt: L,
    ) -> MinMaxResult<I::Item>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> K,
        L: FnMut(&I::Item, &I::Item, &K, &K) -> bool,
    {
        let (mut min, mut max, mut min_key, mut max_key) = match it.next() {
            None => return MinMaxResult::NoElements,
            Some(x) => {
                match it.next() {
                    None => return MinMaxResult::OneElement(x),
                    Some(y) => {
                        let xk = key_for(&x);
                        let yk = key_for(&y);
                        if !lt(&y, &x, &yk, &xk) {
                            (x, y, xk, yk)
                        } else {
                            (y, x, yk, xk)
                        }
                    }
                }
            }
        };
        loop {
            let first = match it.next() {
                None => break,
                Some(x) => x,
            };
            let second = match it.next() {
                None => {
                    let first_key = key_for(&first);
                    if lt(&first, &min, &first_key, &min_key) {
                        min = first;
                    } else if !lt(&first, &max, &first_key, &max_key) {
                        max = first;
                    }
                    break;
                }
                Some(x) => x,
            };
            let first_key = key_for(&first);
            let second_key = key_for(&second);
            if !lt(&second, &first, &second_key, &first_key) {
                if lt(&first, &min, &first_key, &min_key) {
                    min = first;
                    min_key = first_key;
                }
                if !lt(&second, &max, &second_key, &max_key) {
                    max = second;
                    max_key = second_key;
                }
            } else {
                if lt(&second, &min, &second_key, &min_key) {
                    min = second;
                    min_key = second_key;
                }
                if !lt(&first, &max, &first_key, &max_key) {
                    max = first;
                    max_key = first_key;
                }
            }
        }
        MinMaxResult::MinMax(min, max)
    }
}
mod multipeek_impl {
    use crate::size_hint;
    use crate::PeekingNext;
    use alloc::collections::VecDeque;
    use std::iter::Fuse;
    /// See [`multipeek()`] for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct MultiPeek<I>
    where
        I: Iterator,
    {
        iter: Fuse<I>,
        buf: VecDeque<I::Item>,
        index: usize,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for MultiPeek<I>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> MultiPeek<I> {
            MultiPeek {
                iter: ::core::clone::Clone::clone(&self.iter),
                buf: ::core::clone::Clone::clone(&self.buf),
                index: ::core::clone::Clone::clone(&self.index),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for MultiPeek<I>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "MultiPeek",
                "iter",
                &self.iter,
                "buf",
                &self.buf,
                "index",
                &&self.index,
            )
        }
    }
    /// An iterator adaptor that allows the user to peek at multiple `.next()`
    /// values without advancing the base iterator.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::multipeek`].
    pub fn multipeek<I>(iterable: I) -> MultiPeek<I::IntoIter>
    where
        I: IntoIterator,
    {
        MultiPeek {
            iter: iterable.into_iter().fuse(),
            buf: VecDeque::new(),
            index: 0,
        }
    }
    impl<I> MultiPeek<I>
    where
        I: Iterator,
    {
        /// Reset the peeking “cursor”
        pub fn reset_peek(&mut self) {
            self.index = 0;
        }
    }
    impl<I: Iterator> MultiPeek<I> {
        /// Works exactly like `.next()` with the only difference that it doesn't
        /// advance itself. `.peek()` can be called multiple times, to peek
        /// further ahead.
        /// When `.next()` is called, reset the peeking “cursor”.
        pub fn peek(&mut self) -> Option<&I::Item> {
            let ret = if self.index < self.buf.len() {
                Some(&self.buf[self.index])
            } else {
                match self.iter.next() {
                    Some(x) => {
                        self.buf.push_back(x);
                        Some(&self.buf[self.index])
                    }
                    None => return None,
                }
            };
            self.index += 1;
            ret
        }
    }
    impl<I> PeekingNext for MultiPeek<I>
    where
        I: Iterator,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            if self.buf.is_empty() {
                if let Some(r) = self.peek() {
                    if !accept(r) {
                        return None;
                    }
                }
            } else if let Some(r) = self.buf.front() {
                if !accept(r) {
                    return None;
                }
            }
            self.next()
        }
    }
    impl<I> Iterator for MultiPeek<I>
    where
        I: Iterator,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.index = 0;
            self.buf.pop_front().or_else(|| self.iter.next())
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.iter.size_hint(), self.buf.len())
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            init = self.buf.into_iter().fold(init, &mut f);
            self.iter.fold(init, f)
        }
    }
    impl<I> ExactSizeIterator for MultiPeek<I>
    where
        I: ExactSizeIterator,
    {}
}
mod next_array {
    use core::mem::{self, MaybeUninit};
    /// An array of at most `N` elements.
    struct ArrayBuilder<T, const N: usize> {
        /// The (possibly uninitialized) elements of the `ArrayBuilder`.
        ///
        /// # Safety
        ///
        /// The elements of `arr[..len]` are valid `T`s.
        arr: [MaybeUninit<T>; N],
        /// The number of leading elements of `arr` that are valid `T`s, len <= N.
        len: usize,
    }
    impl<T, const N: usize> ArrayBuilder<T, N> {
        /// Initializes a new, empty `ArrayBuilder`.
        pub fn new() -> Self {
            Self {
                arr: [(); N].map(|_| MaybeUninit::uninit()),
                len: 0,
            }
        }
        /// Pushes `value` onto the end of the array.
        ///
        /// # Panics
        ///
        /// This panics if `self.len >= N`.
        #[inline(always)]
        pub fn push(&mut self, value: T) {
            let place = &mut self.arr[self.len];
            *place = MaybeUninit::new(value);
            self.len += 1;
        }
        /// Consumes the elements in the `ArrayBuilder` and returns them as an array
        /// `[T; N]`.
        ///
        /// If `self.len() < N`, this returns `None`.
        pub fn take(&mut self) -> Option<[T; N]> {
            if self.len == N {
                self.len = 0;
                let arr = mem::replace(
                    &mut self.arr,
                    [(); N].map(|_| MaybeUninit::uninit()),
                );
                Some(arr.map(|v| { unsafe { v.assume_init() } }))
            } else {
                None
            }
        }
    }
    impl<T, const N: usize> AsMut<[T]> for ArrayBuilder<T, N> {
        fn as_mut(&mut self) -> &mut [T] {
            let valid = &mut self.arr[..self.len];
            unsafe { slice_assume_init_mut(valid) }
        }
    }
    impl<T, const N: usize> Drop for ArrayBuilder<T, N> {
        fn drop(&mut self) {
            unsafe { core::ptr::drop_in_place(self.as_mut()) }
        }
    }
    /// Assuming all the elements are initialized, get a mutable slice to them.
    ///
    /// # Safety
    ///
    /// The caller guarantees that the elements `T` referenced by `slice` are in a
    /// valid state.
    unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
        unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
    }
    /// Equivalent to `it.next_array()`.
    pub(crate) fn next_array<I, const N: usize>(it: &mut I) -> Option<[I::Item; N]>
    where
        I: Iterator,
    {
        let mut builder = ArrayBuilder::new();
        for _ in 0..N {
            builder.push(it.next()?);
        }
        builder.take()
    }
    mod test {
        use super::ArrayBuilder;
        extern crate test;
        #[rustc_test_marker = "next_array::test::zero_len_take"]
        #[doc(hidden)]
        pub const zero_len_take: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("next_array::test::zero_len_take"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/next_array.rs",
                start_line: 153usize,
                start_col: 8usize,
                end_line: 153usize,
                end_col: 21usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(zero_len_take()),
            ),
        };
        fn zero_len_take() {
            let mut builder = ArrayBuilder::<(), 0>::new();
            let taken = builder.take();
            match (&taken, &Some([(); 0])) {
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
        #[rustc_test_marker = "next_array::test::zero_len_push"]
        #[doc(hidden)]
        pub const zero_len_push: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("next_array::test::zero_len_push"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/next_array.rs",
                start_line: 161usize,
                start_col: 8usize,
                end_line: 161usize,
                end_col: 21usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::Yes,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(zero_len_push()),
            ),
        };
        #[should_panic]
        fn zero_len_push() {
            let mut builder = ArrayBuilder::<(), 0>::new();
            builder.push(());
        }
        extern crate test;
        #[rustc_test_marker = "next_array::test::push_4"]
        #[doc(hidden)]
        pub const push_4: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("next_array::test::push_4"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/next_array.rs",
                start_line: 167usize,
                start_col: 8usize,
                end_line: 167usize,
                end_col: 14usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(push_4()),
            ),
        };
        fn push_4() {
            let mut builder = ArrayBuilder::<(), 4>::new();
            match (&builder.take(), &None) {
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
            builder.push(());
            match (&builder.take(), &None) {
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
            builder.push(());
            match (&builder.take(), &None) {
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
            builder.push(());
            match (&builder.take(), &None) {
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
            builder.push(());
            match (&builder.take(), &Some([(); 4])) {
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
        #[rustc_test_marker = "next_array::test::tracked_drop"]
        #[doc(hidden)]
        pub const tracked_drop: test::TestDescAndFn = test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::StaticTestName("next_array::test::tracked_drop"),
                ignore: false,
                ignore_message: ::core::option::Option::None,
                source_file: "src/next_array.rs",
                start_line: 185usize,
                start_col: 8usize,
                end_line: 185usize,
                end_col: 20usize,
                compile_fail: false,
                no_run: false,
                should_panic: test::ShouldPanic::No,
                test_type: test::TestType::UnitTest,
            },
            testfn: test::StaticTestFn(
                #[coverage(off)]
                || test::assert_test_result(tracked_drop()),
            ),
        };
        fn tracked_drop() {
            use std::panic::{catch_unwind, AssertUnwindSafe};
            use std::sync::atomic::{AtomicU16, Ordering};
            static DROPPED: AtomicU16 = AtomicU16::new(0);
            struct TrackedDrop;
            #[automatically_derived]
            impl ::core::fmt::Debug for TrackedDrop {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(f, "TrackedDrop")
                }
            }
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for TrackedDrop {}
            #[automatically_derived]
            impl ::core::cmp::PartialEq for TrackedDrop {
                #[inline]
                fn eq(&self, other: &TrackedDrop) -> bool {
                    true
                }
            }
            impl Drop for TrackedDrop {
                fn drop(&mut self) {
                    DROPPED.fetch_add(1, Ordering::Relaxed);
                }
            }
            {
                let builder = ArrayBuilder::<TrackedDrop, 0>::new();
                match (&DROPPED.load(Ordering::Relaxed), &0) {
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
                drop(builder);
                match (&DROPPED.load(Ordering::Relaxed), &0) {
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
            {
                let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
                builder.push(TrackedDrop);
                match (&builder.take(), &None) {
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
                match (&DROPPED.load(Ordering::Relaxed), &0) {
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
                drop(builder);
                match (&DROPPED.swap(0, Ordering::Relaxed), &1) {
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
            {
                let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
                builder.push(TrackedDrop);
                builder.push(TrackedDrop);
                if !#[allow(non_exhaustive_omitted_patterns)]
                match builder.take() {
                    Some(_) => true,
                    _ => false,
                } {
                    ::core::panicking::panic(
                        "assertion failed: matches!(builder.take(), Some(_))",
                    )
                }
                match (&DROPPED.swap(0, Ordering::Relaxed), &2) {
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
                drop(builder);
                match (&DROPPED.load(Ordering::Relaxed), &0) {
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
            {
                let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
                builder.push(TrackedDrop);
                builder.push(TrackedDrop);
                if !catch_unwind(
                        AssertUnwindSafe(|| {
                            builder.push(TrackedDrop);
                        }),
                    )
                    .is_err()
                {
                    ::core::panicking::panic(
                        "assertion failed: catch_unwind(AssertUnwindSafe(|| { builder.push(TrackedDrop); })).is_err()",
                    )
                }
                match (&DROPPED.load(Ordering::Relaxed), &1) {
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
                drop(builder);
                match (&DROPPED.swap(0, Ordering::Relaxed), &3) {
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
            {
                let mut builder = ArrayBuilder::<TrackedDrop, 2>::new();
                builder.push(TrackedDrop);
                builder.push(TrackedDrop);
                if !catch_unwind(
                        AssertUnwindSafe(|| {
                            builder.push(TrackedDrop);
                        }),
                    )
                    .is_err()
                {
                    ::core::panicking::panic(
                        "assertion failed: catch_unwind(AssertUnwindSafe(|| { builder.push(TrackedDrop); })).is_err()",
                    )
                }
                match (&DROPPED.load(Ordering::Relaxed), &1) {
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
                if !#[allow(non_exhaustive_omitted_patterns)]
                match builder.take() {
                    Some(_) => true,
                    _ => false,
                } {
                    ::core::panicking::panic(
                        "assertion failed: matches!(builder.take(), Some(_))",
                    )
                }
                match (&DROPPED.load(Ordering::Relaxed), &3) {
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
                builder.push(TrackedDrop);
                builder.push(TrackedDrop);
                if !#[allow(non_exhaustive_omitted_patterns)]
                match builder.take() {
                    Some(_) => true,
                    _ => false,
                } {
                    ::core::panicking::panic(
                        "assertion failed: matches!(builder.take(), Some(_))",
                    )
                }
                match (&DROPPED.swap(0, Ordering::Relaxed), &5) {
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
}
mod pad_tail {
    use crate::size_hint;
    use std::iter::{Fuse, FusedIterator};
    /// An iterator adaptor that pads a sequence to a minimum length by filling
    /// missing elements using a closure.
    ///
    /// Iterator element type is `I::Item`.
    ///
    /// See [`.pad_using()`](crate::Itertools::pad_using) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct PadUsing<I, F> {
        iter: Fuse<I>,
        min: usize,
        pos: usize,
        filler: F,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for PadUsing<I, F> {
        #[inline]
        fn clone(&self) -> PadUsing<I, F> {
            PadUsing {
                iter: ::core::clone::Clone::clone(&self.iter),
                min: ::core::clone::Clone::clone(&self.min),
                pos: ::core::clone::Clone::clone(&self.pos),
                filler: ::core::clone::Clone::clone(&self.filler),
            }
        }
    }
    impl<I, F> std::fmt::Debug for PadUsing<I, F>
    where
        I: std::fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("PadUsing")
                .field("iter", &self.iter)
                .field("min", &self.min)
                .field("pos", &self.pos)
                .finish()
        }
    }
    /// Create a new `PadUsing` iterator.
    pub fn pad_using<I, F>(iter: I, min: usize, filler: F) -> PadUsing<I, F>
    where
        I: Iterator,
        F: FnMut(usize) -> I::Item,
    {
        PadUsing {
            iter: iter.fuse(),
            min,
            pos: 0,
            filler,
        }
    }
    impl<I, F> Iterator for PadUsing<I, F>
    where
        I: Iterator,
        F: FnMut(usize) -> I::Item,
    {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match self.iter.next() {
                None => {
                    if self.pos < self.min {
                        let e = Some((self.filler)(self.pos));
                        self.pos += 1;
                        e
                    } else {
                        None
                    }
                }
                e => {
                    self.pos += 1;
                    e
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let tail = self.min.saturating_sub(self.pos);
            size_hint::max(self.iter.size_hint(), (tail, Some(tail)))
        }
        fn fold<B, G>(self, mut init: B, mut f: G) -> B
        where
            G: FnMut(B, Self::Item) -> B,
        {
            let mut pos = self.pos;
            init = self
                .iter
                .fold(
                    init,
                    |acc, item| {
                        pos += 1;
                        f(acc, item)
                    },
                );
            (pos..self.min).map(self.filler).fold(init, f)
        }
    }
    impl<I, F> DoubleEndedIterator for PadUsing<I, F>
    where
        I: DoubleEndedIterator + ExactSizeIterator,
        F: FnMut(usize) -> I::Item,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.min == 0 {
                self.iter.next_back()
            } else if self.iter.len() >= self.min {
                self.min -= 1;
                self.iter.next_back()
            } else {
                self.min -= 1;
                Some((self.filler)(self.min))
            }
        }
        fn rfold<B, G>(self, mut init: B, mut f: G) -> B
        where
            G: FnMut(B, Self::Item) -> B,
        {
            init = (self.iter.len()..self.min).map(self.filler).rfold(init, &mut f);
            self.iter.rfold(init, f)
        }
    }
    impl<I, F> ExactSizeIterator for PadUsing<I, F>
    where
        I: ExactSizeIterator,
        F: FnMut(usize) -> I::Item,
    {}
    impl<I, F> FusedIterator for PadUsing<I, F>
    where
        I: FusedIterator,
        F: FnMut(usize) -> I::Item,
    {}
}
mod peek_nth {
    use crate::size_hint;
    use crate::PeekingNext;
    use alloc::collections::VecDeque;
    use std::iter::Fuse;
    /// See [`peek_nth()`] for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct PeekNth<I>
    where
        I: Iterator,
    {
        iter: Fuse<I>,
        buf: VecDeque<I::Item>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for PeekNth<I>
    where
        I: Iterator,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> PeekNth<I> {
            PeekNth {
                iter: ::core::clone::Clone::clone(&self.iter),
                buf: ::core::clone::Clone::clone(&self.buf),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for PeekNth<I>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PeekNth",
                "iter",
                &self.iter,
                "buf",
                &&self.buf,
            )
        }
    }
    /// A drop-in replacement for [`std::iter::Peekable`] which adds a `peek_nth`
    /// method allowing the user to `peek` at a value several iterations forward
    /// without advancing the base iterator.
    ///
    /// This differs from `multipeek` in that subsequent calls to `peek` or
    /// `peek_nth` will always return the same value until `next` is called
    /// (making `reset_peek` unnecessary).
    pub fn peek_nth<I>(iterable: I) -> PeekNth<I::IntoIter>
    where
        I: IntoIterator,
    {
        PeekNth {
            iter: iterable.into_iter().fuse(),
            buf: VecDeque::new(),
        }
    }
    impl<I> PeekNth<I>
    where
        I: Iterator,
    {
        /// Works exactly like the `peek` method in [`std::iter::Peekable`].
        pub fn peek(&mut self) -> Option<&I::Item> {
            self.peek_nth(0)
        }
        /// Works exactly like the `peek_mut` method in [`std::iter::Peekable`].
        pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
            self.peek_nth_mut(0)
        }
        /// Returns a reference to the `nth` value without advancing the iterator.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use itertools::peek_nth;
        ///
        /// let xs = vec![1, 2, 3];
        /// let mut iter = peek_nth(xs.into_iter());
        ///
        /// assert_eq!(iter.peek_nth(0), Some(&1));
        /// assert_eq!(iter.next(), Some(1));
        ///
        /// // The iterator does not advance even if we call `peek_nth` multiple times
        /// assert_eq!(iter.peek_nth(0), Some(&2));
        /// assert_eq!(iter.peek_nth(1), Some(&3));
        /// assert_eq!(iter.next(), Some(2));
        ///
        /// // Calling `peek_nth` past the end of the iterator will return `None`
        /// assert_eq!(iter.peek_nth(1), None);
        /// ```
        pub fn peek_nth(&mut self, n: usize) -> Option<&I::Item> {
            let unbuffered_items = (n + 1).saturating_sub(self.buf.len());
            self.buf.extend(self.iter.by_ref().take(unbuffered_items));
            self.buf.get(n)
        }
        /// Returns a mutable reference to the `nth` value without advancing the iterator.
        ///
        /// # Examples
        ///
        /// Basic usage:
        ///
        /// ```
        /// use itertools::peek_nth;
        ///
        /// let xs = vec![1, 2, 3, 4, 5];
        /// let mut iter = peek_nth(xs.into_iter());
        ///
        /// assert_eq!(iter.peek_nth_mut(0), Some(&mut 1));
        /// assert_eq!(iter.next(), Some(1));
        ///
        /// // The iterator does not advance even if we call `peek_nth_mut` multiple times
        /// assert_eq!(iter.peek_nth_mut(0), Some(&mut 2));
        /// assert_eq!(iter.peek_nth_mut(1), Some(&mut 3));
        /// assert_eq!(iter.next(), Some(2));
        ///
        /// // Peek into the iterator and set the value behind the mutable reference.
        /// if let Some(p) = iter.peek_nth_mut(1) {
        ///     assert_eq!(*p, 4);
        ///     *p = 9;
        /// }
        ///
        /// // The value we put in reappears as the iterator continues.
        /// assert_eq!(iter.next(), Some(3));
        /// assert_eq!(iter.next(), Some(9));
        ///
        /// // Calling `peek_nth_mut` past the end of the iterator will return `None`
        /// assert_eq!(iter.peek_nth_mut(1), None);
        /// ```
        pub fn peek_nth_mut(&mut self, n: usize) -> Option<&mut I::Item> {
            let unbuffered_items = (n + 1).saturating_sub(self.buf.len());
            self.buf.extend(self.iter.by_ref().take(unbuffered_items));
            self.buf.get_mut(n)
        }
        /// Works exactly like the `next_if` method in [`std::iter::Peekable`].
        pub fn next_if(
            &mut self,
            func: impl FnOnce(&I::Item) -> bool,
        ) -> Option<I::Item> {
            match self.next() {
                Some(item) if func(&item) => Some(item),
                Some(item) => {
                    self.buf.push_front(item);
                    None
                }
                _ => None,
            }
        }
        /// Works exactly like the `next_if_eq` method in [`std::iter::Peekable`].
        pub fn next_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
        where
            T: ?Sized,
            I::Item: PartialEq<T>,
        {
            self.next_if(|next| next == expected)
        }
    }
    impl<I> Iterator for PeekNth<I>
    where
        I: Iterator,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.buf.pop_front().or_else(|| self.iter.next())
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.iter.size_hint(), self.buf.len())
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            init = self.buf.into_iter().fold(init, &mut f);
            self.iter.fold(init, f)
        }
    }
    impl<I> ExactSizeIterator for PeekNth<I>
    where
        I: ExactSizeIterator,
    {}
    impl<I> PeekingNext for PeekNth<I>
    where
        I: Iterator,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            self.peek().filter(|item| accept(item))?;
            self.next()
        }
    }
}
mod peeking_take_while {
    use crate::PutBack;
    use crate::PutBackN;
    use crate::RepeatN;
    use std::iter::Peekable;
    /// An iterator that allows peeking at an element before deciding to accept it.
    ///
    /// See [`.peeking_take_while()`](crate::Itertools::peeking_take_while)
    /// for more information.
    ///
    /// This is implemented by peeking adaptors like peekable and put back,
    /// but also by a few iterators that can be peeked natively, like the slice’s
    /// by reference iterator ([`std::slice::Iter`]).
    pub trait PeekingNext: Iterator {
        /// Pass a reference to the next iterator element to the closure `accept`;
        /// if `accept` returns `true`, return it as the next element,
        /// else `None`.
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            Self: Sized,
            F: FnOnce(&Self::Item) -> bool;
    }
    impl<I> PeekingNext for &mut I
    where
        I: PeekingNext,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            (*self).peeking_next(accept)
        }
    }
    impl<I> PeekingNext for Peekable<I>
    where
        I: Iterator,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            if let Some(r) = self.peek() {
                if !accept(r) {
                    return None;
                }
            }
            self.next()
        }
    }
    impl<I> PeekingNext for PutBack<I>
    where
        I: Iterator,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            if let Some(r) = self.next() {
                if !accept(&r) {
                    self.put_back(r);
                    return None;
                }
                Some(r)
            } else {
                None
            }
        }
    }
    impl<I> PeekingNext for PutBackN<I>
    where
        I: Iterator,
    {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            if let Some(r) = self.next() {
                if !accept(&r) {
                    self.put_back(r);
                    return None;
                }
                Some(r)
            } else {
                None
            }
        }
    }
    impl<T: Clone> PeekingNext for RepeatN<T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let r = self.elt.as_ref()?;
            if !accept(r) {
                return None;
            }
            self.next()
        }
    }
    /// An iterator adaptor that takes items while a closure returns `true`.
    ///
    /// See [`.peeking_take_while()`](crate::Itertools::peeking_take_while)
    /// for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct PeekingTakeWhile<'a, I, F>
    where
        I: Iterator + 'a,
    {
        iter: &'a mut I,
        f: F,
    }
    impl<'a, I, F> std::fmt::Debug for PeekingTakeWhile<'a, I, F>
    where
        I: Iterator + std::fmt::Debug + 'a,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("PeekingTakeWhile").field("iter", &self.iter).finish()
        }
    }
    /// Create a `PeekingTakeWhile`
    pub fn peeking_take_while<I, F>(iter: &mut I, f: F) -> PeekingTakeWhile<I, F>
    where
        I: Iterator,
    {
        PeekingTakeWhile { iter, f }
    }
    impl<I, F> Iterator for PeekingTakeWhile<'_, I, F>
    where
        I: PeekingNext,
        F: FnMut(&I::Item) -> bool,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.peeking_next(&mut self.f)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
    }
    impl<I, F> PeekingNext for PeekingTakeWhile<'_, I, F>
    where
        I: PeekingNext,
        F: FnMut(&I::Item) -> bool,
    {
        fn peeking_next<G>(&mut self, g: G) -> Option<Self::Item>
        where
            G: FnOnce(&Self::Item) -> bool,
        {
            let f = &mut self.f;
            self.iter.peeking_next(|r| f(r) && g(r))
        }
    }
    impl<'a, T> PeekingNext for ::std::slice::Iter<'a, T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a> PeekingNext for ::std::str::Chars<'a> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a> PeekingNext for ::std::str::CharIndices<'a> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a> PeekingNext for ::std::str::Bytes<'a> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a, T> PeekingNext for ::std::option::Iter<'a, T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a, T> PeekingNext for ::std::result::Iter<'a, T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<T> PeekingNext for ::std::iter::Empty<T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a, T> PeekingNext for alloc::collections::linked_list::Iter<'a, T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<'a, T> PeekingNext for alloc::collections::vec_deque::Iter<'a, T> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
    impl<I: Clone + PeekingNext + DoubleEndedIterator> PeekingNext
    for ::std::iter::Rev<I> {
        fn peeking_next<F>(&mut self, accept: F) -> Option<Self::Item>
        where
            F: FnOnce(&Self::Item) -> bool,
        {
            let saved_state = self.clone();
            if let Some(r) = self.next() {
                if !accept(&r) {
                    *self = saved_state;
                } else {
                    return Some(r)
                }
            }
            None
        }
    }
}
mod permutations {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use std::fmt;
    use std::iter::once;
    use std::iter::FusedIterator;
    use super::lazy_buffer::LazyBuffer;
    use crate::size_hint::{self, SizeHint};
    /// An iterator adaptor that iterates through all the `k`-permutations of the
    /// elements from an iterator.
    ///
    /// See [`.permutations()`](crate::Itertools::permutations) for
    /// more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Permutations<I: Iterator> {
        vals: LazyBuffer<I>,
        state: PermutationState,
    }
    impl<I> Clone for Permutations<I>
    where
        I: Clone + Iterator,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                vals: self.vals.clone(),
                state: self.state.clone(),
            }
        }
    }
    enum PermutationState {
        /// No permutation generated yet.
        Start { k: usize },
        /// Values from the iterator are not fully loaded yet so `n` is still unknown.
        Buffered { k: usize, min_n: usize },
        /// All values from the iterator are known so `n` is known.
        Loaded { indices: Box<[usize]>, cycles: Box<[usize]> },
        /// No permutation left to generate.
        End,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PermutationState {
        #[inline]
        fn clone(&self) -> PermutationState {
            match self {
                PermutationState::Start { k: __self_0 } => {
                    PermutationState::Start {
                        k: ::core::clone::Clone::clone(__self_0),
                    }
                }
                PermutationState::Buffered { k: __self_0, min_n: __self_1 } => {
                    PermutationState::Buffered {
                        k: ::core::clone::Clone::clone(__self_0),
                        min_n: ::core::clone::Clone::clone(__self_1),
                    }
                }
                PermutationState::Loaded { indices: __self_0, cycles: __self_1 } => {
                    PermutationState::Loaded {
                        indices: ::core::clone::Clone::clone(__self_0),
                        cycles: ::core::clone::Clone::clone(__self_1),
                    }
                }
                PermutationState::End => PermutationState::End,
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PermutationState {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                PermutationState::Start { k: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Start",
                        "k",
                        &__self_0,
                    )
                }
                PermutationState::Buffered { k: __self_0, min_n: __self_1 } => {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Buffered",
                        "k",
                        __self_0,
                        "min_n",
                        &__self_1,
                    )
                }
                PermutationState::Loaded { indices: __self_0, cycles: __self_1 } => {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Loaded",
                        "indices",
                        __self_0,
                        "cycles",
                        &__self_1,
                    )
                }
                PermutationState::End => ::core::fmt::Formatter::write_str(f, "End"),
            }
        }
    }
    impl<I> fmt::Debug for Permutations<I>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Permutations")
                .field("vals", &self.vals)
                .field("state", &self.state)
                .finish()
        }
    }
    pub fn permutations<I: Iterator>(iter: I, k: usize) -> Permutations<I> {
        Permutations {
            vals: LazyBuffer::new(iter),
            state: PermutationState::Start { k },
        }
    }
    impl<I> Iterator for Permutations<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        type Item = Vec<I::Item>;
        fn next(&mut self) -> Option<Self::Item> {
            let Self { vals, state } = self;
            match state {
                PermutationState::Start { k: 0 } => {
                    *state = PermutationState::End;
                    Some(Vec::new())
                }
                &mut PermutationState::Start { k } => {
                    vals.prefill(k);
                    if vals.len() != k {
                        *state = PermutationState::End;
                        return None;
                    }
                    *state = PermutationState::Buffered {
                        k,
                        min_n: k,
                    };
                    Some(vals[0..k].to_vec())
                }
                PermutationState::Buffered { ref k, min_n } => {
                    if vals.get_next() {
                        let item = (0..*k - 1)
                            .chain(once(*min_n))
                            .map(|i| vals[i].clone())
                            .collect();
                        *min_n += 1;
                        Some(item)
                    } else {
                        let n = *min_n;
                        let prev_iteration_count = n - *k + 1;
                        let mut indices: Box<[_]> = (0..n).collect();
                        let mut cycles: Box<[_]> = (n - k..n).rev().collect();
                        for _ in 0..prev_iteration_count {
                            if advance(&mut indices, &mut cycles) {
                                *state = PermutationState::End;
                                return None;
                            }
                        }
                        let item = vals.get_at(&indices[0..*k]);
                        *state = PermutationState::Loaded {
                            indices,
                            cycles,
                        };
                        Some(item)
                    }
                }
                PermutationState::Loaded { indices, cycles } => {
                    if advance(indices, cycles) {
                        *state = PermutationState::End;
                        return None;
                    }
                    let k = cycles.len();
                    Some(vals.get_at(&indices[0..k]))
                }
                PermutationState::End => None,
            }
        }
        fn count(self) -> usize {
            let Self { vals, state } = self;
            let n = vals.count();
            state.size_hint_for(n).1.unwrap()
        }
        fn size_hint(&self) -> SizeHint {
            let (mut low, mut upp) = self.vals.size_hint();
            low = self.state.size_hint_for(low).0;
            upp = upp.and_then(|n| self.state.size_hint_for(n).1);
            (low, upp)
        }
    }
    impl<I> FusedIterator for Permutations<I>
    where
        I: Iterator,
        I::Item: Clone,
    {}
    fn advance(indices: &mut [usize], cycles: &mut [usize]) -> bool {
        let n = indices.len();
        let k = cycles.len();
        for i in (0..k).rev() {
            if cycles[i] == 0 {
                cycles[i] = n - i - 1;
                indices[i..].rotate_left(1);
            } else {
                let swap_index = n - cycles[i];
                indices.swap(i, swap_index);
                cycles[i] -= 1;
                return false;
            }
        }
        true
    }
    impl PermutationState {
        fn size_hint_for(&self, n: usize) -> SizeHint {
            let at_start = |n, k| {
                if true {
                    if !(n >= k) {
                        ::core::panicking::panic("assertion failed: n >= k")
                    }
                }
                let total = (n - k + 1..=n)
                    .try_fold(1usize, |acc, i| acc.checked_mul(i));
                (total.unwrap_or(usize::MAX), total)
            };
            match *self {
                Self::Start { k } if n < k => (0, Some(0)),
                Self::Start { k } => at_start(n, k),
                Self::Buffered { k, min_n } => {
                    size_hint::sub_scalar(at_start(n, k), min_n - k + 1)
                }
                Self::Loaded { ref indices, ref cycles } => {
                    let count = cycles
                        .iter()
                        .enumerate()
                        .try_fold(
                            0usize,
                            |acc, (i, &c)| {
                                acc.checked_mul(indices.len() - i)
                                    .and_then(|count| count.checked_add(c))
                            },
                        );
                    (count.unwrap_or(usize::MAX), count)
                }
                Self::End => (0, Some(0)),
            }
        }
    }
}
mod powerset {
    use alloc::vec::Vec;
    use std::fmt;
    use std::iter::FusedIterator;
    use super::combinations::{combinations, Combinations};
    use crate::adaptors::checked_binomial;
    use crate::size_hint::{self, SizeHint};
    /// An iterator to iterate through the powerset of the elements from an iterator.
    ///
    /// See [`.powerset()`](crate::Itertools::powerset) for more
    /// information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Powerset<I: Iterator> {
        combs: Combinations<I>,
    }
    impl<I> Clone for Powerset<I>
    where
        I: Clone + Iterator,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self { combs: self.combs.clone() }
        }
    }
    impl<I> fmt::Debug for Powerset<I>
    where
        I: Iterator + fmt::Debug,
        I::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Powerset").field("combs", &self.combs).finish()
        }
    }
    /// Create a new `Powerset` from a clonable iterator.
    pub fn powerset<I>(src: I) -> Powerset<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        Powerset {
            combs: combinations(src, 0),
        }
    }
    impl<I: Iterator> Powerset<I> {
        /// Returns true if `k` has been incremented, false otherwise.
        fn increment_k(&mut self) -> bool {
            if self.combs.k() < self.combs.n() || self.combs.k() == 0 {
                self.combs.reset(self.combs.k() + 1);
                true
            } else {
                false
            }
        }
    }
    impl<I> Iterator for Powerset<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        type Item = Vec<I::Item>;
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(elt) = self.combs.next() {
                Some(elt)
            } else if self.increment_k() {
                self.combs.next()
            } else {
                None
            }
        }
        fn nth(&mut self, mut n: usize) -> Option<Self::Item> {
            loop {
                match self.combs.try_nth(n) {
                    Ok(item) => return Some(item),
                    Err(steps) => {
                        if !self.increment_k() {
                            return None;
                        }
                        n -= steps;
                    }
                }
            }
        }
        fn size_hint(&self) -> SizeHint {
            let k = self.combs.k();
            let (n_min, n_max) = self.combs.src().size_hint();
            let low = remaining_for(n_min, k).unwrap_or(usize::MAX);
            let upp = n_max.and_then(|n| remaining_for(n, k));
            size_hint::add(self.combs.size_hint(), (low, upp))
        }
        fn count(self) -> usize {
            let k = self.combs.k();
            let (n, combs_count) = self.combs.n_and_count();
            combs_count + remaining_for(n, k).unwrap()
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            let mut it = self.combs;
            if it.k() == 0 {
                init = it.by_ref().fold(init, &mut f);
                it.reset(1);
            }
            init = it.by_ref().fold(init, &mut f);
            for k in it.k() + 1..=it.n() {
                it.reset(k);
                init = it.by_ref().fold(init, &mut f);
            }
            init
        }
    }
    impl<I> FusedIterator for Powerset<I>
    where
        I: Iterator,
        I::Item: Clone,
    {}
    fn remaining_for(n: usize, k: usize) -> Option<usize> {
        (k + 1..=n).try_fold(0usize, |sum, i| sum.checked_add(checked_binomial(n, i)?))
    }
}
mod process_results_impl {
    /// An iterator that produces only the `T` values as long as the
    /// inner iterator produces `Ok(T)`.
    ///
    /// Used by [`process_results`](crate::process_results), see its docs
    /// for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct ProcessResults<'a, I, E: 'a> {
        error: &'a mut Result<(), E>,
        iter: I,
    }
    #[automatically_derived]
    impl<'a, I: ::core::fmt::Debug, E: ::core::fmt::Debug + 'a> ::core::fmt::Debug
    for ProcessResults<'a, I, E> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "ProcessResults",
                "error",
                &self.error,
                "iter",
                &&self.iter,
            )
        }
    }
    impl<I, E> ProcessResults<'_, I, E> {
        #[inline(always)]
        fn next_body<T>(&mut self, item: Option<Result<T, E>>) -> Option<T> {
            match item {
                Some(Ok(x)) => Some(x),
                Some(Err(e)) => {
                    *self.error = Err(e);
                    None
                }
                None => None,
            }
        }
    }
    impl<I, T, E> Iterator for ProcessResults<'_, I, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            let item = self.iter.next();
            self.next_body(item)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.iter.size_hint().1)
        }
        fn fold<B, F>(mut self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let error = self.error;
            self.iter
                .try_fold(
                    init,
                    |acc, opt| match opt {
                        Ok(x) => Ok(f(acc, x)),
                        Err(e) => {
                            *error = Err(e);
                            Err(acc)
                        }
                    },
                )
                .unwrap_or_else(|e| e)
        }
    }
    impl<I, T, E> DoubleEndedIterator for ProcessResults<'_, I, E>
    where
        I: Iterator<Item = Result<T, E>>,
        I: DoubleEndedIterator,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let item = self.iter.next_back();
            self.next_body(item)
        }
        fn rfold<B, F>(mut self, init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            let error = self.error;
            self.iter
                .try_rfold(
                    init,
                    |acc, opt| match opt {
                        Ok(x) => Ok(f(acc, x)),
                        Err(e) => {
                            *error = Err(e);
                            Err(acc)
                        }
                    },
                )
                .unwrap_or_else(|e| e)
        }
    }
    /// “Lift” a function of the values of an iterator so that it can process
    /// an iterator of `Result` values instead.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::process_results`].
    pub fn process_results<I, F, T, E, R>(iterable: I, processor: F) -> Result<R, E>
    where
        I: IntoIterator<Item = Result<T, E>>,
        F: FnOnce(ProcessResults<I::IntoIter, E>) -> R,
    {
        let iter = iterable.into_iter();
        let mut error = Ok(());
        let result = processor(ProcessResults {
            error: &mut error,
            iter,
        });
        error.map(|_| result)
    }
}
mod put_back_n_impl {
    use alloc::vec::Vec;
    use crate::size_hint;
    /// An iterator adaptor that allows putting multiple
    /// items in front of the iterator.
    ///
    /// Iterator element type is `I::Item`.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct PutBackN<I: Iterator> {
        top: Vec<I::Item>,
        iter: I,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Iterator> ::core::fmt::Debug for PutBackN<I>
    where
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PutBackN",
                "top",
                &self.top,
                "iter",
                &&self.iter,
            )
        }
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Iterator> ::core::clone::Clone for PutBackN<I>
    where
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> PutBackN<I> {
            PutBackN {
                top: ::core::clone::Clone::clone(&self.top),
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    /// Create an iterator where you can put back multiple values to the front
    /// of the iteration.
    ///
    /// Iterator element type is `I::Item`.
    pub fn put_back_n<I>(iterable: I) -> PutBackN<I::IntoIter>
    where
        I: IntoIterator,
    {
        PutBackN {
            top: Vec::new(),
            iter: iterable.into_iter(),
        }
    }
    impl<I: Iterator> PutBackN<I> {
        /// Puts `x` in front of the iterator.
        ///
        /// The values are yielded in order of the most recently put back
        /// values first.
        ///
        /// ```rust
        /// use itertools::put_back_n;
        ///
        /// let mut it = put_back_n(1..5);
        /// it.next();
        /// it.put_back(1);
        /// it.put_back(0);
        ///
        /// assert!(itertools::equal(it, 0..5));
        /// ```
        #[inline]
        pub fn put_back(&mut self, x: I::Item) {
            self.top.push(x);
        }
    }
    impl<I: Iterator> Iterator for PutBackN<I> {
        type Item = I::Item;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.top.pop().or_else(|| self.iter.next())
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::add_scalar(self.iter.size_hint(), self.top.len())
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            init = self.top.into_iter().rfold(init, &mut f);
            self.iter.fold(init, f)
        }
    }
}
mod rciter_impl {
    use alloc::rc::Rc;
    use std::cell::RefCell;
    use std::iter::{FusedIterator, IntoIterator};
    /// A wrapper for `Rc<RefCell<I>>`, that implements the `Iterator` trait.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct RcIter<I> {
        /// The boxed iterator.
        pub rciter: Rc<RefCell<I>>,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for RcIter<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RcIter",
                "rciter",
                &&self.rciter,
            )
        }
    }
    /// Return an iterator inside a `Rc<RefCell<_>>` wrapper.
    ///
    /// The returned `RcIter` can be cloned, and each clone will refer back to the
    /// same original iterator.
    ///
    /// `RcIter` allows doing interesting things like using `.zip()` on an iterator with
    /// itself, at the cost of runtime borrow checking which may have a performance
    /// penalty.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::rciter;
    /// use itertools::zip;
    ///
    /// // In this example a range iterator is created and we iterate it using
    /// // three separate handles (two of them given to zip).
    /// // We also use the IntoIterator implementation for `&RcIter`.
    ///
    /// let mut iter = rciter(0..9);
    /// let mut z = zip(&iter, &iter);
    ///
    /// assert_eq!(z.next(), Some((0, 1)));
    /// assert_eq!(z.next(), Some((2, 3)));
    /// assert_eq!(z.next(), Some((4, 5)));
    /// assert_eq!(iter.next(), Some(6));
    /// assert_eq!(z.next(), Some((7, 8)));
    /// assert_eq!(z.next(), None);
    /// ```
    ///
    /// **Panics** in iterator methods if a borrow error is encountered in the
    /// iterator methods. It can only happen if the `RcIter` is reentered in
    /// `.next()`, i.e. if it somehow participates in an “iterator knot”
    /// where it is an adaptor of itself.
    pub fn rciter<I>(iterable: I) -> RcIter<I::IntoIter>
    where
        I: IntoIterator,
    {
        RcIter {
            rciter: Rc::new(RefCell::new(iterable.into_iter())),
        }
    }
    impl<I> Clone for RcIter<I> {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                rciter: self.rciter.clone(),
            }
        }
    }
    impl<A, I> Iterator for RcIter<I>
    where
        I: Iterator<Item = A>,
    {
        type Item = A;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.rciter.borrow_mut().next()
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, self.rciter.borrow().size_hint().1)
        }
    }
    impl<I> DoubleEndedIterator for RcIter<I>
    where
        I: DoubleEndedIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.rciter.borrow_mut().next_back()
        }
    }
    /// Return an iterator from `&RcIter<I>` (by simply cloning it).
    impl<I> IntoIterator for &RcIter<I>
    where
        I: Iterator,
    {
        type Item = I::Item;
        type IntoIter = RcIter<I>;
        fn into_iter(self) -> RcIter<I> {
            self.clone()
        }
    }
    impl<A, I> FusedIterator for RcIter<I>
    where
        I: FusedIterator<Item = A>,
    {}
}
mod repeatn {
    use std::iter::FusedIterator;
    /// An iterator that produces *n* repetitions of an element.
    ///
    /// See [`repeat_n()`](crate::repeat_n) for more information.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub struct RepeatN<A> {
        pub(crate) elt: Option<A>,
        n: usize,
    }
    #[automatically_derived]
    impl<A: ::core::clone::Clone> ::core::clone::Clone for RepeatN<A> {
        #[inline]
        fn clone(&self) -> RepeatN<A> {
            RepeatN {
                elt: ::core::clone::Clone::clone(&self.elt),
                n: ::core::clone::Clone::clone(&self.n),
            }
        }
    }
    #[automatically_derived]
    impl<A: ::core::fmt::Debug> ::core::fmt::Debug for RepeatN<A> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "RepeatN",
                "elt",
                &self.elt,
                "n",
                &&self.n,
            )
        }
    }
    /// Create an iterator that produces `n` repetitions of `element`.
    pub fn repeat_n<A>(element: A, n: usize) -> RepeatN<A>
    where
        A: Clone,
    {
        if n == 0 { RepeatN { elt: None, n } } else { RepeatN { elt: Some(element), n } }
    }
    impl<A> Iterator for RepeatN<A>
    where
        A: Clone,
    {
        type Item = A;
        fn next(&mut self) -> Option<Self::Item> {
            if self.n > 1 {
                self.n -= 1;
                self.elt.as_ref().cloned()
            } else {
                self.n = 0;
                self.elt.take()
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.n, Some(self.n))
        }
        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            match self {
                Self { elt: Some(elt), n } => {
                    if true {
                        if !(n > 0) {
                            ::core::panicking::panic("assertion failed: n > 0")
                        }
                    }
                    init = (1..n).map(|_| elt.clone()).fold(init, &mut f);
                    f(init, elt)
                }
                _ => init,
            }
        }
    }
    impl<A> DoubleEndedIterator for RepeatN<A>
    where
        A: Clone,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            self.next()
        }
        #[inline]
        fn rfold<B, F>(self, init: B, f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            self.fold(init, f)
        }
    }
    impl<A> ExactSizeIterator for RepeatN<A>
    where
        A: Clone,
    {}
    impl<A> FusedIterator for RepeatN<A>
    where
        A: Clone,
    {}
}
mod size_hint {
    //! Arithmetic on `Iterator.size_hint()` values.
    //!
    use std::cmp;
    /// `SizeHint` is the return type of `Iterator::size_hint()`.
    pub type SizeHint = (usize, Option<usize>);
    /// Add `SizeHint` correctly.
    #[inline]
    pub fn add(a: SizeHint, b: SizeHint) -> SizeHint {
        let min = a.0.saturating_add(b.0);
        let max = match (a.1, b.1) {
            (Some(x), Some(y)) => x.checked_add(y),
            _ => None,
        };
        (min, max)
    }
    /// Add `x` correctly to a `SizeHint`.
    #[inline]
    pub fn add_scalar(sh: SizeHint, x: usize) -> SizeHint {
        let (mut low, mut hi) = sh;
        low = low.saturating_add(x);
        hi = hi.and_then(|elt| elt.checked_add(x));
        (low, hi)
    }
    /// Subtract `x` correctly from a `SizeHint`.
    #[inline]
    pub fn sub_scalar(sh: SizeHint, x: usize) -> SizeHint {
        let (mut low, mut hi) = sh;
        low = low.saturating_sub(x);
        hi = hi.map(|elt| elt.saturating_sub(x));
        (low, hi)
    }
    /// Multiply `SizeHint` correctly
    #[inline]
    pub fn mul(a: SizeHint, b: SizeHint) -> SizeHint {
        let low = a.0.saturating_mul(b.0);
        let hi = match (a.1, b.1) {
            (Some(x), Some(y)) => x.checked_mul(y),
            (Some(0), None) | (None, Some(0)) => Some(0),
            _ => None,
        };
        (low, hi)
    }
    /// Multiply `x` correctly with a `SizeHint`.
    #[inline]
    pub fn mul_scalar(sh: SizeHint, x: usize) -> SizeHint {
        let (mut low, mut hi) = sh;
        low = low.saturating_mul(x);
        hi = hi.and_then(|elt| elt.checked_mul(x));
        (low, hi)
    }
    /// Return the maximum
    #[inline]
    pub fn max(a: SizeHint, b: SizeHint) -> SizeHint {
        let (a_lower, a_upper) = a;
        let (b_lower, b_upper) = b;
        let lower = cmp::max(a_lower, b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(x), Some(y)) => Some(cmp::max(x, y)),
            _ => None,
        };
        (lower, upper)
    }
    /// Return the minimum
    #[inline]
    pub fn min(a: SizeHint, b: SizeHint) -> SizeHint {
        let (a_lower, a_upper) = a;
        let (b_lower, b_upper) = b;
        let lower = cmp::min(a_lower, b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(u1), Some(u2)) => Some(cmp::min(u1, u2)),
            _ => a_upper.or(b_upper),
        };
        (lower, upper)
    }
    extern crate test;
    #[rustc_test_marker = "size_hint::mul_size_hints"]
    #[doc(hidden)]
    pub const mul_size_hints: test::TestDescAndFn = test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::StaticTestName("size_hint::mul_size_hints"),
            ignore: false,
            ignore_message: ::core::option::Option::None,
            source_file: "src/size_hint.rs",
            start_line: 90usize,
            start_col: 4usize,
            end_line: 90usize,
            end_col: 18usize,
            compile_fail: false,
            no_run: false,
            should_panic: test::ShouldPanic::No,
            test_type: test::TestType::UnitTest,
        },
        testfn: test::StaticTestFn(
            #[coverage(off)]
            || test::assert_test_result(mul_size_hints()),
        ),
    };
    fn mul_size_hints() {
        match (&mul((3, Some(4)), (3, Some(4))), &(9, Some(16))) {
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
        match (&mul((3, Some(4)), (usize::MAX, None)), &(usize::MAX, None)) {
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
        match (&mul((3, None), (0, Some(0))), &(0, Some(0))) {
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
mod sources {
    //! Iterators that are sources (produce elements from parameters,
    //! not from another iterator).
    #![allow(deprecated)]
    use std::fmt;
    use std::mem;
    /// Creates a new unfold source with the specified closure as the "iterator
    /// function" and an initial state to eventually pass to the closure
    ///
    /// `unfold` is a general iterator builder: it has a mutable state value,
    /// and a closure with access to the state that produces the next value.
    ///
    /// This more or less equivalent to a regular struct with an [`Iterator`]
    /// implementation, and is useful for one-off iterators.
    ///
    /// ```
    /// // an iterator that yields sequential Fibonacci numbers,
    /// // and stops at the maximum representable value.
    ///
    /// use itertools::unfold;
    ///
    /// let mut fibonacci = unfold((1u32, 1u32), |(x1, x2)| {
    ///     // Attempt to get the next Fibonacci number
    ///     let next = x1.saturating_add(*x2);
    ///
    ///     // Shift left: ret <- x1 <- x2 <- next
    ///     let ret = *x1;
    ///     *x1 = *x2;
    ///     *x2 = next;
    ///
    ///     // If addition has saturated at the maximum, we are finished
    ///     if ret == *x1 && ret > 1 {
    ///         None
    ///     } else {
    ///         Some(ret)
    ///     }
    /// });
    ///
    /// itertools::assert_equal(fibonacci.by_ref().take(8),
    ///                         vec![1, 1, 2, 3, 5, 8, 13, 21]);
    /// assert_eq!(fibonacci.last(), Some(2_971_215_073))
    /// ```
    #[deprecated(
        note = "Use [std::iter::from_fn](https://doc.rust-lang.org/std/iter/fn.from_fn.html) instead",
        since = "0.13.0"
    )]
    pub fn unfold<A, St, F>(initial_state: St, f: F) -> Unfold<St, F>
    where
        F: FnMut(&mut St) -> Option<A>,
    {
        Unfold { f, state: initial_state }
    }
    impl<St, F> fmt::Debug for Unfold<St, F>
    where
        St: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Unfold").field("state", &self.state).finish()
        }
    }
    /// See [`unfold`](crate::unfold) for more information.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[deprecated(
        note = "Use [std::iter::FromFn](https://doc.rust-lang.org/std/iter/struct.FromFn.html) instead",
        since = "0.13.0"
    )]
    pub struct Unfold<St, F> {
        f: F,
        /// Internal state that will be passed to the closure on the next iteration
        pub state: St,
    }
    #[automatically_derived]
    impl<St: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for Unfold<St, F> {
        #[inline]
        fn clone(&self) -> Unfold<St, F> {
            Unfold {
                f: ::core::clone::Clone::clone(&self.f),
                state: ::core::clone::Clone::clone(&self.state),
            }
        }
    }
    impl<A, St, F> Iterator for Unfold<St, F>
    where
        F: FnMut(&mut St) -> Option<A>,
    {
        type Item = A;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            (self.f)(&mut self.state)
        }
    }
    /// An iterator that infinitely applies function to value and yields results.
    ///
    /// This `struct` is created by the [`iterate()`](crate::iterate) function.
    /// See its documentation for more.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub struct Iterate<St, F> {
        state: St,
        f: F,
    }
    #[automatically_derived]
    impl<St: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for Iterate<St, F> {
        #[inline]
        fn clone(&self) -> Iterate<St, F> {
            Iterate {
                state: ::core::clone::Clone::clone(&self.state),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<St, F> fmt::Debug for Iterate<St, F>
    where
        St: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Iterate").field("state", &self.state).finish()
        }
    }
    impl<St, F> Iterator for Iterate<St, F>
    where
        F: FnMut(&St) -> St,
    {
        type Item = St;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let next_state = (self.f)(&self.state);
            Some(mem::replace(&mut self.state, next_state))
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, None)
        }
    }
    /// Creates a new iterator that infinitely applies function to value and yields results.
    ///
    /// ```
    /// use itertools::iterate;
    ///
    /// itertools::assert_equal(iterate(1, |i| i % 3 + 1).take(5), vec![1, 2, 3, 1, 2]);
    /// ```
    ///
    /// **Panics** if compute the next value does.
    ///
    /// ```should_panic
    /// # use itertools::iterate;
    /// let mut it = iterate(25u32, |x| x - 10).take_while(|&x| x > 10);
    /// assert_eq!(it.next(), Some(25)); // `Iterate` holds 15.
    /// assert_eq!(it.next(), Some(15)); // `Iterate` holds 5.
    /// it.next(); // `5 - 10` overflows.
    /// ```
    ///
    /// You can alternatively use [`core::iter::successors`] as it better describes a finite iterator.
    pub fn iterate<St, F>(initial_value: St, f: F) -> Iterate<St, F>
    where
        F: FnMut(&St) -> St,
    {
        Iterate { state: initial_value, f }
    }
}
mod take_while_inclusive {
    use core::iter::FusedIterator;
    use std::fmt;
    /// An iterator adaptor that consumes elements while the given predicate is
    /// `true`, including the element for which the predicate first returned
    /// `false`.
    ///
    /// See [`.take_while_inclusive()`](crate::Itertools::take_while_inclusive)
    /// for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct TakeWhileInclusive<I, F> {
        iter: I,
        predicate: F,
        done: bool,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, F: ::core::clone::Clone> ::core::clone::Clone
    for TakeWhileInclusive<I, F> {
        #[inline]
        fn clone(&self) -> TakeWhileInclusive<I, F> {
            TakeWhileInclusive {
                iter: ::core::clone::Clone::clone(&self.iter),
                predicate: ::core::clone::Clone::clone(&self.predicate),
                done: ::core::clone::Clone::clone(&self.done),
            }
        }
    }
    impl<I, F> TakeWhileInclusive<I, F>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> bool,
    {
        /// Create a new [`TakeWhileInclusive`] from an iterator and a predicate.
        pub(crate) fn new(iter: I, predicate: F) -> Self {
            Self {
                iter,
                predicate,
                done: false,
            }
        }
    }
    impl<I, F> fmt::Debug for TakeWhileInclusive<I, F>
    where
        I: Iterator + fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("TakeWhileInclusive")
                .field("iter", &self.iter)
                .field("done", &self.done)
                .finish()
        }
    }
    impl<I, F> Iterator for TakeWhileInclusive<I, F>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> bool,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                None
            } else {
                self.iter
                    .next()
                    .map(|item| {
                        if !(self.predicate)(&item) {
                            self.done = true;
                        }
                        item
                    })
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            if self.done { (0, Some(0)) } else { (0, self.iter.size_hint().1) }
        }
        fn fold<B, Fold>(mut self, init: B, mut f: Fold) -> B
        where
            Fold: FnMut(B, Self::Item) -> B,
        {
            if self.done {
                init
            } else {
                let predicate = &mut self.predicate;
                self.iter
                    .try_fold(
                        init,
                        |mut acc, item| {
                            let is_ok = predicate(&item);
                            acc = f(acc, item);
                            if is_ok { Ok(acc) } else { Err(acc) }
                        },
                    )
                    .unwrap_or_else(|err| err)
            }
        }
    }
    impl<I, F> FusedIterator for TakeWhileInclusive<I, F>
    where
        I: Iterator,
        F: FnMut(&I::Item) -> bool,
    {}
}
mod tee {
    use super::size_hint;
    use alloc::collections::VecDeque;
    use alloc::rc::Rc;
    use std::cell::RefCell;
    /// Common buffer object for the two tee halves
    struct TeeBuffer<A, I> {
        backlog: VecDeque<A>,
        iter: I,
        /// The owner field indicates which id should read from the backlog
        owner: bool,
    }
    #[automatically_derived]
    impl<A: ::core::fmt::Debug, I: ::core::fmt::Debug> ::core::fmt::Debug
    for TeeBuffer<A, I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "TeeBuffer",
                "backlog",
                &self.backlog,
                "iter",
                &self.iter,
                "owner",
                &&self.owner,
            )
        }
    }
    /// One half of an iterator pair where both return the same elements.
    ///
    /// See [`.tee()`](crate::Itertools::tee) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Tee<I>
    where
        I: Iterator,
    {
        rcbuffer: Rc<RefCell<TeeBuffer<I::Item, I>>>,
        id: bool,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for Tee<I>
    where
        I: Iterator,
        I::Item: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Tee",
                "rcbuffer",
                &self.rcbuffer,
                "id",
                &&self.id,
            )
        }
    }
    pub fn new<I>(iter: I) -> (Tee<I>, Tee<I>)
    where
        I: Iterator,
    {
        let buffer = TeeBuffer {
            backlog: VecDeque::new(),
            iter,
            owner: false,
        };
        let t1 = Tee {
            rcbuffer: Rc::new(RefCell::new(buffer)),
            id: true,
        };
        let t2 = Tee {
            rcbuffer: t1.rcbuffer.clone(),
            id: false,
        };
        (t1, t2)
    }
    impl<I> Iterator for Tee<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let mut buffer = self.rcbuffer.borrow_mut();
            if buffer.owner == self.id {
                match buffer.backlog.pop_front() {
                    None => {}
                    some_elt => return some_elt,
                }
            }
            match buffer.iter.next() {
                None => None,
                Some(elt) => {
                    buffer.backlog.push_back(elt.clone());
                    buffer.owner = !self.id;
                    Some(elt)
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let buffer = self.rcbuffer.borrow();
            let sh = buffer.iter.size_hint();
            if buffer.owner == self.id {
                let log_len = buffer.backlog.len();
                size_hint::add_scalar(sh, log_len)
            } else {
                sh
            }
        }
    }
    impl<I> ExactSizeIterator for Tee<I>
    where
        I: ExactSizeIterator,
        I::Item: Clone,
    {}
}
mod tuple_impl {
    //! Some iterator that produces tuples
    use std::iter::Cycle;
    use std::iter::Fuse;
    use std::iter::FusedIterator;
    use crate::size_hint;
    /// Implemented for homogeneous tuples of size up to 12.
    pub trait HomogeneousTuple: TupleCollect {}
    impl<T: TupleCollect> HomogeneousTuple for T {}
    /// An iterator over a incomplete tuple.
    ///
    /// See [`.tuples()`](crate::Itertools::tuples) and
    /// [`Tuples::into_buffer()`].
    pub struct TupleBuffer<T>
    where
        T: HomogeneousTuple,
    {
        cur: usize,
        buf: T::Buffer,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for TupleBuffer<T>
    where
        T: HomogeneousTuple,
        T::Buffer: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> TupleBuffer<T> {
            TupleBuffer {
                cur: ::core::clone::Clone::clone(&self.cur),
                buf: ::core::clone::Clone::clone(&self.buf),
            }
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for TupleBuffer<T>
    where
        T: HomogeneousTuple,
        T::Buffer: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TupleBuffer",
                "cur",
                &self.cur,
                "buf",
                &&self.buf,
            )
        }
    }
    impl<T> TupleBuffer<T>
    where
        T: HomogeneousTuple,
    {
        fn new(buf: T::Buffer) -> Self {
            Self { cur: 0, buf }
        }
    }
    impl<T> Iterator for TupleBuffer<T>
    where
        T: HomogeneousTuple,
    {
        type Item = T::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let s = self.buf.as_mut();
            if let Some(ref mut item) = s.get_mut(self.cur) {
                self.cur += 1;
                item.take()
            } else {
                None
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let buffer = &self.buf.as_ref()[self.cur..];
            let len = if buffer.is_empty() {
                0
            } else {
                buffer.iter().position(|x| x.is_none()).unwrap_or(buffer.len())
            };
            (len, Some(len))
        }
    }
    impl<T> ExactSizeIterator for TupleBuffer<T>
    where
        T: HomogeneousTuple,
    {}
    /// An iterator that groups the items in tuples of a specific size.
    ///
    /// See [`.tuples()`](crate::Itertools::tuples) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        iter: Fuse<I>,
        buf: T::Buffer,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, T: ::core::clone::Clone> ::core::clone::Clone
    for Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
        T::Buffer: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Tuples<I, T> {
            Tuples {
                iter: ::core::clone::Clone::clone(&self.iter),
                buf: ::core::clone::Clone::clone(&self.buf),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, T: ::core::fmt::Debug> ::core::fmt::Debug
    for Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
        T::Buffer: ::core::fmt::Debug,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Tuples",
                "iter",
                &self.iter,
                "buf",
                &&self.buf,
            )
        }
    }
    /// Create a new tuples iterator.
    pub fn tuples<I, T>(iter: I) -> Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        Tuples {
            iter: iter.fuse(),
            buf: Default::default(),
        }
    }
    impl<I, T> Iterator for Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            T::collect_from_iter(&mut self.iter, &mut self.buf)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let buffered = T::buffer_len(&self.buf);
            let (unbuffered_lo, unbuffered_hi) = self.iter.size_hint();
            let total_lo = add_then_div(unbuffered_lo, buffered, T::num_items())
                .unwrap_or(usize::MAX);
            let total_hi = unbuffered_hi
                .and_then(|hi| add_then_div(hi, buffered, T::num_items()));
            (total_lo, total_hi)
        }
    }
    /// `(n + a) / d` avoiding overflow when possible, returns `None` if it overflows.
    fn add_then_div(n: usize, a: usize, d: usize) -> Option<usize> {
        if true {
            match (&d, &0) {
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
        (n / d).checked_add(a / d)?.checked_add((n % d + a % d) / d)
    }
    impl<I, T> ExactSizeIterator for Tuples<I, T>
    where
        I: ExactSizeIterator<Item = T::Item>,
        T: HomogeneousTuple,
    {}
    impl<I, T> Tuples<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        /// Return a buffer with the produced items that was not enough to be grouped in a tuple.
        ///
        /// ```
        /// use itertools::Itertools;
        ///
        /// let mut iter = (0..5).tuples();
        /// assert_eq!(Some((0, 1, 2)), iter.next());
        /// assert_eq!(None, iter.next());
        /// itertools::assert_equal(vec![3, 4], iter.into_buffer());
        /// ```
        pub fn into_buffer(self) -> TupleBuffer<T> {
            TupleBuffer::new(self.buf)
        }
    }
    /// An iterator over all contiguous windows that produces tuples of a specific size.
    ///
    /// See [`.tuple_windows()`](crate::Itertools::tuple_windows) for more
    /// information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct TupleWindows<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        iter: I,
        last: Option<T>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, T: ::core::clone::Clone> ::core::clone::Clone
    for TupleWindows<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        #[inline]
        fn clone(&self) -> TupleWindows<I, T> {
            TupleWindows {
                iter: ::core::clone::Clone::clone(&self.iter),
                last: ::core::clone::Clone::clone(&self.last),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, T: ::core::fmt::Debug> ::core::fmt::Debug
    for TupleWindows<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TupleWindows",
                "iter",
                &self.iter,
                "last",
                &&self.last,
            )
        }
    }
    /// Create a new tuple windows iterator.
    pub fn tuple_windows<I, T>(iter: I) -> TupleWindows<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple,
        T::Item: Clone,
    {
        TupleWindows { last: None, iter }
    }
    impl<I, T> Iterator for TupleWindows<I, T>
    where
        I: Iterator<Item = T::Item>,
        T: HomogeneousTuple + Clone,
        T::Item: Clone,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            if T::num_items() == 1 {
                return T::collect_from_iter_no_buf(&mut self.iter);
            }
            if let Some(new) = self.iter.next() {
                if let Some(ref mut last) = self.last {
                    last.left_shift_push(new);
                    Some(last.clone())
                } else {
                    use std::iter::once;
                    let iter = once(new).chain(&mut self.iter);
                    self.last = T::collect_from_iter_no_buf(iter);
                    self.last.clone()
                }
            } else {
                None
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let mut sh = self.iter.size_hint();
            if self.last.is_none() {
                sh = size_hint::sub_scalar(sh, T::num_items() - 1);
            }
            sh
        }
    }
    impl<I, T> ExactSizeIterator for TupleWindows<I, T>
    where
        I: ExactSizeIterator<Item = T::Item>,
        T: HomogeneousTuple + Clone,
        T::Item: Clone,
    {}
    impl<I, T> FusedIterator for TupleWindows<I, T>
    where
        I: FusedIterator<Item = T::Item>,
        T: HomogeneousTuple + Clone,
        T::Item: Clone,
    {}
    /// An iterator over all windows, wrapping back to the first elements when the
    /// window would otherwise exceed the length of the iterator, producing tuples
    /// of a specific size.
    ///
    /// See [`.circular_tuple_windows()`](crate::Itertools::circular_tuple_windows) for more
    /// information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
    {
        iter: TupleWindows<Cycle<I>, T>,
        len: usize,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, T: ::core::fmt::Debug> ::core::fmt::Debug
    for CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
    {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "CircularTupleWindows",
                "iter",
                &self.iter,
                "len",
                &&self.len,
            )
        }
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, T: ::core::clone::Clone> ::core::clone::Clone
    for CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
    {
        #[inline]
        fn clone(&self) -> CircularTupleWindows<I, T> {
            CircularTupleWindows {
                iter: ::core::clone::Clone::clone(&self.iter),
                len: ::core::clone::Clone::clone(&self.len),
            }
        }
    }
    pub fn circular_tuple_windows<I, T>(iter: I) -> CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone + ExactSizeIterator,
        T: TupleCollect + Clone,
        T::Item: Clone,
    {
        let len = iter.len();
        let iter = tuple_windows(iter.cycle());
        CircularTupleWindows { iter, len }
    }
    impl<I, T> Iterator for CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
        T::Item: Clone,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            if self.len != 0 {
                self.len -= 1;
                self.iter.next()
            } else {
                None
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.len, Some(self.len))
        }
    }
    impl<I, T> ExactSizeIterator for CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
        T::Item: Clone,
    {}
    impl<I, T> FusedIterator for CircularTupleWindows<I, T>
    where
        I: Iterator<Item = T::Item> + Clone,
        T: TupleCollect + Clone,
        T::Item: Clone,
    {}
    pub trait TupleCollect: Sized {
        type Item;
        type Buffer: Default + AsRef<[Option<Self::Item>]> + AsMut<[Option<Self::Item>]>;
        fn buffer_len(buf: &Self::Buffer) -> usize {
            let s = buf.as_ref();
            s.iter().position(Option::is_none).unwrap_or(s.len())
        }
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = Self::Item>;
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = Self::Item>;
        fn num_items() -> usize;
        fn left_shift_push(&mut self, item: Self::Item);
    }
    impl<A> TupleCollect for (A,) {
        type Item = A;
        type Buffer = [Option<A>; 1 + 0 - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut l = None;
            loop {
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((l.unwrap(),));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + 0
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (ref mut l,) = self;
            item = replace(l, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + 0) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut k = None;
            let mut l = None;
            loop {
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((k.unwrap(), l.unwrap()));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + 0)
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (ref mut k, ref mut l) = self;
            item = replace(l, item);
            item = replace(k, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + 0)) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((j.unwrap(), k.unwrap(), l.unwrap()));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + 0))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (ref mut j, ref mut k, ref mut l) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + (1 + 0))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((i.unwrap(), j.unwrap(), k.unwrap(), l.unwrap()));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + 0)))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (ref mut i, ref mut j, ref mut k, ref mut l) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + (1 + (1 + 0)))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + 0))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (ref mut h, ref mut i, ref mut j, ref mut k, ref mut l) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + (1 + (1 + (1 + 0))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + 0)))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<A>; 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut e = None;
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                e = iter.next();
                if e.is_none() {
                    break;
                }
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    e.unwrap(),
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = e;
                i += 1;
            }
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let e = iter.next()?;
                    e
                },
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut e,
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            item = replace(e, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<
            A,
        >; 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut d = None;
            let mut e = None;
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                d = iter.next();
                if d.is_none() {
                    break;
                }
                e = iter.next();
                if e.is_none() {
                    break;
                }
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    d.unwrap(),
                    e.unwrap(),
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = d;
                i += 1;
            }
            if i < s.len() {
                s[i] = e;
                i += 1;
            }
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let d = iter.next()?;
                    d
                },
                {
                    let e = iter.next()?;
                    e
                },
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut d,
                ref mut e,
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            item = replace(e, item);
            item = replace(d, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<
            A,
        >; 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut c = None;
            let mut d = None;
            let mut e = None;
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                c = iter.next();
                if c.is_none() {
                    break;
                }
                d = iter.next();
                if d.is_none() {
                    break;
                }
                e = iter.next();
                if e.is_none() {
                    break;
                }
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    c.unwrap(),
                    d.unwrap(),
                    e.unwrap(),
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = c;
                i += 1;
            }
            if i < s.len() {
                s[i] = d;
                i += 1;
            }
            if i < s.len() {
                s[i] = e;
                i += 1;
            }
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let c = iter.next()?;
                    c
                },
                {
                    let d = iter.next()?;
                    d
                },
                {
                    let e = iter.next()?;
                    e
                },
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut c,
                ref mut d,
                ref mut e,
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            item = replace(e, item);
            item = replace(d, item);
            item = replace(c, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<
            A,
        >; 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut b = None;
            let mut c = None;
            let mut d = None;
            let mut e = None;
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                b = iter.next();
                if b.is_none() {
                    break;
                }
                c = iter.next();
                if c.is_none() {
                    break;
                }
                d = iter.next();
                if d.is_none() {
                    break;
                }
                e = iter.next();
                if e.is_none() {
                    break;
                }
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    b.unwrap(),
                    c.unwrap(),
                    d.unwrap(),
                    e.unwrap(),
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = b;
                i += 1;
            }
            if i < s.len() {
                s[i] = c;
                i += 1;
            }
            if i < s.len() {
                s[i] = d;
                i += 1;
            }
            if i < s.len() {
                s[i] = e;
                i += 1;
            }
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let b = iter.next()?;
                    b
                },
                {
                    let c = iter.next()?;
                    c
                },
                {
                    let d = iter.next()?;
                    d
                },
                {
                    let e = iter.next()?;
                    e
                },
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut b,
                ref mut c,
                ref mut d,
                ref mut e,
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            item = replace(e, item);
            item = replace(d, item);
            item = replace(c, item);
            item = replace(b, item);
            drop(item);
        }
    }
    impl<A> TupleCollect for (A, A, A, A, A, A, A, A, A, A, A, A) {
        type Item = A;
        type Buffer = [Option<
            A,
        >; 1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0))))))))))) - 1];
        #[allow(unused_assignments, unused_mut)]
        fn collect_from_iter<I>(iter: I, buf: &mut Self::Buffer) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            let mut a = None;
            let mut b = None;
            let mut c = None;
            let mut d = None;
            let mut e = None;
            let mut f = None;
            let mut g = None;
            let mut h = None;
            let mut i = None;
            let mut j = None;
            let mut k = None;
            let mut l = None;
            loop {
                a = iter.next();
                if a.is_none() {
                    break;
                }
                b = iter.next();
                if b.is_none() {
                    break;
                }
                c = iter.next();
                if c.is_none() {
                    break;
                }
                d = iter.next();
                if d.is_none() {
                    break;
                }
                e = iter.next();
                if e.is_none() {
                    break;
                }
                f = iter.next();
                if f.is_none() {
                    break;
                }
                g = iter.next();
                if g.is_none() {
                    break;
                }
                h = iter.next();
                if h.is_none() {
                    break;
                }
                i = iter.next();
                if i.is_none() {
                    break;
                }
                j = iter.next();
                if j.is_none() {
                    break;
                }
                k = iter.next();
                if k.is_none() {
                    break;
                }
                l = iter.next();
                if l.is_none() {
                    break;
                }
                return Some((
                    a.unwrap(),
                    b.unwrap(),
                    c.unwrap(),
                    d.unwrap(),
                    e.unwrap(),
                    f.unwrap(),
                    g.unwrap(),
                    h.unwrap(),
                    i.unwrap(),
                    j.unwrap(),
                    k.unwrap(),
                    l.unwrap(),
                ));
            }
            let mut i = 0;
            let mut s = buf.as_mut();
            if i < s.len() {
                s[i] = a;
                i += 1;
            }
            if i < s.len() {
                s[i] = b;
                i += 1;
            }
            if i < s.len() {
                s[i] = c;
                i += 1;
            }
            if i < s.len() {
                s[i] = d;
                i += 1;
            }
            if i < s.len() {
                s[i] = e;
                i += 1;
            }
            if i < s.len() {
                s[i] = f;
                i += 1;
            }
            if i < s.len() {
                s[i] = g;
                i += 1;
            }
            if i < s.len() {
                s[i] = h;
                i += 1;
            }
            if i < s.len() {
                s[i] = i;
                i += 1;
            }
            if i < s.len() {
                s[i] = j;
                i += 1;
            }
            if i < s.len() {
                s[i] = k;
                i += 1;
            }
            if i < s.len() {
                s[i] = l;
                i += 1;
            }
            return None;
        }
        fn collect_from_iter_no_buf<I>(iter: I) -> Option<Self>
        where
            I: IntoIterator<Item = A>,
        {
            let mut iter = iter.into_iter();
            Some((
                {
                    let a = iter.next()?;
                    a
                },
                {
                    let b = iter.next()?;
                    b
                },
                {
                    let c = iter.next()?;
                    c
                },
                {
                    let d = iter.next()?;
                    d
                },
                {
                    let e = iter.next()?;
                    e
                },
                {
                    let f = iter.next()?;
                    f
                },
                {
                    let g = iter.next()?;
                    g
                },
                {
                    let h = iter.next()?;
                    h
                },
                {
                    let i = iter.next()?;
                    i
                },
                {
                    let j = iter.next()?;
                    j
                },
                {
                    let k = iter.next()?;
                    k
                },
                {
                    let l = iter.next()?;
                    l
                },
            ))
        }
        fn num_items() -> usize {
            1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + (1 + 0)))))))))))
        }
        fn left_shift_push(&mut self, mut item: A) {
            use std::mem::replace;
            let &mut (
                ref mut a,
                ref mut b,
                ref mut c,
                ref mut d,
                ref mut e,
                ref mut f,
                ref mut g,
                ref mut h,
                ref mut i,
                ref mut j,
                ref mut k,
                ref mut l,
            ) = self;
            item = replace(l, item);
            item = replace(k, item);
            item = replace(j, item);
            item = replace(i, item);
            item = replace(h, item);
            item = replace(g, item);
            item = replace(f, item);
            item = replace(e, item);
            item = replace(d, item);
            item = replace(c, item);
            item = replace(b, item);
            item = replace(a, item);
            drop(item);
        }
    }
}
mod unique_impl {
    use std::collections::hash_map::Entry;
    use std::collections::HashMap;
    use std::fmt;
    use std::hash::Hash;
    use std::iter::FusedIterator;
    /// An iterator adapter to filter out duplicate elements.
    ///
    /// See [`.unique_by()`](crate::Itertools::unique) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct UniqueBy<I: Iterator, V, F> {
        iter: I,
        used: HashMap<V, ()>,
        f: F,
    }
    #[automatically_derived]
    impl<
        I: ::core::clone::Clone + Iterator,
        V: ::core::clone::Clone,
        F: ::core::clone::Clone,
    > ::core::clone::Clone for UniqueBy<I, V, F> {
        #[inline]
        fn clone(&self) -> UniqueBy<I, V, F> {
            UniqueBy {
                iter: ::core::clone::Clone::clone(&self.iter),
                used: ::core::clone::Clone::clone(&self.used),
                f: ::core::clone::Clone::clone(&self.f),
            }
        }
    }
    impl<I, V, F> fmt::Debug for UniqueBy<I, V, F>
    where
        I: Iterator + fmt::Debug,
        V: fmt::Debug + Hash + Eq,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("UniqueBy")
                .field("iter", &self.iter)
                .field("used", &self.used)
                .finish()
        }
    }
    /// Create a new `UniqueBy` iterator.
    pub fn unique_by<I, V, F>(iter: I, f: F) -> UniqueBy<I, V, F>
    where
        V: Eq + Hash,
        F: FnMut(&I::Item) -> V,
        I: Iterator,
    {
        UniqueBy {
            iter,
            used: HashMap::new(),
            f,
        }
    }
    fn count_new_keys<I, K>(mut used: HashMap<K, ()>, iterable: I) -> usize
    where
        I: IntoIterator<Item = K>,
        K: Hash + Eq,
    {
        let iter = iterable.into_iter();
        let current_used = used.len();
        used.extend(iter.map(|key| (key, ())));
        used.len() - current_used
    }
    impl<I, V, F> Iterator for UniqueBy<I, V, F>
    where
        I: Iterator,
        V: Eq + Hash,
        F: FnMut(&I::Item) -> V,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let Self { iter, used, f } = self;
            iter.find(|v| used.insert(f(v), ()).is_none())
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (low, hi) = self.iter.size_hint();
            ((low > 0 && self.used.is_empty()) as usize, hi)
        }
        fn count(self) -> usize {
            let mut key_f = self.f;
            count_new_keys(self.used, self.iter.map(move |elt| key_f(&elt)))
        }
    }
    impl<I, V, F> DoubleEndedIterator for UniqueBy<I, V, F>
    where
        I: DoubleEndedIterator,
        V: Eq + Hash,
        F: FnMut(&I::Item) -> V,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let Self { iter, used, f } = self;
            iter.rfind(|v| used.insert(f(v), ()).is_none())
        }
    }
    impl<I, V, F> FusedIterator for UniqueBy<I, V, F>
    where
        I: FusedIterator,
        V: Eq + Hash,
        F: FnMut(&I::Item) -> V,
    {}
    impl<I> Iterator for Unique<I>
    where
        I: Iterator,
        I::Item: Eq + Hash + Clone,
    {
        type Item = I::Item;
        fn next(&mut self) -> Option<Self::Item> {
            let UniqueBy { iter, used, .. } = &mut self.iter;
            iter.find_map(|v| {
                if let Entry::Vacant(entry) = used.entry(v) {
                    let elt = entry.key().clone();
                    entry.insert(());
                    return Some(elt);
                }
                None
            })
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            let (low, hi) = self.iter.iter.size_hint();
            ((low > 0 && self.iter.used.is_empty()) as usize, hi)
        }
        fn count(self) -> usize {
            count_new_keys(self.iter.used, self.iter.iter)
        }
    }
    impl<I> DoubleEndedIterator for Unique<I>
    where
        I: DoubleEndedIterator,
        I::Item: Eq + Hash + Clone,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            let UniqueBy { iter, used, .. } = &mut self.iter;
            iter.rev()
                .find_map(|v| {
                    if let Entry::Vacant(entry) = used.entry(v) {
                        let elt = entry.key().clone();
                        entry.insert(());
                        return Some(elt);
                    }
                    None
                })
        }
    }
    impl<I> FusedIterator for Unique<I>
    where
        I: FusedIterator,
        I::Item: Eq + Hash + Clone,
    {}
    /// An iterator adapter to filter out duplicate elements.
    ///
    /// See [`.unique()`](crate::Itertools::unique) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Unique<I>
    where
        I: Iterator,
        I::Item: Eq + Hash + Clone,
    {
        iter: UniqueBy<I, I::Item, ()>,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone> ::core::clone::Clone for Unique<I>
    where
        I: Iterator,
        I::Item: Eq + Hash + Clone,
        I::Item: ::core::clone::Clone,
    {
        #[inline]
        fn clone(&self) -> Unique<I> {
            Unique {
                iter: ::core::clone::Clone::clone(&self.iter),
            }
        }
    }
    impl<I> fmt::Debug for Unique<I>
    where
        I: Iterator + fmt::Debug,
        I::Item: Hash + Eq + fmt::Debug + Clone,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("Unique").field("iter", &self.iter).finish()
        }
    }
    pub fn unique<I>(iter: I) -> Unique<I>
    where
        I: Iterator,
        I::Item: Eq + Hash + Clone,
    {
        Unique {
            iter: UniqueBy {
                iter,
                used: HashMap::new(),
                f: (),
            },
        }
    }
}
mod unziptuple {
    /// Converts an iterator of tuples into a tuple of containers.
    ///
    /// `multiunzip()` consumes an entire iterator of n-ary tuples, producing `n` collections, one for each
    /// column.
    ///
    /// This function is, in some sense, the opposite of [`multizip`].
    ///
    /// ```
    /// use itertools::multiunzip;
    ///
    /// let inputs = vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)];
    ///
    /// let (a, b, c): (Vec<_>, Vec<_>, Vec<_>) = multiunzip(inputs);
    ///
    /// assert_eq!(a, vec![1, 4, 7]);
    /// assert_eq!(b, vec![2, 5, 8]);
    /// assert_eq!(c, vec![3, 6, 9]);
    /// ```
    ///
    /// [`multizip`]: crate::multizip
    pub fn multiunzip<FromI, I>(i: I) -> FromI
    where
        I: IntoIterator,
        I::IntoIter: MultiUnzip<FromI>,
    {
        i.into_iter().multiunzip()
    }
    /// An iterator that can be unzipped into multiple collections.
    ///
    /// See [`.multiunzip()`](crate::Itertools::multiunzip) for more information.
    pub trait MultiUnzip<FromI>: Iterator {
        /// Unzip this iterator into multiple collections.
        fn multiunzip(self) -> FromI;
    }
    #[allow(non_snake_case)]
    impl<IT: Iterator<Item = ()>> MultiUnzip<()> for IT {
        fn multiunzip(self) -> () {
            let mut res = ();
            let () = &mut res;
            self.fold((), |(), ()| {});
            res
        }
    }
    #[allow(non_snake_case)]
    impl<IT: Iterator<Item = (A,)>, A, FromA: Default + Extend<A>> MultiUnzip<(FromA,)>
    for IT {
        fn multiunzip(self) -> (FromA,) {
            let mut res = (FromA::default(),);
            let (FromA,) = &mut res;
            self.fold(
                (),
                |(), (A,)| {
                    FromA.extend(std::iter::once(A));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
    > MultiUnzip<(FromA, FromB)> for IT {
        fn multiunzip(self) -> (FromA, FromB) {
            let mut res = (FromA::default(), FromB::default());
            let (FromA, FromB) = &mut res;
            self.fold(
                (),
                |(), (A, B)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
    > MultiUnzip<(FromA, FromB, FromC)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC) {
            let mut res = (FromA::default(), FromB::default(), FromC::default());
            let (FromA, FromB, FromC) = &mut res;
            self.fold(
                (),
                |(), (A, B, C)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
    > MultiUnzip<(FromA, FromB, FromC, FromD)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC, FromD) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
            );
            let (FromA, FromB, FromC, FromD) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC, FromD, FromE) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE, FromF)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC, FromD, FromE, FromF) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE, FromF) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE, FromF, FromG)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC, FromD, FromE, FromF, FromG) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE, FromF, FromG) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G, H)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
        H,
        FromH: Default + Extend<H>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH)> for IT {
        fn multiunzip(self) -> (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
                FromH::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G, H)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                    FromH.extend(std::iter::once(H));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G, H, I)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
        H,
        FromH: Default + Extend<H>,
        I,
        FromI: Default + Extend<I>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI)>
    for IT {
        fn multiunzip(
            self,
        ) -> (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
                FromH::default(),
                FromI::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G, H, I)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                    FromH.extend(std::iter::once(H));
                    FromI.extend(std::iter::once(I));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G, H, I, J)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
        H,
        FromH: Default + Extend<H>,
        I,
        FromI: Default + Extend<I>,
        J,
        FromJ: Default + Extend<J>,
    > MultiUnzip<(FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ)>
    for IT {
        fn multiunzip(
            self,
        ) -> (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
                FromH::default(),
                FromI::default(),
                FromJ::default(),
            );
            let (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G, H, I, J)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                    FromH.extend(std::iter::once(H));
                    FromI.extend(std::iter::once(I));
                    FromJ.extend(std::iter::once(J));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G, H, I, J, K)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
        H,
        FromH: Default + Extend<H>,
        I,
        FromI: Default + Extend<I>,
        J,
        FromJ: Default + Extend<J>,
        K,
        FromK: Default + Extend<K>,
    > MultiUnzip<
        (FromA, FromB, FromC, FromD, FromE, FromF, FromG, FromH, FromI, FromJ, FromK),
    > for IT {
        fn multiunzip(
            self,
        ) -> (
            FromA,
            FromB,
            FromC,
            FromD,
            FromE,
            FromF,
            FromG,
            FromH,
            FromI,
            FromJ,
            FromK,
        ) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
                FromH::default(),
                FromI::default(),
                FromJ::default(),
                FromK::default(),
            );
            let (
                FromA,
                FromB,
                FromC,
                FromD,
                FromE,
                FromF,
                FromG,
                FromH,
                FromI,
                FromJ,
                FromK,
            ) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G, H, I, J, K)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                    FromH.extend(std::iter::once(H));
                    FromI.extend(std::iter::once(I));
                    FromJ.extend(std::iter::once(J));
                    FromK.extend(std::iter::once(K));
                },
            );
            res
        }
    }
    #[allow(non_snake_case)]
    impl<
        IT: Iterator<Item = (A, B, C, D, E, F, G, H, I, J, K, L)>,
        A,
        FromA: Default + Extend<A>,
        B,
        FromB: Default + Extend<B>,
        C,
        FromC: Default + Extend<C>,
        D,
        FromD: Default + Extend<D>,
        E,
        FromE: Default + Extend<E>,
        F,
        FromF: Default + Extend<F>,
        G,
        FromG: Default + Extend<G>,
        H,
        FromH: Default + Extend<H>,
        I,
        FromI: Default + Extend<I>,
        J,
        FromJ: Default + Extend<J>,
        K,
        FromK: Default + Extend<K>,
        L,
        FromL: Default + Extend<L>,
    > MultiUnzip<
        (
            FromA,
            FromB,
            FromC,
            FromD,
            FromE,
            FromF,
            FromG,
            FromH,
            FromI,
            FromJ,
            FromK,
            FromL,
        ),
    > for IT {
        fn multiunzip(
            self,
        ) -> (
            FromA,
            FromB,
            FromC,
            FromD,
            FromE,
            FromF,
            FromG,
            FromH,
            FromI,
            FromJ,
            FromK,
            FromL,
        ) {
            let mut res = (
                FromA::default(),
                FromB::default(),
                FromC::default(),
                FromD::default(),
                FromE::default(),
                FromF::default(),
                FromG::default(),
                FromH::default(),
                FromI::default(),
                FromJ::default(),
                FromK::default(),
                FromL::default(),
            );
            let (
                FromA,
                FromB,
                FromC,
                FromD,
                FromE,
                FromF,
                FromG,
                FromH,
                FromI,
                FromJ,
                FromK,
                FromL,
            ) = &mut res;
            self.fold(
                (),
                |(), (A, B, C, D, E, F, G, H, I, J, K, L)| {
                    FromA.extend(std::iter::once(A));
                    FromB.extend(std::iter::once(B));
                    FromC.extend(std::iter::once(C));
                    FromD.extend(std::iter::once(D));
                    FromE.extend(std::iter::once(E));
                    FromF.extend(std::iter::once(F));
                    FromG.extend(std::iter::once(G));
                    FromH.extend(std::iter::once(H));
                    FromI.extend(std::iter::once(I));
                    FromJ.extend(std::iter::once(J));
                    FromK.extend(std::iter::once(K));
                    FromL.extend(std::iter::once(L));
                },
            );
            res
        }
    }
}
mod with_position {
    use std::fmt;
    use std::iter::{Fuse, FusedIterator, Peekable};
    /// An iterator adaptor that wraps each element in an [`Position`].
    ///
    /// Iterator element type is `(Position, I::Item)`.
    ///
    /// See [`.with_position()`](crate::Itertools::with_position) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct WithPosition<I>
    where
        I: Iterator,
    {
        handled_first: bool,
        peekable: Peekable<Fuse<I>>,
    }
    impl<I> fmt::Debug for WithPosition<I>
    where
        I: Iterator,
        Peekable<Fuse<I>>: fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("WithPosition")
                .field("handled_first", &self.handled_first)
                .field("peekable", &self.peekable)
                .finish()
        }
    }
    impl<I> Clone for WithPosition<I>
    where
        I: Clone + Iterator,
        I::Item: Clone,
    {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                handled_first: self.handled_first.clone(),
                peekable: self.peekable.clone(),
            }
        }
    }
    /// Create a new `WithPosition` iterator.
    pub fn with_position<I>(iter: I) -> WithPosition<I>
    where
        I: Iterator,
    {
        WithPosition {
            handled_first: false,
            peekable: iter.fuse().peekable(),
        }
    }
    /// The first component of the value yielded by `WithPosition`.
    /// Indicates the position of this element in the iterator results.
    ///
    /// See [`.with_position()`](crate::Itertools::with_position) for more information.
    pub enum Position {
        /// This is the first element.
        First,
        /// This is neither the first nor the last element.
        Middle,
        /// This is the last element.
        Last,
        /// This is the only element.
        Only,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Position {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Position {}
    #[automatically_derived]
    impl ::core::clone::Clone for Position {
        #[inline]
        fn clone(&self) -> Position {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Position {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Position::First => "First",
                    Position::Middle => "Middle",
                    Position::Last => "Last",
                    Position::Only => "Only",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Position {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Position {
        #[inline]
        fn eq(&self, other: &Position) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Position {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    impl<I: Iterator> Iterator for WithPosition<I> {
        type Item = (Position, I::Item);
        fn next(&mut self) -> Option<Self::Item> {
            match self.peekable.next() {
                Some(item) => {
                    if !self.handled_first {
                        self.handled_first = true;
                        match self.peekable.peek() {
                            Some(_) => Some((Position::First, item)),
                            None => Some((Position::Only, item)),
                        }
                    } else {
                        match self.peekable.peek() {
                            Some(_) => Some((Position::Middle, item)),
                            None => Some((Position::Last, item)),
                        }
                    }
                }
                None => None,
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.peekable.size_hint()
        }
        fn fold<B, F>(mut self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            if let Some(mut head) = self.peekable.next() {
                if !self.handled_first {
                    match self.peekable.next() {
                        Some(second) => {
                            let first = std::mem::replace(&mut head, second);
                            init = f(init, (Position::First, first));
                        }
                        None => return f(init, (Position::Only, head)),
                    }
                }
                init = self
                    .peekable
                    .fold(
                        init,
                        |acc, mut item| {
                            std::mem::swap(&mut head, &mut item);
                            f(acc, (Position::Middle, item))
                        },
                    );
                init = f(init, (Position::Last, head));
            }
            init
        }
    }
    impl<I> ExactSizeIterator for WithPosition<I>
    where
        I: ExactSizeIterator,
    {}
    impl<I: Iterator> FusedIterator for WithPosition<I> {}
}
mod zip_eq_impl {
    use super::size_hint;
    /// An iterator which iterates two other iterators simultaneously
    /// and panic if they have different lengths.
    ///
    /// See [`.zip_eq()`](crate::Itertools::zip_eq) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct ZipEq<I, J> {
        a: I,
        b: J,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, J: ::core::clone::Clone> ::core::clone::Clone
    for ZipEq<I, J> {
        #[inline]
        fn clone(&self) -> ZipEq<I, J> {
            ZipEq {
                a: ::core::clone::Clone::clone(&self.a),
                b: ::core::clone::Clone::clone(&self.b),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, J: ::core::fmt::Debug> ::core::fmt::Debug
    for ZipEq<I, J> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "ZipEq",
                "a",
                &self.a,
                "b",
                &&self.b,
            )
        }
    }
    /// Zips two iterators but **panics** if they are not of the same length.
    ///
    /// [`IntoIterator`] enabled version of [`Itertools::zip_eq`](crate::Itertools::zip_eq).
    ///
    /// ```
    /// use itertools::zip_eq;
    ///
    /// let data = [1, 2, 3, 4, 5];
    /// for (a, b) in zip_eq(&data[..data.len() - 1], &data[1..]) {
    ///     /* loop body */
    ///     # let _ = (a, b);
    /// }
    /// ```
    pub fn zip_eq<I, J>(i: I, j: J) -> ZipEq<I::IntoIter, J::IntoIter>
    where
        I: IntoIterator,
        J: IntoIterator,
    {
        ZipEq {
            a: i.into_iter(),
            b: j.into_iter(),
        }
    }
    impl<I, J> Iterator for ZipEq<I, J>
    where
        I: Iterator,
        J: Iterator,
    {
        type Item = (I::Item, J::Item);
        fn next(&mut self) -> Option<Self::Item> {
            match (self.a.next(), self.b.next()) {
                (None, None) => None,
                (Some(a), Some(b)) => Some((a, b)),
                (None, Some(_)) | (Some(_), None) => {
                    ::std::rt::begin_panic(
                        "itertools: .zip_eq() reached end of one iterator before the other",
                    );
                }
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::min(self.a.size_hint(), self.b.size_hint())
        }
    }
    impl<I, J> ExactSizeIterator for ZipEq<I, J>
    where
        I: ExactSizeIterator,
        J: ExactSizeIterator,
    {}
}
mod zip_longest {
    use super::size_hint;
    use std::cmp::Ordering::{Equal, Greater, Less};
    use std::iter::{Fuse, FusedIterator};
    use crate::either_or_both::EitherOrBoth;
    /// An iterator which iterates two other iterators simultaneously
    /// and wraps the elements in [`EitherOrBoth`].
    ///
    /// This iterator is *fused*.
    ///
    /// See [`.zip_longest()`](crate::Itertools::zip_longest) for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct ZipLongest<T, U> {
        a: Fuse<T>,
        b: Fuse<U>,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone, U: ::core::clone::Clone> ::core::clone::Clone
    for ZipLongest<T, U> {
        #[inline]
        fn clone(&self) -> ZipLongest<T, U> {
            ZipLongest {
                a: ::core::clone::Clone::clone(&self.a),
                b: ::core::clone::Clone::clone(&self.b),
            }
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug, U: ::core::fmt::Debug> ::core::fmt::Debug
    for ZipLongest<T, U> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "ZipLongest",
                "a",
                &self.a,
                "b",
                &&self.b,
            )
        }
    }
    /// Create a new `ZipLongest` iterator.
    pub fn zip_longest<T, U>(a: T, b: U) -> ZipLongest<T, U>
    where
        T: Iterator,
        U: Iterator,
    {
        ZipLongest {
            a: a.fuse(),
            b: b.fuse(),
        }
    }
    impl<T, U> Iterator for ZipLongest<T, U>
    where
        T: Iterator,
        U: Iterator,
    {
        type Item = EitherOrBoth<T::Item, U::Item>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            match (self.a.next(), self.b.next()) {
                (None, None) => None,
                (Some(a), None) => Some(EitherOrBoth::Left(a)),
                (None, Some(b)) => Some(EitherOrBoth::Right(b)),
                (Some(a), Some(b)) => Some(EitherOrBoth::Both(a, b)),
            }
        }
        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            size_hint::max(self.a.size_hint(), self.b.size_hint())
        }
        #[inline]
        fn fold<B, F>(self, init: B, mut f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            let Self { mut a, mut b } = self;
            let res = a
                .try_fold(
                    init,
                    |init, a| match b.next() {
                        Some(b) => Ok(f(init, EitherOrBoth::Both(a, b))),
                        None => Err(f(init, EitherOrBoth::Left(a))),
                    },
                );
            match res {
                Ok(acc) => b.map(EitherOrBoth::Right).fold(acc, f),
                Err(acc) => a.map(EitherOrBoth::Left).fold(acc, f),
            }
        }
    }
    impl<T, U> DoubleEndedIterator for ZipLongest<T, U>
    where
        T: DoubleEndedIterator + ExactSizeIterator,
        U: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            match self.a.len().cmp(&self.b.len()) {
                Equal => {
                    match (self.a.next_back(), self.b.next_back()) {
                        (None, None) => None,
                        (Some(a), Some(b)) => Some(EitherOrBoth::Both(a, b)),
                        (Some(a), None) => Some(EitherOrBoth::Left(a)),
                        (None, Some(b)) => Some(EitherOrBoth::Right(b)),
                    }
                }
                Greater => self.a.next_back().map(EitherOrBoth::Left),
                Less => self.b.next_back().map(EitherOrBoth::Right),
            }
        }
        fn rfold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Self::Item) -> B,
        {
            let Self { mut a, mut b } = self;
            let a_len = a.len();
            let b_len = b.len();
            match a_len.cmp(&b_len) {
                Equal => {}
                Greater => {
                    init = a
                        .by_ref()
                        .rev()
                        .take(a_len - b_len)
                        .map(EitherOrBoth::Left)
                        .fold(init, &mut f);
                }
                Less => {
                    init = b
                        .by_ref()
                        .rev()
                        .take(b_len - a_len)
                        .map(EitherOrBoth::Right)
                        .fold(init, &mut f);
                }
            }
            a.rfold(
                init,
                |acc, item_a| {
                    f(acc, EitherOrBoth::Both(item_a, b.next_back().unwrap()))
                },
            )
        }
    }
    impl<T, U> ExactSizeIterator for ZipLongest<T, U>
    where
        T: ExactSizeIterator,
        U: ExactSizeIterator,
    {}
    impl<T, U> FusedIterator for ZipLongest<T, U>
    where
        T: Iterator,
        U: Iterator,
    {}
}
mod ziptuple {
    use super::size_hint;
    /// See [`multizip`] for more information.
    #[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
    pub struct Zip<T> {
        t: T,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for Zip<T> {
        #[inline]
        fn clone(&self) -> Zip<T> {
            Zip {
                t: ::core::clone::Clone::clone(&self.t),
            }
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Zip<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(f, "Zip", "t", &&self.t)
        }
    }
    /// An iterator that generalizes `.zip()` and allows running multiple iterators in lockstep.
    ///
    /// The iterator `Zip<(I, J, ..., M)>` is formed from a tuple of iterators (or values that
    /// implement [`IntoIterator`]) and yields elements
    /// until any of the subiterators yields `None`.
    ///
    /// The iterator element type is a tuple like like `(A, B, ..., E)` where `A` to `E` are the
    /// element types of the subiterator.
    ///
    /// **Note:** The result of this function is a value of a named type (`Zip<(I, J,
    /// ..)>` of each component iterator `I, J, ...`) if each component iterator is
    /// nameable.
    ///
    /// Prefer [`izip!()`](crate::izip) over `multizip` for the performance benefits of using the
    /// standard library `.zip()`. Prefer `multizip` if a nameable type is needed.
    ///
    /// ```
    /// use itertools::multizip;
    ///
    /// // iterate over three sequences side-by-side
    /// let mut results = [0, 0, 0, 0];
    /// let inputs = [3, 7, 9, 6];
    ///
    /// for (r, index, input) in multizip((&mut results, 0..10, &inputs)) {
    ///     *r = index * 10 + input;
    /// }
    ///
    /// assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
    /// ```
    pub fn multizip<T, U>(t: U) -> Zip<T>
    where
        Zip<T>: From<U> + Iterator,
    {
        Zip::from(t)
    }
    #[allow(non_snake_case)]
    impl<A: IntoIterator> From<(A,)> for Zip<(A::IntoIter,)> {
        fn from(t: (A,)) -> Self {
            let (A,) = t;
            Zip { t: (A.into_iter(),) }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A> Iterator for Zip<(A,)>
    where
        A: Iterator,
    {
        type Item = (A::Item,);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A,) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A,))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A,) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A> ExactSizeIterator for Zip<(A,)>
    where
        A: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A> DoubleEndedIterator for Zip<(A,)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A,) = self.t;
            let size = *[A.len()].iter().min().unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            match (A.next_back(),) {
                (Some(A),) => Some((A,)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<A: IntoIterator, B: IntoIterator> From<(A, B)>
    for Zip<(A::IntoIter, B::IntoIter)> {
        fn from(t: (A, B)) -> Self {
            let (A, B) = t;
            Zip {
                t: (A.into_iter(), B.into_iter()),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B> Iterator for Zip<(A, B)>
    where
        A: Iterator,
        B: Iterator,
    {
        type Item = (A::Item, B::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B> ExactSizeIterator for Zip<(A, B)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B> DoubleEndedIterator for Zip<(A, B)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B) = self.t;
            let size = *[A.len(), B.len()].iter().min().unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            match (A.next_back(), B.next_back()) {
                (Some(A), Some(B)) => Some((A, B)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<A: IntoIterator, B: IntoIterator, C: IntoIterator> From<(A, B, C)>
    for Zip<(A::IntoIter, B::IntoIter, C::IntoIter)> {
        fn from(t: (A, B, C)) -> Self {
            let (A, B, C) = t;
            Zip {
                t: (A.into_iter(), B.into_iter(), C.into_iter()),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C> Iterator for Zip<(A, B, C)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
    {
        type Item = (A::Item, B::Item, C::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C> ExactSizeIterator for Zip<(A, B, C)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C> DoubleEndedIterator for Zip<(A, B, C)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C) = self.t;
            let size = *[A.len(), B.len(), C.len()].iter().min().unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            match (A.next_back(), B.next_back(), C.next_back()) {
                (Some(A), Some(B), Some(C)) => Some((A, B, C)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
    > From<(A, B, C, D)> for Zip<(A::IntoIter, B::IntoIter, C::IntoIter, D::IntoIter)> {
        fn from(t: (A, B, C, D)) -> Self {
            let (A, B, C, D) = t;
            Zip {
                t: (A.into_iter(), B.into_iter(), C.into_iter(), D.into_iter()),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D> Iterator for Zip<(A, B, C, D)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
    {
        type Item = (A::Item, B::Item, C::Item, D::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D> ExactSizeIterator for Zip<(A, B, C, D)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D> DoubleEndedIterator for Zip<(A, B, C, D)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D) = self.t;
            let size = *[A.len(), B.len(), C.len(), D.len()].iter().min().unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            match (A.next_back(), B.next_back(), C.next_back(), D.next_back()) {
                (Some(A), Some(B), Some(C), Some(D)) => Some((A, B, C, D)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
    > From<(A, B, C, D, E)>
    for Zip<(A::IntoIter, B::IntoIter, C::IntoIter, D::IntoIter, E::IntoIter)> {
        fn from(t: (A, B, C, D, E)) -> Self {
            let (A, B, C, D, E) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E> Iterator for Zip<(A, B, C, D, E)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
    {
        type Item = (A::Item, B::Item, C::Item, D::Item, E::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D, ref mut E) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E> ExactSizeIterator for Zip<(A, B, C, D, E)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E> DoubleEndedIterator for Zip<(A, B, C, D, E)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D, ref mut E) = self.t;
            let size = *[A.len(), B.len(), C.len(), D.len(), E.len()]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
            ) {
                (Some(A), Some(B), Some(C), Some(D), Some(E)) => Some((A, B, C, D, E)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
    > From<(A, B, C, D, E, F)>
    for Zip<
        (A::IntoIter, B::IntoIter, C::IntoIter, D::IntoIter, E::IntoIter, F::IntoIter),
    > {
        fn from(t: (A, B, C, D, E, F)) -> Self {
            let (A, B, C, D, E, F) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F> Iterator for Zip<(A, B, C, D, E, F)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
    {
        type Item = (A::Item, B::Item, C::Item, D::Item, E::Item, F::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D, ref mut E, ref mut F) = self
                .t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E, ref F) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F> ExactSizeIterator for Zip<(A, B, C, D, E, F)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F> DoubleEndedIterator for Zip<(A, B, C, D, E, F)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (ref mut A, ref mut B, ref mut C, ref mut D, ref mut E, ref mut F) = self
                .t;
            let size = *[A.len(), B.len(), C.len(), D.len(), E.len(), F.len()]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
            ) {
                (Some(A), Some(B), Some(C), Some(D), Some(E), Some(F)) => {
                    Some((A, B, C, D, E, F))
                }
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
    > From<(A, B, C, D, E, F, G)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G)) -> Self {
            let (A, B, C, D, E, F, G) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G> Iterator for Zip<(A, B, C, D, E, F, G)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
    {
        type Item = (A::Item, B::Item, C::Item, D::Item, E::Item, F::Item, G::Item);
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E, ref F, ref G) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G> ExactSizeIterator for Zip<(A, B, C, D, E, F, G)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G> DoubleEndedIterator for Zip<(A, B, C, D, E, F, G)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
            ) = self.t;
            let size = *[A.len(), B.len(), C.len(), D.len(), E.len(), F.len(), G.len()]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
            ) {
                (Some(A), Some(B), Some(C), Some(D), Some(E), Some(F), Some(G)) => {
                    Some((A, B, C, D, E, F, G))
                }
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
        H: IntoIterator,
    > From<(A, B, C, D, E, F, G, H)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
            H::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G, H)) -> Self {
            let (A, B, C, D, E, F, G, H) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                    H.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G, H> Iterator for Zip<(A, B, C, D, E, F, G, H)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
        H: Iterator,
    {
        type Item = (
            A::Item,
            B::Item,
            C::Item,
            D::Item,
            E::Item,
            F::Item,
            G::Item,
            H::Item,
        );
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            let H = match H.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G, H))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E, ref F, ref G, ref H) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            let sh = size_hint::min(H.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H> ExactSizeIterator for Zip<(A, B, C, D, E, F, G, H)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
        H: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H> DoubleEndedIterator for Zip<(A, B, C, D, E, F, G, H)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
        H: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
            ) = self.t;
            let size = *[
                A.len(),
                B.len(),
                C.len(),
                D.len(),
                E.len(),
                F.len(),
                G.len(),
                H.len(),
            ]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            if H.len() != size {
                for _ in 0..H.len() - size {
                    H.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
                H.next_back(),
            ) {
                (
                    Some(A),
                    Some(B),
                    Some(C),
                    Some(D),
                    Some(E),
                    Some(F),
                    Some(G),
                    Some(H),
                ) => Some((A, B, C, D, E, F, G, H)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
        H: IntoIterator,
        I: IntoIterator,
    > From<(A, B, C, D, E, F, G, H, I)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
            H::IntoIter,
            I::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G, H, I)) -> Self {
            let (A, B, C, D, E, F, G, H, I) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                    H.into_iter(),
                    I.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G, H, I> Iterator for Zip<(A, B, C, D, E, F, G, H, I)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
        H: Iterator,
        I: Iterator,
    {
        type Item = (
            A::Item,
            B::Item,
            C::Item,
            D::Item,
            E::Item,
            F::Item,
            G::Item,
            H::Item,
            I::Item,
        );
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            let H = match H.next() {
                None => return None,
                Some(elt) => elt,
            };
            let I = match I.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G, H, I))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E, ref F, ref G, ref H, ref I) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            let sh = size_hint::min(H.size_hint(), sh);
            let sh = size_hint::min(I.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I> ExactSizeIterator
    for Zip<(A, B, C, D, E, F, G, H, I)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
        H: ExactSizeIterator,
        I: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I> DoubleEndedIterator
    for Zip<(A, B, C, D, E, F, G, H, I)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
        H: DoubleEndedIterator + ExactSizeIterator,
        I: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
            ) = self.t;
            let size = *[
                A.len(),
                B.len(),
                C.len(),
                D.len(),
                E.len(),
                F.len(),
                G.len(),
                H.len(),
                I.len(),
            ]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            if H.len() != size {
                for _ in 0..H.len() - size {
                    H.next_back();
                }
            }
            if I.len() != size {
                for _ in 0..I.len() - size {
                    I.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
                H.next_back(),
                I.next_back(),
            ) {
                (
                    Some(A),
                    Some(B),
                    Some(C),
                    Some(D),
                    Some(E),
                    Some(F),
                    Some(G),
                    Some(H),
                    Some(I),
                ) => Some((A, B, C, D, E, F, G, H, I)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
        H: IntoIterator,
        I: IntoIterator,
        J: IntoIterator,
    > From<(A, B, C, D, E, F, G, H, I, J)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
            H::IntoIter,
            I::IntoIter,
            J::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G, H, I, J)) -> Self {
            let (A, B, C, D, E, F, G, H, I, J) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                    H.into_iter(),
                    I.into_iter(),
                    J.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G, H, I, J> Iterator for Zip<(A, B, C, D, E, F, G, H, I, J)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
        H: Iterator,
        I: Iterator,
        J: Iterator,
    {
        type Item = (
            A::Item,
            B::Item,
            C::Item,
            D::Item,
            E::Item,
            F::Item,
            G::Item,
            H::Item,
            I::Item,
            J::Item,
        );
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            let H = match H.next() {
                None => return None,
                Some(elt) => elt,
            };
            let I = match I.next() {
                None => return None,
                Some(elt) => elt,
            };
            let J = match J.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G, H, I, J))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (ref A, ref B, ref C, ref D, ref E, ref F, ref G, ref H, ref I, ref J) = self
                .t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            let sh = size_hint::min(H.size_hint(), sh);
            let sh = size_hint::min(I.size_hint(), sh);
            let sh = size_hint::min(J.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J> ExactSizeIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
        H: ExactSizeIterator,
        I: ExactSizeIterator,
        J: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J> DoubleEndedIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
        H: DoubleEndedIterator + ExactSizeIterator,
        I: DoubleEndedIterator + ExactSizeIterator,
        J: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
            ) = self.t;
            let size = *[
                A.len(),
                B.len(),
                C.len(),
                D.len(),
                E.len(),
                F.len(),
                G.len(),
                H.len(),
                I.len(),
                J.len(),
            ]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            if H.len() != size {
                for _ in 0..H.len() - size {
                    H.next_back();
                }
            }
            if I.len() != size {
                for _ in 0..I.len() - size {
                    I.next_back();
                }
            }
            if J.len() != size {
                for _ in 0..J.len() - size {
                    J.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
                H.next_back(),
                I.next_back(),
                J.next_back(),
            ) {
                (
                    Some(A),
                    Some(B),
                    Some(C),
                    Some(D),
                    Some(E),
                    Some(F),
                    Some(G),
                    Some(H),
                    Some(I),
                    Some(J),
                ) => Some((A, B, C, D, E, F, G, H, I, J)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
        H: IntoIterator,
        I: IntoIterator,
        J: IntoIterator,
        K: IntoIterator,
    > From<(A, B, C, D, E, F, G, H, I, J, K)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
            H::IntoIter,
            I::IntoIter,
            J::IntoIter,
            K::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G, H, I, J, K)) -> Self {
            let (A, B, C, D, E, F, G, H, I, J, K) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                    H.into_iter(),
                    I.into_iter(),
                    J.into_iter(),
                    K.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G, H, I, J, K> Iterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
        H: Iterator,
        I: Iterator,
        J: Iterator,
        K: Iterator,
    {
        type Item = (
            A::Item,
            B::Item,
            C::Item,
            D::Item,
            E::Item,
            F::Item,
            G::Item,
            H::Item,
            I::Item,
            J::Item,
            K::Item,
        );
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
                ref mut K,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            let H = match H.next() {
                None => return None,
                Some(elt) => elt,
            };
            let I = match I.next() {
                None => return None,
                Some(elt) => elt,
            };
            let J = match J.next() {
                None => return None,
                Some(elt) => elt,
            };
            let K = match K.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G, H, I, J, K))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (
                ref A,
                ref B,
                ref C,
                ref D,
                ref E,
                ref F,
                ref G,
                ref H,
                ref I,
                ref J,
                ref K,
            ) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            let sh = size_hint::min(H.size_hint(), sh);
            let sh = size_hint::min(I.size_hint(), sh);
            let sh = size_hint::min(J.size_hint(), sh);
            let sh = size_hint::min(K.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J, K> ExactSizeIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
        H: ExactSizeIterator,
        I: ExactSizeIterator,
        J: ExactSizeIterator,
        K: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J, K> DoubleEndedIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
        H: DoubleEndedIterator + ExactSizeIterator,
        I: DoubleEndedIterator + ExactSizeIterator,
        J: DoubleEndedIterator + ExactSizeIterator,
        K: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
                ref mut K,
            ) = self.t;
            let size = *[
                A.len(),
                B.len(),
                C.len(),
                D.len(),
                E.len(),
                F.len(),
                G.len(),
                H.len(),
                I.len(),
                J.len(),
                K.len(),
            ]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            if H.len() != size {
                for _ in 0..H.len() - size {
                    H.next_back();
                }
            }
            if I.len() != size {
                for _ in 0..I.len() - size {
                    I.next_back();
                }
            }
            if J.len() != size {
                for _ in 0..J.len() - size {
                    J.next_back();
                }
            }
            if K.len() != size {
                for _ in 0..K.len() - size {
                    K.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
                H.next_back(),
                I.next_back(),
                J.next_back(),
                K.next_back(),
            ) {
                (
                    Some(A),
                    Some(B),
                    Some(C),
                    Some(D),
                    Some(E),
                    Some(F),
                    Some(G),
                    Some(H),
                    Some(I),
                    Some(J),
                    Some(K),
                ) => Some((A, B, C, D, E, F, G, H, I, J, K)),
                _ => None,
            }
        }
    }
    #[allow(non_snake_case)]
    impl<
        A: IntoIterator,
        B: IntoIterator,
        C: IntoIterator,
        D: IntoIterator,
        E: IntoIterator,
        F: IntoIterator,
        G: IntoIterator,
        H: IntoIterator,
        I: IntoIterator,
        J: IntoIterator,
        K: IntoIterator,
        L: IntoIterator,
    > From<(A, B, C, D, E, F, G, H, I, J, K, L)>
    for Zip<
        (
            A::IntoIter,
            B::IntoIter,
            C::IntoIter,
            D::IntoIter,
            E::IntoIter,
            F::IntoIter,
            G::IntoIter,
            H::IntoIter,
            I::IntoIter,
            J::IntoIter,
            K::IntoIter,
            L::IntoIter,
        ),
    > {
        fn from(t: (A, B, C, D, E, F, G, H, I, J, K, L)) -> Self {
            let (A, B, C, D, E, F, G, H, I, J, K, L) = t;
            Zip {
                t: (
                    A.into_iter(),
                    B.into_iter(),
                    C.into_iter(),
                    D.into_iter(),
                    E.into_iter(),
                    F.into_iter(),
                    G.into_iter(),
                    H.into_iter(),
                    I.into_iter(),
                    J.into_iter(),
                    K.into_iter(),
                    L.into_iter(),
                ),
            }
        }
    }
    #[allow(non_snake_case)]
    #[allow(unused_assignments)]
    impl<A, B, C, D, E, F, G, H, I, J, K, L> Iterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K, L)>
    where
        A: Iterator,
        B: Iterator,
        C: Iterator,
        D: Iterator,
        E: Iterator,
        F: Iterator,
        G: Iterator,
        H: Iterator,
        I: Iterator,
        J: Iterator,
        K: Iterator,
        L: Iterator,
    {
        type Item = (
            A::Item,
            B::Item,
            C::Item,
            D::Item,
            E::Item,
            F::Item,
            G::Item,
            H::Item,
            I::Item,
            J::Item,
            K::Item,
            L::Item,
        );
        fn next(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
                ref mut K,
                ref mut L,
            ) = self.t;
            let A = match A.next() {
                None => return None,
                Some(elt) => elt,
            };
            let B = match B.next() {
                None => return None,
                Some(elt) => elt,
            };
            let C = match C.next() {
                None => return None,
                Some(elt) => elt,
            };
            let D = match D.next() {
                None => return None,
                Some(elt) => elt,
            };
            let E = match E.next() {
                None => return None,
                Some(elt) => elt,
            };
            let F = match F.next() {
                None => return None,
                Some(elt) => elt,
            };
            let G = match G.next() {
                None => return None,
                Some(elt) => elt,
            };
            let H = match H.next() {
                None => return None,
                Some(elt) => elt,
            };
            let I = match I.next() {
                None => return None,
                Some(elt) => elt,
            };
            let J = match J.next() {
                None => return None,
                Some(elt) => elt,
            };
            let K = match K.next() {
                None => return None,
                Some(elt) => elt,
            };
            let L = match L.next() {
                None => return None,
                Some(elt) => elt,
            };
            Some((A, B, C, D, E, F, G, H, I, J, K, L))
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            let sh = (usize::MAX, None);
            let (
                ref A,
                ref B,
                ref C,
                ref D,
                ref E,
                ref F,
                ref G,
                ref H,
                ref I,
                ref J,
                ref K,
                ref L,
            ) = self.t;
            let sh = size_hint::min(A.size_hint(), sh);
            let sh = size_hint::min(B.size_hint(), sh);
            let sh = size_hint::min(C.size_hint(), sh);
            let sh = size_hint::min(D.size_hint(), sh);
            let sh = size_hint::min(E.size_hint(), sh);
            let sh = size_hint::min(F.size_hint(), sh);
            let sh = size_hint::min(G.size_hint(), sh);
            let sh = size_hint::min(H.size_hint(), sh);
            let sh = size_hint::min(I.size_hint(), sh);
            let sh = size_hint::min(J.size_hint(), sh);
            let sh = size_hint::min(K.size_hint(), sh);
            let sh = size_hint::min(L.size_hint(), sh);
            sh
        }
    }
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J, K, L> ExactSizeIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K, L)>
    where
        A: ExactSizeIterator,
        B: ExactSizeIterator,
        C: ExactSizeIterator,
        D: ExactSizeIterator,
        E: ExactSizeIterator,
        F: ExactSizeIterator,
        G: ExactSizeIterator,
        H: ExactSizeIterator,
        I: ExactSizeIterator,
        J: ExactSizeIterator,
        K: ExactSizeIterator,
        L: ExactSizeIterator,
    {}
    #[allow(non_snake_case)]
    impl<A, B, C, D, E, F, G, H, I, J, K, L> DoubleEndedIterator
    for Zip<(A, B, C, D, E, F, G, H, I, J, K, L)>
    where
        A: DoubleEndedIterator + ExactSizeIterator,
        B: DoubleEndedIterator + ExactSizeIterator,
        C: DoubleEndedIterator + ExactSizeIterator,
        D: DoubleEndedIterator + ExactSizeIterator,
        E: DoubleEndedIterator + ExactSizeIterator,
        F: DoubleEndedIterator + ExactSizeIterator,
        G: DoubleEndedIterator + ExactSizeIterator,
        H: DoubleEndedIterator + ExactSizeIterator,
        I: DoubleEndedIterator + ExactSizeIterator,
        J: DoubleEndedIterator + ExactSizeIterator,
        K: DoubleEndedIterator + ExactSizeIterator,
        L: DoubleEndedIterator + ExactSizeIterator,
    {
        #[inline]
        fn next_back(&mut self) -> Option<Self::Item> {
            let (
                ref mut A,
                ref mut B,
                ref mut C,
                ref mut D,
                ref mut E,
                ref mut F,
                ref mut G,
                ref mut H,
                ref mut I,
                ref mut J,
                ref mut K,
                ref mut L,
            ) = self.t;
            let size = *[
                A.len(),
                B.len(),
                C.len(),
                D.len(),
                E.len(),
                F.len(),
                G.len(),
                H.len(),
                I.len(),
                J.len(),
                K.len(),
                L.len(),
            ]
                .iter()
                .min()
                .unwrap();
            if A.len() != size {
                for _ in 0..A.len() - size {
                    A.next_back();
                }
            }
            if B.len() != size {
                for _ in 0..B.len() - size {
                    B.next_back();
                }
            }
            if C.len() != size {
                for _ in 0..C.len() - size {
                    C.next_back();
                }
            }
            if D.len() != size {
                for _ in 0..D.len() - size {
                    D.next_back();
                }
            }
            if E.len() != size {
                for _ in 0..E.len() - size {
                    E.next_back();
                }
            }
            if F.len() != size {
                for _ in 0..F.len() - size {
                    F.next_back();
                }
            }
            if G.len() != size {
                for _ in 0..G.len() - size {
                    G.next_back();
                }
            }
            if H.len() != size {
                for _ in 0..H.len() - size {
                    H.next_back();
                }
            }
            if I.len() != size {
                for _ in 0..I.len() - size {
                    I.next_back();
                }
            }
            if J.len() != size {
                for _ in 0..J.len() - size {
                    J.next_back();
                }
            }
            if K.len() != size {
                for _ in 0..K.len() - size {
                    K.next_back();
                }
            }
            if L.len() != size {
                for _ in 0..L.len() - size {
                    L.next_back();
                }
            }
            match (
                A.next_back(),
                B.next_back(),
                C.next_back(),
                D.next_back(),
                E.next_back(),
                F.next_back(),
                G.next_back(),
                H.next_back(),
                I.next_back(),
                J.next_back(),
                K.next_back(),
                L.next_back(),
            ) {
                (
                    Some(A),
                    Some(B),
                    Some(C),
                    Some(D),
                    Some(E),
                    Some(F),
                    Some(G),
                    Some(H),
                    Some(I),
                    Some(J),
                    Some(K),
                    Some(L),
                ) => Some((A, B, C, D, E, F, G, H, I, J, K, L)),
                _ => None,
            }
        }
    }
}
/// An [`Iterator`] blanket implementation that provides extra adaptors and
/// methods.
///
/// This trait defines a number of methods. They are divided into two groups:
///
/// * *Adaptors* take an iterator and parameter as input, and return
///   a new iterator value. These are listed first in the trait. An example
///   of an adaptor is [`.interleave()`](Itertools::interleave)
///
/// * *Regular methods* are those that don't return iterators and instead
///   return a regular value of some other kind.
///   [`.next_tuple()`](Itertools::next_tuple) is an example and the first regular
///   method in the list.
pub trait Itertools: Iterator {
    /// Alternate elements from two iterators until both have run out.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (1..7).interleave(vec![-1, -2]);
    /// itertools::assert_equal(it, vec![1, -1, 2, -2, 3, 4, 5, 6]);
    /// ```
    fn interleave<J>(self, other: J) -> Interleave<Self, J::IntoIter>
    where
        J: IntoIterator<Item = Self::Item>,
        Self: Sized,
    {
        interleave(self, other)
    }
    /// Alternate elements from two iterators until at least one of them has run
    /// out.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (1..7).interleave_shortest(vec![-1, -2]);
    /// itertools::assert_equal(it, vec![1, -1, 2, -2, 3]);
    /// ```
    fn interleave_shortest<J>(self, other: J) -> InterleaveShortest<Self, J::IntoIter>
    where
        J: IntoIterator<Item = Self::Item>,
        Self: Sized,
    {
        adaptors::interleave_shortest(self, other.into_iter())
    }
    /// An iterator adaptor to insert a particular value
    /// between each element of the adapted iterator.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// itertools::assert_equal((0..3).intersperse(8), vec![0, 8, 1, 8, 2]);
    /// ```
    fn intersperse(self, element: Self::Item) -> Intersperse<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        intersperse::intersperse(self, element)
    }
    /// An iterator adaptor to insert a particular value created by a function
    /// between each element of the adapted iterator.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut i = 10;
    /// itertools::assert_equal((0..3).intersperse_with(|| { i -= 1; i }), vec![0, 9, 1, 8, 2]);
    /// assert_eq!(i, 8);
    /// ```
    fn intersperse_with<F>(self, element: F) -> IntersperseWith<Self, F>
    where
        Self: Sized,
        F: FnMut() -> Self::Item,
    {
        intersperse::intersperse_with(self, element)
    }
    /// Returns an iterator over a subsection of the iterator.
    ///
    /// Works similarly to [`slice::get`](https://doc.rust-lang.org/std/primitive.slice.html#method.get).
    ///
    /// **Panics** for ranges `..=usize::MAX` and `0..=usize::MAX`.
    ///
    /// It's a generalisation of [`Iterator::take`] and [`Iterator::skip`],
    /// and uses these under the hood.
    /// Therefore, the resulting iterator is:
    /// - [`ExactSizeIterator`] if the adapted iterator is [`ExactSizeIterator`].
    /// - [`DoubleEndedIterator`] if the adapted iterator is [`DoubleEndedIterator`] and [`ExactSizeIterator`].
    ///
    /// # Unspecified Behavior
    /// The result of indexing with an exhausted [`core::ops::RangeInclusive`] is unspecified.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let vec = vec![3, 1, 4, 1, 5];
    ///
    /// let mut range: Vec<_> =
    ///         vec.iter().get(1..=3).copied().collect();
    /// assert_eq!(&range, &[1, 4, 1]);
    ///
    /// // It works with other types of ranges, too
    /// range = vec.iter().get(..2).copied().collect();
    /// assert_eq!(&range, &[3, 1]);
    ///
    /// range = vec.iter().get(0..1).copied().collect();
    /// assert_eq!(&range, &[3]);
    ///
    /// range = vec.iter().get(2..).copied().collect();
    /// assert_eq!(&range, &[4, 1, 5]);
    ///
    /// range = vec.iter().get(..=2).copied().collect();
    /// assert_eq!(&range, &[3, 1, 4]);
    ///
    /// range = vec.iter().get(..).copied().collect();
    /// assert_eq!(range, vec);
    /// ```
    fn get<R>(self, index: R) -> R::Output
    where
        Self: Sized,
        R: traits::IteratorIndex<Self>,
    {
        iter_index::get(self, index)
    }
    /// Create an iterator which iterates over both this and the specified
    /// iterator simultaneously, yielding pairs of two optional elements.
    ///
    /// This iterator is *fused*.
    ///
    /// As long as neither input iterator is exhausted yet, it yields two values
    /// via `EitherOrBoth::Both`.
    ///
    /// When the parameter iterator is exhausted, it only yields a value from the
    /// `self` iterator via `EitherOrBoth::Left`.
    ///
    /// When the `self` iterator is exhausted, it only yields a value from the
    /// parameter iterator via `EitherOrBoth::Right`.
    ///
    /// When both iterators return `None`, all further invocations of `.next()`
    /// will return `None`.
    ///
    /// Iterator element type is
    /// [`EitherOrBoth<Self::Item, J::Item>`](EitherOrBoth).
    ///
    /// ```rust
    /// use itertools::EitherOrBoth::{Both, Right};
    /// use itertools::Itertools;
    /// let it = (0..1).zip_longest(1..3);
    /// itertools::assert_equal(it, vec![Both(0, 1), Right(2)]);
    /// ```
    #[inline]
    fn zip_longest<J>(self, other: J) -> ZipLongest<Self, J::IntoIter>
    where
        J: IntoIterator,
        Self: Sized,
    {
        zip_longest::zip_longest(self, other.into_iter())
    }
    /// Create an iterator which iterates over both this and the specified
    /// iterator simultaneously, yielding pairs of elements.
    ///
    /// **Panics** if the iterators reach an end and they are not of equal
    /// lengths.
    #[inline]
    fn zip_eq<J>(self, other: J) -> ZipEq<Self, J::IntoIter>
    where
        J: IntoIterator,
        Self: Sized,
    {
        zip_eq(self, other)
    }
    /// A “meta iterator adaptor”. Its closure receives a reference to the
    /// iterator and may pick off as many elements as it likes, to produce the
    /// next iterator element.
    ///
    /// Iterator element type is `B`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // An adaptor that gathers elements in pairs
    /// let pit = (0..4).batching(|it| {
    ///            match it.next() {
    ///                None => None,
    ///                Some(x) => match it.next() {
    ///                    None => None,
    ///                    Some(y) => Some((x, y)),
    ///                }
    ///            }
    ///        });
    ///
    /// itertools::assert_equal(pit, vec![(0, 1), (2, 3)]);
    /// ```
    ///
    fn batching<B, F>(self, f: F) -> Batching<Self, F>
    where
        F: FnMut(&mut Self) -> Option<B>,
        Self: Sized,
    {
        adaptors::batching(self, f)
    }
    /// Return an *iterable* that can group iterator elements.
    /// Consecutive elements that map to the same key (“runs”), are assigned
    /// to the same group.
    ///
    /// `ChunkBy` is the storage for the lazy grouping operation.
    ///
    /// If the groups are consumed in order, or if each group's iterator is
    /// dropped without keeping it around, then `ChunkBy` uses no
    /// allocations.  It needs allocations only if several group iterators
    /// are alive at the same time.
    ///
    /// This type implements [`IntoIterator`] (it is **not** an iterator
    /// itself), because the group iterators need to borrow from this
    /// value. It should be stored in a local variable or temporary and
    /// iterated.
    ///
    /// Iterator element type is `(K, Group)`: the group's key and the
    /// group iterator.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // chunk data into runs of larger than zero or not.
    /// let data = vec![1, 3, -2, -2, 1, 0, 1, 2];
    /// // chunks:     |---->|------>|--------->|
    ///
    /// // Note: The `&` is significant here, `ChunkBy` is iterable
    /// // only by reference. You can also call `.into_iter()` explicitly.
    /// let mut data_grouped = Vec::new();
    /// for (key, chunk) in &data.into_iter().chunk_by(|elt| *elt >= 0) {
    ///     data_grouped.push((key, chunk.collect()));
    /// }
    /// assert_eq!(data_grouped, vec![(true, vec![1, 3]), (false, vec![-2, -2]), (true, vec![1, 0, 1, 2])]);
    /// ```
    fn chunk_by<K, F>(self, key: F) -> ChunkBy<K, Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: PartialEq,
    {
        groupbylazy::new(self, key)
    }
    /// See [`.chunk_by()`](Itertools::chunk_by).
    #[deprecated(note = "Use .chunk_by() instead", since = "0.13.0")]
    fn group_by<K, F>(self, key: F) -> ChunkBy<K, Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: PartialEq,
    {
        self.chunk_by(key)
    }
    /// Return an *iterable* that can chunk the iterator.
    ///
    /// Yield subiterators (chunks) that each yield a fixed number elements,
    /// determined by `size`. The last chunk will be shorter if there aren't
    /// enough elements.
    ///
    /// `IntoChunks` is based on `ChunkBy`: it is iterable (implements
    /// `IntoIterator`, **not** `Iterator`), and it only buffers if several
    /// chunk iterators are alive at the same time.
    ///
    /// Iterator element type is `Chunk`, each chunk's iterator.
    ///
    /// **Panics** if `size` is 0.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1, 1, 2, -2, 6, 0, 3, 1];
    /// //chunk size=3 |------->|-------->|--->|
    ///
    /// // Note: The `&` is significant here, `IntoChunks` is iterable
    /// // only by reference. You can also call `.into_iter()` explicitly.
    /// for chunk in &data.into_iter().chunks(3) {
    ///     // Check that the sum of each chunk is 4.
    ///     assert_eq!(4, chunk.sum());
    /// }
    /// ```
    fn chunks(self, size: usize) -> IntoChunks<Self>
    where
        Self: Sized,
    {
        if !(size != 0) {
            ::core::panicking::panic("assertion failed: size != 0")
        }
        groupbylazy::new_chunks(self, size)
    }
    /// Return an iterator over all contiguous windows producing tuples of
    /// a specific size (up to 12).
    ///
    /// `tuple_windows` clones the iterator elements so that they can be
    /// part of successive windows, this makes it most suited for iterators
    /// of references and other values that are cheap to copy.
    ///
    /// ```
    /// use itertools::Itertools;
    /// let mut v = Vec::new();
    ///
    /// // pairwise iteration
    /// for (a, b) in (1..5).tuple_windows() {
    ///     v.push((a, b));
    /// }
    /// assert_eq!(v, vec![(1, 2), (2, 3), (3, 4)]);
    ///
    /// let mut it = (1..5).tuple_windows();
    /// assert_eq!(Some((1, 2, 3)), it.next());
    /// assert_eq!(Some((2, 3, 4)), it.next());
    /// assert_eq!(None, it.next());
    ///
    /// // this requires a type hint
    /// let it = (1..5).tuple_windows::<(_, _, _)>();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (2, 3, 4)]);
    ///
    /// // you can also specify the complete type
    /// use itertools::TupleWindows;
    /// use std::ops::Range;
    ///
    /// let it: TupleWindows<Range<u32>, (u32, u32, u32)> = (1..5).tuple_windows();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (2, 3, 4)]);
    /// ```
    fn tuple_windows<T>(self) -> TupleWindows<Self, T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: traits::HomogeneousTuple,
        T::Item: Clone,
    {
        tuple_impl::tuple_windows(self)
    }
    /// Return an iterator over all windows, wrapping back to the first
    /// elements when the window would otherwise exceed the length of the
    /// iterator, producing tuples of a specific size (up to 12).
    ///
    /// `circular_tuple_windows` clones the iterator elements so that they can be
    /// part of successive windows, this makes it most suited for iterators
    /// of references and other values that are cheap to copy.
    ///
    /// ```
    /// use itertools::Itertools;
    /// let mut v = Vec::new();
    /// for (a, b) in (1..5).circular_tuple_windows() {
    ///     v.push((a, b));
    /// }
    /// assert_eq!(v, vec![(1, 2), (2, 3), (3, 4), (4, 1)]);
    ///
    /// let mut it = (1..5).circular_tuple_windows();
    /// assert_eq!(Some((1, 2, 3)), it.next());
    /// assert_eq!(Some((2, 3, 4)), it.next());
    /// assert_eq!(Some((3, 4, 1)), it.next());
    /// assert_eq!(Some((4, 1, 2)), it.next());
    /// assert_eq!(None, it.next());
    ///
    /// // this requires a type hint
    /// let it = (1..5).circular_tuple_windows::<(_, _, _)>();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (2, 3, 4), (3, 4, 1), (4, 1, 2)]);
    /// ```
    fn circular_tuple_windows<T>(self) -> CircularTupleWindows<Self, T>
    where
        Self: Sized + Clone + Iterator<Item = T::Item> + ExactSizeIterator,
        T: tuple_impl::TupleCollect + Clone,
        T::Item: Clone,
    {
        tuple_impl::circular_tuple_windows(self)
    }
    /// Return an iterator that groups the items in tuples of a specific size
    /// (up to 12).
    ///
    /// See also the method [`.next_tuple()`](Itertools::next_tuple).
    ///
    /// ```
    /// use itertools::Itertools;
    /// let mut v = Vec::new();
    /// for (a, b) in (1..5).tuples() {
    ///     v.push((a, b));
    /// }
    /// assert_eq!(v, vec![(1, 2), (3, 4)]);
    ///
    /// let mut it = (1..7).tuples();
    /// assert_eq!(Some((1, 2, 3)), it.next());
    /// assert_eq!(Some((4, 5, 6)), it.next());
    /// assert_eq!(None, it.next());
    ///
    /// // this requires a type hint
    /// let it = (1..7).tuples::<(_, _, _)>();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (4, 5, 6)]);
    ///
    /// // you can also specify the complete type
    /// use itertools::Tuples;
    /// use std::ops::Range;
    ///
    /// let it: Tuples<Range<u32>, (u32, u32, u32)> = (1..7).tuples();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (4, 5, 6)]);
    /// ```
    ///
    /// See also [`Tuples::into_buffer`].
    fn tuples<T>(self) -> Tuples<Self, T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: traits::HomogeneousTuple,
    {
        tuple_impl::tuples(self)
    }
    /// Split into an iterator pair that both yield all elements from
    /// the original iterator.
    ///
    /// **Note:** If the iterator is clonable, prefer using that instead
    /// of using this method. Cloning is likely to be more efficient.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    /// let xs = vec![0, 1, 2, 3];
    ///
    /// let (mut t1, t2) = xs.into_iter().tee();
    /// itertools::assert_equal(t1.next(), Some(0));
    /// itertools::assert_equal(t2, 0..4);
    /// itertools::assert_equal(t1, 1..4);
    /// ```
    fn tee(self) -> (Tee<Self>, Tee<Self>)
    where
        Self: Sized,
        Self::Item: Clone,
    {
        tee::new(self)
    }
    /// Convert each item of the iterator using the [`Into`] trait.
    ///
    /// ```rust
    /// use itertools::Itertools;
    ///
    /// (1i32..42i32).map_into::<f64>().collect_vec();
    /// ```
    fn map_into<R>(self) -> MapInto<Self, R>
    where
        Self: Sized,
        Self::Item: Into<R>,
    {
        adaptors::map_into(self)
    }
    /// Return an iterator adaptor that applies the provided closure
    /// to every `Result::Ok` value. `Result::Err` values are
    /// unchanged.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let input = vec![Ok(41), Err(false), Ok(11)];
    /// let it = input.into_iter().map_ok(|i| i + 1);
    /// itertools::assert_equal(it, vec![Ok(42), Err(false), Ok(12)]);
    /// ```
    fn map_ok<F, T, U, E>(self, f: F) -> MapOk<Self, F>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnMut(T) -> U,
    {
        adaptors::map_ok(self, f)
    }
    /// Return an iterator adaptor that filters every `Result::Ok`
    /// value with the provided closure. `Result::Err` values are
    /// unchanged.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let input = vec![Ok(22), Err(false), Ok(11)];
    /// let it = input.into_iter().filter_ok(|&i| i > 20);
    /// itertools::assert_equal(it, vec![Ok(22), Err(false)]);
    /// ```
    fn filter_ok<F, T, E>(self, f: F) -> FilterOk<Self, F>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnMut(&T) -> bool,
    {
        adaptors::filter_ok(self, f)
    }
    /// Return an iterator adaptor that filters and transforms every
    /// `Result::Ok` value with the provided closure. `Result::Err`
    /// values are unchanged.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let input = vec![Ok(22), Err(false), Ok(11)];
    /// let it = input.into_iter().filter_map_ok(|i| if i > 20 { Some(i * 2) } else { None });
    /// itertools::assert_equal(it, vec![Ok(44), Err(false)]);
    /// ```
    fn filter_map_ok<F, T, U, E>(self, f: F) -> FilterMapOk<Self, F>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnMut(T) -> Option<U>,
    {
        adaptors::filter_map_ok(self, f)
    }
    /// Return an iterator adaptor that flattens every `Result::Ok` value into
    /// a series of `Result::Ok` values. `Result::Err` values are unchanged.
    ///
    /// This is useful when you have some common error type for your crate and
    /// need to propagate it upwards, but the `Result::Ok` case needs to be flattened.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let input = vec![Ok(0..2), Err(false), Ok(2..4)];
    /// let it = input.iter().cloned().flatten_ok();
    /// itertools::assert_equal(it.clone(), vec![Ok(0), Ok(1), Err(false), Ok(2), Ok(3)]);
    ///
    /// // This can also be used to propagate errors when collecting.
    /// let output_result: Result<Vec<i32>, bool> = it.collect();
    /// assert_eq!(output_result, Err(false));
    /// ```
    fn flatten_ok<T, E>(self) -> FlattenOk<Self, T, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        T: IntoIterator,
    {
        flatten_ok::flatten_ok(self)
    }
    /// “Lift” a function of the values of the current iterator so as to process
    /// an iterator of `Result` values instead.
    ///
    /// `processor` is a closure that receives an adapted version of the iterator
    /// as the only argument — the adapted iterator produces elements of type `T`,
    /// as long as the original iterator produces `Ok` values.
    ///
    /// If the original iterable produces an error at any point, the adapted
    /// iterator ends and it will return the error iself.
    ///
    /// Otherwise, the return value from the closure is returned wrapped
    /// inside `Ok`.
    ///
    /// # Example
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// type Item = Result<i32, &'static str>;
    ///
    /// let first_values: Vec<Item> = vec![Ok(1), Ok(0), Ok(3)];
    /// let second_values: Vec<Item> = vec![Ok(2), Ok(1), Err("overflow")];
    ///
    /// // “Lift” the iterator .max() method to work on the Ok-values.
    /// let first_max = first_values.into_iter().process_results(|iter| iter.max().unwrap_or(0));
    /// let second_max = second_values.into_iter().process_results(|iter| iter.max().unwrap_or(0));
    ///
    /// assert_eq!(first_max, Ok(3));
    /// assert!(second_max.is_err());
    /// ```
    fn process_results<F, T, E, R>(self, processor: F) -> Result<R, E>
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        F: FnOnce(ProcessResults<Self, E>) -> R,
    {
        process_results(self, processor)
    }
    /// Return an iterator adaptor that merges the two base iterators in
    /// ascending order.  If both base iterators are sorted (ascending), the
    /// result is sorted.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a = (0..11).step_by(3);
    /// let b = (0..11).step_by(5);
    /// let it = a.merge(b);
    /// itertools::assert_equal(it, vec![0, 0, 3, 5, 6, 9, 10]);
    /// ```
    fn merge<J>(self, other: J) -> Merge<Self, J::IntoIter>
    where
        Self: Sized,
        Self::Item: PartialOrd,
        J: IntoIterator<Item = Self::Item>,
    {
        merge(self, other)
    }
    /// Return an iterator adaptor that merges the two base iterators in order.
    /// This is much like [`.merge()`](Itertools::merge) but allows for a custom ordering.
    ///
    /// This can be especially useful for sequences of tuples.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a = (0..).zip("bc".chars());
    /// let b = (0..).zip("ad".chars());
    /// let it = a.merge_by(b, |x, y| x.1 <= y.1);
    /// itertools::assert_equal(it, vec![(0, 'a'), (0, 'b'), (1, 'c'), (1, 'd')]);
    /// ```
    fn merge_by<J, F>(self, other: J, is_first: F) -> MergeBy<Self, J::IntoIter, F>
    where
        Self: Sized,
        J: IntoIterator<Item = Self::Item>,
        F: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        merge_join::merge_by_new(self, other, is_first)
    }
    /// Create an iterator that merges items from both this and the specified
    /// iterator in ascending order.
    ///
    /// The function can either return an `Ordering` variant or a boolean.
    ///
    /// If `cmp_fn` returns `Ordering`,
    /// it chooses whether to pair elements based on the `Ordering` returned by the
    /// specified compare function. At any point, inspecting the tip of the
    /// iterators `I` and `J` as items `i` of type `I::Item` and `j` of type
    /// `J::Item` respectively, the resulting iterator will:
    ///
    /// - Emit `EitherOrBoth::Left(i)` when `i < j`,
    ///   and remove `i` from its source iterator
    /// - Emit `EitherOrBoth::Right(j)` when `i > j`,
    ///   and remove `j` from its source iterator
    /// - Emit `EitherOrBoth::Both(i, j)` when  `i == j`,
    ///   and remove both `i` and `j` from their respective source iterators
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::EitherOrBoth::{Left, Right, Both};
    ///
    /// let a = vec![0, 2, 4, 6, 1].into_iter();
    /// let b = (0..10).step_by(3);
    ///
    /// itertools::assert_equal(
    ///     // This performs a diff in the style of the Unix command comm(1),
    ///     // generalized to arbitrary types rather than text.
    ///     a.merge_join_by(b, Ord::cmp),
    ///     vec![Both(0, 0), Left(2), Right(3), Left(4), Both(6, 6), Left(1), Right(9)]
    /// );
    /// ```
    ///
    /// If `cmp_fn` returns `bool`,
    /// it chooses whether to pair elements based on the boolean returned by the
    /// specified function. At any point, inspecting the tip of the
    /// iterators `I` and `J` as items `i` of type `I::Item` and `j` of type
    /// `J::Item` respectively, the resulting iterator will:
    ///
    /// - Emit `Either::Left(i)` when `true`,
    ///   and remove `i` from its source iterator
    /// - Emit `Either::Right(j)` when `false`,
    ///   and remove `j` from its source iterator
    ///
    /// It is similar to the `Ordering` case if the first argument is considered
    /// "less" than the second argument.
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::Either::{Left, Right};
    ///
    /// let a = vec![0, 2, 4, 6, 1].into_iter();
    /// let b = (0..10).step_by(3);
    ///
    /// itertools::assert_equal(
    ///     a.merge_join_by(b, |i, j| i <= j),
    ///     vec![Left(0), Right(0), Left(2), Right(3), Left(4), Left(6), Left(1), Right(6), Right(9)]
    /// );
    /// ```
    #[inline]
    #[doc(alias = "comm")]
    fn merge_join_by<J, F, T>(
        self,
        other: J,
        cmp_fn: F,
    ) -> MergeJoinBy<Self, J::IntoIter, F>
    where
        J: IntoIterator,
        F: FnMut(&Self::Item, &J::Item) -> T,
        Self: Sized,
    {
        merge_join_by(self, other, cmp_fn)
    }
    /// Return an iterator adaptor that flattens an iterator of iterators by
    /// merging them in ascending order.
    ///
    /// If all base iterators are sorted (ascending), the result is sorted.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a = (0..6).step_by(3);
    /// let b = (1..6).step_by(3);
    /// let c = (2..6).step_by(3);
    /// let it = vec![a, b, c].into_iter().kmerge();
    /// itertools::assert_equal(it, vec![0, 1, 2, 3, 4, 5]);
    /// ```
    fn kmerge(self) -> KMerge<<Self::Item as IntoIterator>::IntoIter>
    where
        Self: Sized,
        Self::Item: IntoIterator,
        <Self::Item as IntoIterator>::Item: PartialOrd,
    {
        kmerge(self)
    }
    /// Return an iterator adaptor that flattens an iterator of iterators by
    /// merging them according to the given closure.
    ///
    /// The closure `first` is called with two elements *a*, *b* and should
    /// return `true` if *a* is ordered before *b*.
    ///
    /// If all base iterators are sorted according to `first`, the result is
    /// sorted.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a = vec![-1f64, 2., 3., -5., 6., -7.];
    /// let b = vec![0., 2., -4.];
    /// let mut it = vec![a, b].into_iter().kmerge_by(|a, b| a.abs() < b.abs());
    /// assert_eq!(it.next(), Some(0.));
    /// assert_eq!(it.last(), Some(-7.));
    /// ```
    fn kmerge_by<F>(
        self,
        first: F,
    ) -> KMergeBy<<Self::Item as IntoIterator>::IntoIter, F>
    where
        Self: Sized,
        Self::Item: IntoIterator,
        F: FnMut(
            &<Self::Item as IntoIterator>::Item,
            &<Self::Item as IntoIterator>::Item,
        ) -> bool,
    {
        kmerge_by(self, first)
    }
    /// Return an iterator adaptor that iterates over the cartesian product of
    /// the element sets of two iterators `self` and `J`.
    ///
    /// Iterator element type is `(Self::Item, J::Item)`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (0..2).cartesian_product("αβ".chars());
    /// itertools::assert_equal(it, vec![(0, 'α'), (0, 'β'), (1, 'α'), (1, 'β')]);
    /// ```
    fn cartesian_product<J>(self, other: J) -> Product<Self, J::IntoIter>
    where
        Self: Sized,
        Self::Item: Clone,
        J: IntoIterator,
        J::IntoIter: Clone,
    {
        adaptors::cartesian_product(self, other.into_iter())
    }
    /// Return an iterator adaptor that iterates over the cartesian product of
    /// all subiterators returned by meta-iterator `self`.
    ///
    /// All provided iterators must yield the same `Item` type. To generate
    /// the product of iterators yielding multiple types, use the
    /// [`iproduct`] macro instead.
    ///
    /// The iterator element type is `Vec<T>`, where `T` is the iterator element
    /// of the subiterators.
    ///
    /// Note that the iterator is fused.
    ///
    /// ```
    /// use itertools::Itertools;
    /// let mut multi_prod = (0..3).map(|i| (i * 2)..(i * 2 + 2))
    ///     .multi_cartesian_product();
    /// assert_eq!(multi_prod.next(), Some(vec![0, 2, 4]));
    /// assert_eq!(multi_prod.next(), Some(vec![0, 2, 5]));
    /// assert_eq!(multi_prod.next(), Some(vec![0, 3, 4]));
    /// assert_eq!(multi_prod.next(), Some(vec![0, 3, 5]));
    /// assert_eq!(multi_prod.next(), Some(vec![1, 2, 4]));
    /// assert_eq!(multi_prod.next(), Some(vec![1, 2, 5]));
    /// assert_eq!(multi_prod.next(), Some(vec![1, 3, 4]));
    /// assert_eq!(multi_prod.next(), Some(vec![1, 3, 5]));
    /// assert_eq!(multi_prod.next(), None);
    /// ```
    ///
    /// If the adapted iterator is empty, the result is an iterator yielding a single empty vector.
    /// This is known as the [nullary cartesian product](https://en.wikipedia.org/wiki/Empty_product#Nullary_Cartesian_product).
    ///
    /// ```
    /// use itertools::Itertools;
    /// let mut nullary_cartesian_product = (0..0).map(|i| (i * 2)..(i * 2 + 2)).multi_cartesian_product();
    /// assert_eq!(nullary_cartesian_product.next(), Some(vec![]));
    /// assert_eq!(nullary_cartesian_product.next(), None);
    /// ```
    fn multi_cartesian_product(
        self,
    ) -> MultiProduct<<Self::Item as IntoIterator>::IntoIter>
    where
        Self: Sized,
        Self::Item: IntoIterator,
        <Self::Item as IntoIterator>::IntoIter: Clone,
        <Self::Item as IntoIterator>::Item: Clone,
    {
        adaptors::multi_cartesian_product(self)
    }
    /// Return an iterator adaptor that uses the passed-in closure to
    /// optionally merge together consecutive elements.
    ///
    /// The closure `f` is passed two elements, `previous` and `current` and may
    /// return either (1) `Ok(combined)` to merge the two values or
    /// (2) `Err((previous', current'))` to indicate they can't be merged.
    /// In (2), the value `previous'` is emitted by the iterator.
    /// Either (1) `combined` or (2) `current'` becomes the previous value
    /// when coalesce continues with the next pair of elements to merge. The
    /// value that remains at the end is also emitted by the iterator.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sum same-sign runs together
    /// let data = vec![-1., -2., -3., 3., 1., 0., -1.];
    /// itertools::assert_equal(data.into_iter().coalesce(|x, y|
    ///         if (x >= 0.) == (y >= 0.) {
    ///             Ok(x + y)
    ///         } else {
    ///             Err((x, y))
    ///         }),
    ///         vec![-6., 4., -1.]);
    /// ```
    fn coalesce<F>(self, f: F) -> Coalesce<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item, Self::Item) -> Result<Self::Item, (Self::Item, Self::Item)>,
    {
        adaptors::coalesce(self, f)
    }
    /// Remove duplicates from sections of consecutive identical elements.
    /// If the iterator is sorted, all elements will be unique.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1., 1., 2., 3., 3., 2., 2.];
    /// itertools::assert_equal(data.into_iter().dedup(),
    ///                         vec![1., 2., 3., 2.]);
    /// ```
    fn dedup(self) -> Dedup<Self>
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        adaptors::dedup(self)
    }
    /// Remove duplicates from sections of consecutive identical elements,
    /// determining equality using a comparison function.
    /// If the iterator is sorted, all elements will be unique.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![(0, 1.), (1, 1.), (0, 2.), (0, 3.), (1, 3.), (1, 2.), (2, 2.)];
    /// itertools::assert_equal(data.into_iter().dedup_by(|x, y| x.1 == y.1),
    ///                         vec![(0, 1.), (0, 2.), (0, 3.), (1, 2.)]);
    /// ```
    fn dedup_by<Cmp>(self, cmp: Cmp) -> DedupBy<Self, Cmp>
    where
        Self: Sized,
        Cmp: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        adaptors::dedup_by(self, cmp)
    }
    /// Remove duplicates from sections of consecutive identical elements, while keeping a count of
    /// how many repeated elements were present.
    /// If the iterator is sorted, all elements will be unique.
    ///
    /// Iterator element type is `(usize, Self::Item)`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec!['a', 'a', 'b', 'c', 'c', 'b', 'b'];
    /// itertools::assert_equal(data.into_iter().dedup_with_count(),
    ///                         vec![(2, 'a'), (1, 'b'), (2, 'c'), (2, 'b')]);
    /// ```
    fn dedup_with_count(self) -> DedupWithCount<Self>
    where
        Self: Sized,
    {
        adaptors::dedup_with_count(self)
    }
    /// Remove duplicates from sections of consecutive identical elements, while keeping a count of
    /// how many repeated elements were present.
    /// This will determine equality using a comparison function.
    /// If the iterator is sorted, all elements will be unique.
    ///
    /// Iterator element type is `(usize, Self::Item)`.
    ///
    /// This iterator is *fused*.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![(0, 'a'), (1, 'a'), (0, 'b'), (0, 'c'), (1, 'c'), (1, 'b'), (2, 'b')];
    /// itertools::assert_equal(data.into_iter().dedup_by_with_count(|x, y| x.1 == y.1),
    ///                         vec![(2, (0, 'a')), (1, (0, 'b')), (2, (0, 'c')), (2, (1, 'b'))]);
    /// ```
    fn dedup_by_with_count<Cmp>(self, cmp: Cmp) -> DedupByWithCount<Self, Cmp>
    where
        Self: Sized,
        Cmp: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        adaptors::dedup_by_with_count(self, cmp)
    }
    /// Return an iterator adaptor that produces elements that appear more than once during the
    /// iteration. Duplicates are detected using hash and equality.
    ///
    /// The iterator is stable, returning the duplicate items in the order in which they occur in
    /// the adapted iterator. Each duplicate item is returned exactly once. If an item appears more
    /// than twice, the second item is the item retained and the rest are discarded.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![10, 20, 30, 20, 40, 10, 50];
    /// itertools::assert_equal(data.into_iter().duplicates(),
    ///                         vec![20, 10]);
    /// ```
    fn duplicates(self) -> Duplicates<Self>
    where
        Self: Sized,
        Self::Item: Eq + Hash,
    {
        duplicates_impl::duplicates(self)
    }
    /// Return an iterator adaptor that produces elements that appear more than once during the
    /// iteration. Duplicates are detected using hash and equality.
    ///
    /// Duplicates are detected by comparing the key they map to with the keying function `f` by
    /// hash and equality. The keys are stored in a hash map in the iterator.
    ///
    /// The iterator is stable, returning the duplicate items in the order in which they occur in
    /// the adapted iterator. Each duplicate item is returned exactly once. If an item appears more
    /// than twice, the second item is the item retained and the rest are discarded.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec!["a", "bb", "aa", "c", "ccc"];
    /// itertools::assert_equal(data.into_iter().duplicates_by(|s| s.len()),
    ///                         vec!["aa", "c"]);
    /// ```
    fn duplicates_by<V, F>(self, f: F) -> DuplicatesBy<Self, V, F>
    where
        Self: Sized,
        V: Eq + Hash,
        F: FnMut(&Self::Item) -> V,
    {
        duplicates_impl::duplicates_by(self, f)
    }
    /// Return an iterator adaptor that filters out elements that have
    /// already been produced once during the iteration. Duplicates
    /// are detected using hash and equality.
    ///
    /// Clones of visited elements are stored in a hash set in the
    /// iterator.
    ///
    /// The iterator is stable, returning the non-duplicate items in the order
    /// in which they occur in the adapted iterator. In a set of duplicate
    /// items, the first item encountered is the item retained.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![10, 20, 30, 20, 40, 10, 50];
    /// itertools::assert_equal(data.into_iter().unique(),
    ///                         vec![10, 20, 30, 40, 50]);
    /// ```
    fn unique(self) -> Unique<Self>
    where
        Self: Sized,
        Self::Item: Clone + Eq + Hash,
    {
        unique_impl::unique(self)
    }
    /// Return an iterator adaptor that filters out elements that have
    /// already been produced once during the iteration.
    ///
    /// Duplicates are detected by comparing the key they map to
    /// with the keying function `f` by hash and equality.
    /// The keys are stored in a hash set in the iterator.
    ///
    /// The iterator is stable, returning the non-duplicate items in the order
    /// in which they occur in the adapted iterator. In a set of duplicate
    /// items, the first item encountered is the item retained.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec!["a", "bb", "aa", "c", "ccc"];
    /// itertools::assert_equal(data.into_iter().unique_by(|s| s.len()),
    ///                         vec!["a", "bb", "ccc"]);
    /// ```
    fn unique_by<V, F>(self, f: F) -> UniqueBy<Self, V, F>
    where
        Self: Sized,
        V: Eq + Hash,
        F: FnMut(&Self::Item) -> V,
    {
        unique_impl::unique_by(self, f)
    }
    /// Return an iterator adaptor that borrows from this iterator and
    /// takes items while the closure `accept` returns `true`.
    ///
    /// This adaptor can only be used on iterators that implement `PeekingNext`
    /// like `.peekable()`, `put_back` and a few other collection iterators.
    ///
    /// The last and rejected element (first `false`) is still available when
    /// `peeking_take_while` is done.
    ///
    ///
    /// See also [`.take_while_ref()`](Itertools::take_while_ref)
    /// which is a similar adaptor.
    fn peeking_take_while<F>(&mut self, accept: F) -> PeekingTakeWhile<Self, F>
    where
        Self: Sized + PeekingNext,
        F: FnMut(&Self::Item) -> bool,
    {
        peeking_take_while::peeking_take_while(self, accept)
    }
    /// Return an iterator adaptor that borrows from a `Clone`-able iterator
    /// to only pick off elements while the predicate `accept` returns `true`.
    ///
    /// It uses the `Clone` trait to restore the original iterator so that the
    /// last and rejected element (first `false`) is still available when
    /// `take_while_ref` is done.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut hexadecimals = "0123456789abcdef".chars();
    ///
    /// let decimals = hexadecimals.take_while_ref(|c| c.is_numeric())
    ///                            .collect::<String>();
    /// assert_eq!(decimals, "0123456789");
    /// assert_eq!(hexadecimals.next(), Some('a'));
    ///
    /// ```
    fn take_while_ref<F>(&mut self, accept: F) -> TakeWhileRef<Self, F>
    where
        Self: Clone,
        F: FnMut(&Self::Item) -> bool,
    {
        adaptors::take_while_ref(self, accept)
    }
    /// Returns an iterator adaptor that consumes elements while the given
    /// predicate is `true`, *including* the element for which the predicate
    /// first returned `false`.
    ///
    /// The [`.take_while()`][std::iter::Iterator::take_while] adaptor is useful
    /// when you want items satisfying a predicate, but to know when to stop
    /// taking elements, we have to consume that first element that doesn't
    /// satisfy the predicate. This adaptor includes that element where
    /// [`.take_while()`][std::iter::Iterator::take_while] would drop it.
    ///
    /// The [`.take_while_ref()`][crate::Itertools::take_while_ref] adaptor
    /// serves a similar purpose, but this adaptor doesn't require [`Clone`]ing
    /// the underlying elements.
    ///
    /// ```rust
    /// # use itertools::Itertools;
    /// let items = vec![1, 2, 3, 4, 5];
    /// let filtered: Vec<_> = items
    ///     .into_iter()
    ///     .take_while_inclusive(|&n| n % 3 != 0)
    ///     .collect();
    ///
    /// assert_eq!(filtered, vec![1, 2, 3]);
    /// ```
    ///
    /// ```rust
    /// # use itertools::Itertools;
    /// let items = vec![1, 2, 3, 4, 5];
    ///
    /// let take_while_inclusive_result: Vec<_> = items
    ///     .iter()
    ///     .copied()
    ///     .take_while_inclusive(|&n| n % 3 != 0)
    ///     .collect();
    /// let take_while_result: Vec<_> = items
    ///     .into_iter()
    ///     .take_while(|&n| n % 3 != 0)
    ///     .collect();
    ///
    /// assert_eq!(take_while_inclusive_result, vec![1, 2, 3]);
    /// assert_eq!(take_while_result, vec![1, 2]);
    /// // both iterators have the same items remaining at this point---the 3
    /// // is lost from the `take_while` vec
    /// ```
    ///
    /// ```rust
    /// # use itertools::Itertools;
    /// #[derive(Debug, PartialEq)]
    /// struct NoCloneImpl(i32);
    ///
    /// let non_clonable_items: Vec<_> = vec![1, 2, 3, 4, 5]
    ///     .into_iter()
    ///     .map(NoCloneImpl)
    ///     .collect();
    /// let filtered: Vec<_> = non_clonable_items
    ///     .into_iter()
    ///     .take_while_inclusive(|n| n.0 % 3 != 0)
    ///     .collect();
    /// let expected: Vec<_> = vec![1, 2, 3].into_iter().map(NoCloneImpl).collect();
    /// assert_eq!(filtered, expected);
    #[doc(alias = "take_until")]
    fn take_while_inclusive<F>(self, accept: F) -> TakeWhileInclusive<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> bool,
    {
        take_while_inclusive::TakeWhileInclusive::new(self, accept)
    }
    /// Return an iterator adaptor that filters `Option<A>` iterator elements
    /// and produces `A`. Stops on the first `None` encountered.
    ///
    /// Iterator element type is `A`, the unwrapped element.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // List all hexadecimal digits
    /// itertools::assert_equal(
    ///     (0..).map(|i| std::char::from_digit(i, 16)).while_some(),
    ///     "0123456789abcdef".chars());
    ///
    /// ```
    fn while_some<A>(self) -> WhileSome<Self>
    where
        Self: Sized + Iterator<Item = Option<A>>,
    {
        adaptors::while_some(self)
    }
    /// Return an iterator adaptor that iterates over the combinations of the
    /// elements from an iterator.
    ///
    /// Iterator element can be any homogeneous tuple of type `Self::Item` with
    /// size up to 12.
    ///
    /// # Guarantees
    ///
    /// If the adapted iterator is deterministic,
    /// this iterator adapter yields items in a reliable order.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut v = Vec::new();
    /// for (a, b) in (1..5).tuple_combinations() {
    ///     v.push((a, b));
    /// }
    /// assert_eq!(v, vec![(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)]);
    ///
    /// let mut it = (1..5).tuple_combinations();
    /// assert_eq!(Some((1, 2, 3)), it.next());
    /// assert_eq!(Some((1, 2, 4)), it.next());
    /// assert_eq!(Some((1, 3, 4)), it.next());
    /// assert_eq!(Some((2, 3, 4)), it.next());
    /// assert_eq!(None, it.next());
    ///
    /// // this requires a type hint
    /// let it = (1..5).tuple_combinations::<(_, _, _)>();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (1, 2, 4), (1, 3, 4), (2, 3, 4)]);
    ///
    /// // you can also specify the complete type
    /// use itertools::TupleCombinations;
    /// use std::ops::Range;
    ///
    /// let it: TupleCombinations<Range<u32>, (u32, u32, u32)> = (1..5).tuple_combinations();
    /// itertools::assert_equal(it, vec![(1, 2, 3), (1, 2, 4), (1, 3, 4), (2, 3, 4)]);
    /// ```
    fn tuple_combinations<T>(self) -> TupleCombinations<Self, T>
    where
        Self: Sized + Clone,
        Self::Item: Clone,
        T: adaptors::HasCombination<Self>,
    {
        adaptors::tuple_combinations(self)
    }
    /// Return an iterator adaptor that iterates over the combinations of the
    /// elements from an iterator.
    ///
    /// Iterator element type is [Self::Item; K]. The iterator produces a new
    /// array per iteration, and clones the iterator elements.
    ///
    /// # Guarantees
    ///
    /// If the adapted iterator is deterministic,
    /// this iterator adapter yields items in a reliable order.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut v = Vec::new();
    /// for [a, b] in (1..5).array_combinations() {
    ///     v.push([a, b]);
    /// }
    /// assert_eq!(v, vec![[1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4]]);
    ///
    /// let mut it = (1..5).array_combinations();
    /// assert_eq!(Some([1, 2, 3]), it.next());
    /// assert_eq!(Some([1, 2, 4]), it.next());
    /// assert_eq!(Some([1, 3, 4]), it.next());
    /// assert_eq!(Some([2, 3, 4]), it.next());
    /// assert_eq!(None, it.next());
    ///
    /// // this requires a type hint
    /// let it = (1..5).array_combinations::<3>();
    /// itertools::assert_equal(it, vec![[1, 2, 3], [1, 2, 4], [1, 3, 4], [2, 3, 4]]);
    ///
    /// // you can also specify the complete type
    /// use itertools::ArrayCombinations;
    /// use std::ops::Range;
    ///
    /// let it: ArrayCombinations<Range<u32>, 3> = (1..5).array_combinations();
    /// itertools::assert_equal(it, vec![[1, 2, 3], [1, 2, 4], [1, 3, 4], [2, 3, 4]]);
    /// ```
    fn array_combinations<const K: usize>(self) -> ArrayCombinations<Self, K>
    where
        Self: Sized + Clone,
        Self::Item: Clone,
    {
        combinations::array_combinations(self)
    }
    /// Return an iterator adaptor that iterates over the `k`-length combinations of
    /// the elements from an iterator.
    ///
    /// Iterator element type is `Vec<Self::Item>`. The iterator produces a new `Vec` per iteration,
    /// and clones the iterator elements.
    ///
    /// # Guarantees
    ///
    /// If the adapted iterator is deterministic,
    /// this iterator adapter yields items in a reliable order.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (1..5).combinations(3);
    /// itertools::assert_equal(it, vec![
    ///     vec![1, 2, 3],
    ///     vec![1, 2, 4],
    ///     vec![1, 3, 4],
    ///     vec![2, 3, 4],
    /// ]);
    /// ```
    ///
    /// Note: Combinations does not take into account the equality of the iterated values.
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = vec![1, 2, 2].into_iter().combinations(2);
    /// itertools::assert_equal(it, vec![
    ///     vec![1, 2], // Note: these are the same
    ///     vec![1, 2], // Note: these are the same
    ///     vec![2, 2],
    /// ]);
    /// ```
    fn combinations(self, k: usize) -> Combinations<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        combinations::combinations(self, k)
    }
    /// Return an iterator that iterates over the `k`-length combinations of
    /// the elements from an iterator, with replacement.
    ///
    /// Iterator element type is `Vec<Self::Item>`. The iterator produces a new `Vec` per iteration,
    /// and clones the iterator elements.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (1..4).combinations_with_replacement(2);
    /// itertools::assert_equal(it, vec![
    ///     vec![1, 1],
    ///     vec![1, 2],
    ///     vec![1, 3],
    ///     vec![2, 2],
    ///     vec![2, 3],
    ///     vec![3, 3],
    /// ]);
    /// ```
    fn combinations_with_replacement(self, k: usize) -> CombinationsWithReplacement<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        combinations_with_replacement::combinations_with_replacement(self, k)
    }
    /// Return an iterator adaptor that iterates over all k-permutations of the
    /// elements from an iterator.
    ///
    /// Iterator element type is `Vec<Self::Item>` with length `k`. The iterator
    /// produces a new `Vec` per iteration, and clones the iterator elements.
    ///
    /// If `k` is greater than the length of the input iterator, the resultant
    /// iterator adaptor will be empty.
    ///
    /// If you are looking for permutations with replacements,
    /// use `repeat_n(iter, k).multi_cartesian_product()` instead.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let perms = (5..8).permutations(2);
    /// itertools::assert_equal(perms, vec![
    ///     vec![5, 6],
    ///     vec![5, 7],
    ///     vec![6, 5],
    ///     vec![6, 7],
    ///     vec![7, 5],
    ///     vec![7, 6],
    /// ]);
    /// ```
    ///
    /// Note: Permutations does not take into account the equality of the iterated values.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = vec![2, 2].into_iter().permutations(2);
    /// itertools::assert_equal(it, vec![
    ///     vec![2, 2], // Note: these are the same
    ///     vec![2, 2], // Note: these are the same
    /// ]);
    /// ```
    ///
    /// Note: The source iterator is collected lazily, and will not be
    /// re-iterated if the permutations adaptor is completed and re-iterated.
    fn permutations(self, k: usize) -> Permutations<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        permutations::permutations(self, k)
    }
    /// Return an iterator that iterates through the powerset of the elements from an
    /// iterator.
    ///
    /// Iterator element type is `Vec<Self::Item>`. The iterator produces a new `Vec`
    /// per iteration, and clones the iterator elements.
    ///
    /// The powerset of a set contains all subsets including the empty set and the full
    /// input set. A powerset has length _2^n_ where _n_ is the length of the input
    /// set.
    ///
    /// Each `Vec` produced by this iterator represents a subset of the elements
    /// produced by the source iterator.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let sets = (1..4).powerset().collect::<Vec<_>>();
    /// itertools::assert_equal(sets, vec![
    ///     vec![],
    ///     vec![1],
    ///     vec![2],
    ///     vec![3],
    ///     vec![1, 2],
    ///     vec![1, 3],
    ///     vec![2, 3],
    ///     vec![1, 2, 3],
    /// ]);
    /// ```
    fn powerset(self) -> Powerset<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        powerset::powerset(self)
    }
    /// Return an iterator adaptor that pads the sequence to a minimum length of
    /// `min` by filling missing elements using a closure `f`.
    ///
    /// Iterator element type is `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let it = (0..5).pad_using(10, |i| 2*i);
    /// itertools::assert_equal(it, vec![0, 1, 2, 3, 4, 10, 12, 14, 16, 18]);
    ///
    /// let it = (0..10).pad_using(5, |i| 2*i);
    /// itertools::assert_equal(it, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    ///
    /// let it = (0..5).pad_using(10, |i| 2*i).rev();
    /// itertools::assert_equal(it, vec![18, 16, 14, 12, 10, 4, 3, 2, 1, 0]);
    /// ```
    fn pad_using<F>(self, min: usize, f: F) -> PadUsing<Self, F>
    where
        Self: Sized,
        F: FnMut(usize) -> Self::Item,
    {
        pad_tail::pad_using(self, min, f)
    }
    /// Return an iterator adaptor that combines each element with a `Position` to
    /// ease special-case handling of the first or last elements.
    ///
    /// Iterator element type is
    /// [`(Position, Self::Item)`](Position)
    ///
    /// ```
    /// use itertools::{Itertools, Position};
    ///
    /// let it = (0..4).with_position();
    /// itertools::assert_equal(it,
    ///                         vec![(Position::First, 0),
    ///                              (Position::Middle, 1),
    ///                              (Position::Middle, 2),
    ///                              (Position::Last, 3)]);
    ///
    /// let it = (0..1).with_position();
    /// itertools::assert_equal(it, vec![(Position::Only, 0)]);
    /// ```
    fn with_position(self) -> WithPosition<Self>
    where
        Self: Sized,
    {
        with_position::with_position(self)
    }
    /// Return an iterator adaptor that yields the indices of all elements
    /// satisfying a predicate, counted from the start of the iterator.
    ///
    /// Equivalent to `iter.enumerate().filter(|(_, v)| predicate(*v)).map(|(i, _)| i)`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1, 2, 3, 3, 4, 6, 7, 9];
    /// itertools::assert_equal(data.iter().positions(|v| v % 2 == 0), vec![1, 4, 5]);
    ///
    /// itertools::assert_equal(data.iter().positions(|v| v % 2 == 1).rev(), vec![7, 6, 3, 2, 0]);
    /// ```
    fn positions<P>(self, predicate: P) -> Positions<Self, P>
    where
        Self: Sized,
        P: FnMut(Self::Item) -> bool,
    {
        adaptors::positions(self, predicate)
    }
    /// Return an iterator adaptor that applies a mutating function
    /// to each element before yielding it.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let input = vec![vec![1], vec![3, 2, 1]];
    /// let it = input.into_iter().update(|v| v.push(0));
    /// itertools::assert_equal(it, vec![vec![1, 0], vec![3, 2, 1, 0]]);
    /// ```
    fn update<F>(self, updater: F) -> Update<Self, F>
    where
        Self: Sized,
        F: FnMut(&mut Self::Item),
    {
        adaptors::update(self, updater)
    }
    /// Advances the iterator and returns the next items grouped in an array of
    /// a specific size.
    ///
    /// If there are enough elements to be grouped in an array, then the array
    /// is returned inside `Some`, otherwise `None` is returned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut iter = 1..5;
    ///
    /// assert_eq!(Some([1, 2]), iter.next_array());
    /// ```
    fn next_array<const N: usize>(&mut self) -> Option<[Self::Item; N]>
    where
        Self: Sized,
    {
        next_array::next_array(self)
    }
    /// Collects all items from the iterator into an array of a specific size.
    ///
    /// If the number of elements inside the iterator is **exactly** equal to
    /// the array size, then the array is returned inside `Some`, otherwise
    /// `None` is returned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let iter = 1..3;
    ///
    /// if let Some([x, y]) = iter.collect_array() {
    ///     assert_eq!([x, y], [1, 2])
    /// } else {
    ///     panic!("Expected two elements")
    /// }
    /// ```
    fn collect_array<const N: usize>(mut self) -> Option<[Self::Item; N]>
    where
        Self: Sized,
    {
        self.next_array().filter(|_| self.next().is_none())
    }
    /// Advances the iterator and returns the next items grouped in a tuple of
    /// a specific size (up to 12).
    ///
    /// If there are enough elements to be grouped in a tuple, then the tuple is
    /// returned inside `Some`, otherwise `None` is returned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut iter = 1..5;
    ///
    /// assert_eq!(Some((1, 2)), iter.next_tuple());
    /// ```
    fn next_tuple<T>(&mut self) -> Option<T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: traits::HomogeneousTuple,
    {
        T::collect_from_iter_no_buf(self)
    }
    /// Collects all items from the iterator into a tuple of a specific size
    /// (up to 12).
    ///
    /// If the number of elements inside the iterator is **exactly** equal to
    /// the tuple size, then the tuple is returned inside `Some`, otherwise
    /// `None` is returned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let iter = 1..3;
    ///
    /// if let Some((x, y)) = iter.collect_tuple() {
    ///     assert_eq!((x, y), (1, 2))
    /// } else {
    ///     panic!("Expected two elements")
    /// }
    /// ```
    fn collect_tuple<T>(mut self) -> Option<T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: traits::HomogeneousTuple,
    {
        match self.next_tuple() {
            elt @ Some(_) => {
                match self.next() {
                    Some(_) => None,
                    None => elt,
                }
            }
            _ => None,
        }
    }
    /// Find the position and value of the first element satisfying a predicate.
    ///
    /// The iterator is not advanced past the first element found.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let text = "Hα";
    /// assert_eq!(text.chars().find_position(|ch| ch.is_lowercase()), Some((1, 'α')));
    /// ```
    fn find_position<P>(&mut self, mut pred: P) -> Option<(usize, Self::Item)>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        self.enumerate().find(|(_, elt)| pred(elt))
    }
    /// Find the value of the first element satisfying a predicate or return the last element, if any.
    ///
    /// The iterator is not advanced past the first element found.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let numbers = [1, 2, 3, 4];
    /// assert_eq!(numbers.iter().find_or_last(|&&x| x > 5), Some(&4));
    /// assert_eq!(numbers.iter().find_or_last(|&&x| x > 2), Some(&3));
    /// assert_eq!(std::iter::empty::<i32>().find_or_last(|&x| x > 5), None);
    ///
    /// // An iterator of Results can return the first Ok or the last Err:
    /// let input = vec![Err(()), Ok(11), Err(()), Ok(22)];
    /// assert_eq!(input.into_iter().find_or_last(Result::is_ok), Some(Ok(11)));
    ///
    /// let input: Vec<Result<(), i32>> = vec![Err(11), Err(22)];
    /// assert_eq!(input.into_iter().find_or_last(Result::is_ok), Some(Err(22)));
    ///
    /// assert_eq!(std::iter::empty::<Result<(), i32>>().find_or_last(Result::is_ok), None);
    /// ```
    fn find_or_last<P>(mut self, mut predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        let mut prev = None;
        self.find_map(|x| {
                if predicate(&x) {
                    Some(x)
                } else {
                    prev = Some(x);
                    None
                }
            })
            .or(prev)
    }
    /// Find the value of the first element satisfying a predicate or return the first element, if any.
    ///
    /// The iterator is not advanced past the first element found.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let numbers = [1, 2, 3, 4];
    /// assert_eq!(numbers.iter().find_or_first(|&&x| x > 5), Some(&1));
    /// assert_eq!(numbers.iter().find_or_first(|&&x| x > 2), Some(&3));
    /// assert_eq!(std::iter::empty::<i32>().find_or_first(|&x| x > 5), None);
    ///
    /// // An iterator of Results can return the first Ok or the first Err:
    /// let input = vec![Err(()), Ok(11), Err(()), Ok(22)];
    /// assert_eq!(input.into_iter().find_or_first(Result::is_ok), Some(Ok(11)));
    ///
    /// let input: Vec<Result<(), i32>> = vec![Err(11), Err(22)];
    /// assert_eq!(input.into_iter().find_or_first(Result::is_ok), Some(Err(11)));
    ///
    /// assert_eq!(std::iter::empty::<Result<(), i32>>().find_or_first(Result::is_ok), None);
    /// ```
    fn find_or_first<P>(mut self, mut predicate: P) -> Option<Self::Item>
    where
        Self: Sized,
        P: FnMut(&Self::Item) -> bool,
    {
        let first = self.next()?;
        Some(
            if predicate(&first) {
                first
            } else {
                self.find(|x| predicate(x)).unwrap_or(first)
            },
        )
    }
    /// Returns `true` if the given item is present in this iterator.
    ///
    /// This method is short-circuiting. If the given item is present in this
    /// iterator, this method will consume the iterator up-to-and-including
    /// the item. If the given item is not present in this iterator, the
    /// iterator will be exhausted.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// #[derive(PartialEq, Debug)]
    /// enum Enum { A, B, C, D, E, }
    ///
    /// let mut iter = vec![Enum::A, Enum::B, Enum::C, Enum::D].into_iter();
    ///
    /// // search `iter` for `B`
    /// assert_eq!(iter.contains(&Enum::B), true);
    /// // `B` was found, so the iterator now rests at the item after `B` (i.e, `C`).
    /// assert_eq!(iter.next(), Some(Enum::C));
    ///
    /// // search `iter` for `E`
    /// assert_eq!(iter.contains(&Enum::E), false);
    /// // `E` wasn't found, so `iter` is now exhausted
    /// assert_eq!(iter.next(), None);
    /// ```
    fn contains<Q>(&mut self, query: &Q) -> bool
    where
        Self: Sized,
        Self::Item: Borrow<Q>,
        Q: PartialEq + ?Sized,
    {
        self.any(|x| x.borrow() == query)
    }
    /// Check whether all elements compare equal.
    ///
    /// Empty iterators are considered to have equal elements:
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1, 1, 1, 2, 2, 3, 3, 3, 4, 5, 5];
    /// assert!(!data.iter().all_equal());
    /// assert!(data[0..3].iter().all_equal());
    /// assert!(data[3..5].iter().all_equal());
    /// assert!(data[5..8].iter().all_equal());
    ///
    /// let data : Option<usize> = None;
    /// assert!(data.into_iter().all_equal());
    /// ```
    fn all_equal(&mut self) -> bool
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        match self.next() {
            None => true,
            Some(a) => self.all(|x| a == x),
        }
    }
    /// If there are elements and they are all equal, return a single copy of that element.
    /// If there are no elements, return an Error containing None.
    /// If there are elements and they are not all equal, return a tuple containing the first
    /// two non-equal elements found.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1, 1, 1, 2, 2, 3, 3, 3, 4, 5, 5];
    /// assert_eq!(data.iter().all_equal_value(), Err(Some((&1, &2))));
    /// assert_eq!(data[0..3].iter().all_equal_value(), Ok(&1));
    /// assert_eq!(data[3..5].iter().all_equal_value(), Ok(&2));
    /// assert_eq!(data[5..8].iter().all_equal_value(), Ok(&3));
    ///
    /// let data : Option<usize> = None;
    /// assert_eq!(data.into_iter().all_equal_value(), Err(None));
    /// ```
    #[allow(clippy::type_complexity)]
    fn all_equal_value(&mut self) -> Result<Self::Item, Option<(Self::Item, Self::Item)>>
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        let first = self.next().ok_or(None)?;
        let other = self.find(|x| x != &first);
        if let Some(other) = other { Err(Some((first, other))) } else { Ok(first) }
    }
    /// Check whether all elements are unique (non equal).
    ///
    /// Empty iterators are considered to have unique elements:
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![1, 2, 3, 4, 1, 5];
    /// assert!(!data.iter().all_unique());
    /// assert!(data[0..4].iter().all_unique());
    /// assert!(data[1..6].iter().all_unique());
    ///
    /// let data : Option<usize> = None;
    /// assert!(data.into_iter().all_unique());
    /// ```
    fn all_unique(&mut self) -> bool
    where
        Self: Sized,
        Self::Item: Eq + Hash,
    {
        let mut used = HashSet::new();
        self.all(move |elt| used.insert(elt))
    }
    /// Consume the first `n` elements from the iterator eagerly,
    /// and return the same iterator again.
    ///
    /// It works similarly to `.skip(n)` except it is eager and
    /// preserves the iterator type.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let iter = "αβγ".chars().dropping(2);
    /// itertools::assert_equal(iter, "γ".chars());
    /// ```
    ///
    /// *Fusing notes: if the iterator is exhausted by dropping,
    /// the result of calling `.next()` again depends on the iterator implementation.*
    fn dropping(mut self, n: usize) -> Self
    where
        Self: Sized,
    {
        if n > 0 {
            self.nth(n - 1);
        }
        self
    }
    /// Consume the last `n` elements from the iterator eagerly,
    /// and return the same iterator again.
    ///
    /// This is only possible on double ended iterators. `n` may be
    /// larger than the number of elements.
    ///
    /// Note: This method is eager, dropping the back elements immediately and
    /// preserves the iterator type.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let init = vec![0, 3, 6, 9].into_iter().dropping_back(1);
    /// itertools::assert_equal(init, vec![0, 3, 6]);
    /// ```
    fn dropping_back(mut self, n: usize) -> Self
    where
        Self: Sized + DoubleEndedIterator,
    {
        if n > 0 {
            (&mut self).rev().nth(n - 1);
        }
        self
    }
    /// Combine all an iterator's elements into one element by using [`Extend`].
    ///
    /// This combinator will extend the first item with each of the rest of the
    /// items of the iterator. If the iterator is empty, the default value of
    /// `I::Item` is returned.
    ///
    /// ```rust
    /// use itertools::Itertools;
    ///
    /// let input = vec![vec![1], vec![2, 3], vec![4, 5, 6]];
    /// assert_eq!(input.into_iter().concat(),
    ///            vec![1, 2, 3, 4, 5, 6]);
    /// ```
    fn concat(self) -> Self::Item
    where
        Self: Sized,
        Self::Item: Extend<<<Self as Iterator>::Item as IntoIterator>::Item>
            + IntoIterator + Default,
    {
        concat(self)
    }
    /// `.collect_vec()` is simply a type specialization of [`Iterator::collect`],
    /// for convenience.
    fn collect_vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
    /// `.try_collect()` is more convenient way of writing
    /// `.collect::<Result<_, _>>()`
    ///
    /// # Example
    ///
    /// ```
    /// use std::{fs, io};
    /// use itertools::Itertools;
    ///
    /// fn process_dir_entries(entries: &[fs::DirEntry]) {
    ///     // ...
    ///     # let _ = entries;
    /// }
    ///
    /// fn do_stuff() -> io::Result<()> {
    ///     let entries: Vec<_> = fs::read_dir(".")?.try_collect()?;
    ///     process_dir_entries(&entries);
    ///
    ///     Ok(())
    /// }
    ///
    /// # let _ = do_stuff;
    /// ```
    fn try_collect<T, U, E>(self) -> Result<U, E>
    where
        Self: Sized + Iterator<Item = Result<T, E>>,
        Result<U, E>: FromIterator<Result<T, E>>,
    {
        self.collect()
    }
    /// Assign to each reference in `self` from the `from` iterator,
    /// stopping at the shortest of the two iterators.
    ///
    /// The `from` iterator is queried for its next element before the `self`
    /// iterator, and if either is exhausted the method is done.
    ///
    /// Return the number of elements written.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut xs = [0; 4];
    /// xs.iter_mut().set_from(1..);
    /// assert_eq!(xs, [1, 2, 3, 4]);
    /// ```
    #[inline]
    fn set_from<'a, A: 'a, J>(&mut self, from: J) -> usize
    where
        Self: Iterator<Item = &'a mut A>,
        J: IntoIterator<Item = A>,
    {
        from.into_iter().zip(self).map(|(new, old)| *old = new).count()
    }
    /// Combine all iterator elements into one `String`, separated by `sep`.
    ///
    /// Use the `Display` implementation of each element.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// assert_eq!(["a", "b", "c"].iter().join(", "), "a, b, c");
    /// assert_eq!([1, 2, 3].iter().join(", "), "1, 2, 3");
    /// ```
    fn join(&mut self, sep: &str) -> String
    where
        Self::Item: std::fmt::Display,
    {
        match self.next() {
            None => String::new(),
            Some(first_elt) => {
                let (lower, _) = self.size_hint();
                let mut result = String::with_capacity(sep.len() * lower);
                (&mut result).write_fmt(format_args!("{0}", first_elt)).unwrap();
                self.for_each(|elt| {
                    result.push_str(sep);
                    (&mut result).write_fmt(format_args!("{0}", elt)).unwrap();
                });
                result
            }
        }
    }
    /// Format all iterator elements, separated by `sep`.
    ///
    /// All elements are formatted (any formatting trait)
    /// with `sep` inserted between each element.
    ///
    /// **Panics** if the formatter helper is formatted more than once.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = [1.1, 2.71828, -3.];
    /// assert_eq!(
    ///     format!("{:.2}", data.iter().format(", ")),
    ///            "1.10, 2.72, -3.00");
    /// ```
    fn format(self, sep: &str) -> Format<Self>
    where
        Self: Sized,
    {
        format::new_format_default(self, sep)
    }
    /// Format all iterator elements, separated by `sep`.
    ///
    /// This is a customizable version of [`.format()`](Itertools::format).
    ///
    /// The supplied closure `format` is called once per iterator element,
    /// with two arguments: the element and a callback that takes a
    /// `&Display` value, i.e. any reference to type that implements `Display`.
    ///
    /// Using `&format_args!(...)` is the most versatile way to apply custom
    /// element formatting. The callback can be called multiple times if needed.
    ///
    /// **Panics** if the formatter helper is formatted more than once.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = [1.1, 2.71828, -3.];
    /// let data_formatter = data.iter().format_with(", ", |elt, f| f(&format_args!("{:.2}", elt)));
    /// assert_eq!(format!("{}", data_formatter),
    ///            "1.10, 2.72, -3.00");
    ///
    /// // .format_with() is recursively composable
    /// let matrix = [[1., 2., 3.],
    ///               [4., 5., 6.]];
    /// let matrix_formatter = matrix.iter().format_with("\n", |row, f| {
    ///                                 f(&row.iter().format_with(", ", |elt, g| g(&elt)))
    ///                              });
    /// assert_eq!(format!("{}", matrix_formatter),
    ///            "1, 2, 3\n4, 5, 6");
    ///
    ///
    /// ```
    fn format_with<F>(self, sep: &str, format: F) -> FormatWith<Self, F>
    where
        Self: Sized,
        F: FnMut(
            Self::Item,
            &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result,
        ) -> fmt::Result,
    {
        format::new_format(self, sep, format)
    }
    /// Fold `Result` values from an iterator.
    ///
    /// Only `Ok` values are folded. If no error is encountered, the folded
    /// value is returned inside `Ok`. Otherwise, the operation terminates
    /// and returns the first `Err` value it encounters. No iterator elements are
    /// consumed after the first error.
    ///
    /// The first accumulator value is the `start` parameter.
    /// Each iteration passes the accumulator value and the next value inside `Ok`
    /// to the fold function `f` and its return value becomes the new accumulator value.
    ///
    /// For example the sequence *Ok(1), Ok(2), Ok(3)* will result in a
    /// computation like this:
    ///
    /// ```no_run
    /// # let start = 0;
    /// # let f = |x, y| x + y;
    /// let mut accum = start;
    /// accum = f(accum, 1);
    /// accum = f(accum, 2);
    /// accum = f(accum, 3);
    /// # let _ = accum;
    /// ```
    ///
    /// With a `start` value of 0 and an addition as folding function,
    /// this effectively results in *((0 + 1) + 2) + 3*
    ///
    /// ```
    /// use std::ops::Add;
    /// use itertools::Itertools;
    ///
    /// let values = [1, 2, -2, -1, 2, 1];
    /// assert_eq!(
    ///     values.iter()
    ///           .map(Ok::<_, ()>)
    ///           .fold_ok(0, Add::add),
    ///     Ok(3)
    /// );
    /// assert!(
    ///     values.iter()
    ///           .map(|&x| if x >= 0 { Ok(x) } else { Err("Negative number") })
    ///           .fold_ok(0, Add::add)
    ///           .is_err()
    /// );
    /// ```
    fn fold_ok<A, E, B, F>(&mut self, mut start: B, mut f: F) -> Result<B, E>
    where
        Self: Iterator<Item = Result<A, E>>,
        F: FnMut(B, A) -> B,
    {
        for elt in self {
            match elt {
                Ok(v) => start = f(start, v),
                Err(u) => return Err(u),
            }
        }
        Ok(start)
    }
    /// Fold `Option` values from an iterator.
    ///
    /// Only `Some` values are folded. If no `None` is encountered, the folded
    /// value is returned inside `Some`. Otherwise, the operation terminates
    /// and returns `None`. No iterator elements are consumed after the `None`.
    ///
    /// This is the `Option` equivalent to [`fold_ok`](Itertools::fold_ok).
    ///
    /// ```
    /// use std::ops::Add;
    /// use itertools::Itertools;
    ///
    /// let mut values = vec![Some(1), Some(2), Some(-2)].into_iter();
    /// assert_eq!(values.fold_options(5, Add::add), Some(5 + 1 + 2 - 2));
    ///
    /// let mut more_values = vec![Some(2), None, Some(0)].into_iter();
    /// assert!(more_values.fold_options(0, Add::add).is_none());
    /// assert_eq!(more_values.next().unwrap(), Some(0));
    /// ```
    fn fold_options<A, B, F>(&mut self, mut start: B, mut f: F) -> Option<B>
    where
        Self: Iterator<Item = Option<A>>,
        F: FnMut(B, A) -> B,
    {
        for elt in self {
            match elt {
                Some(v) => start = f(start, v),
                None => return None,
            }
        }
        Some(start)
    }
    /// Accumulator of the elements in the iterator.
    ///
    /// Like `.fold()`, without a base case. If the iterator is
    /// empty, return `None`. With just one element, return it.
    /// Otherwise elements are accumulated in sequence using the closure `f`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// assert_eq!((0..10).fold1(|x, y| x + y).unwrap_or(0), 45);
    /// assert_eq!((0..0).fold1(|x, y| x * y), None);
    /// ```
    #[deprecated(
        note = "Use [`Iterator::reduce`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.reduce) instead",
        since = "0.10.2"
    )]
    fn fold1<F>(mut self, f: F) -> Option<Self::Item>
    where
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
        Self: Sized,
    {
        self.next().map(move |x| self.fold(x, f))
    }
    /// Accumulate the elements in the iterator in a tree-like manner.
    ///
    /// You can think of it as, while there's more than one item, repeatedly
    /// combining adjacent items.  It does so in bottom-up-merge-sort order,
    /// however, so that it needs only logarithmic stack space.
    ///
    /// This produces a call tree like the following (where the calls under
    /// an item are done after reading that item):
    ///
    /// ```text
    /// 1 2 3 4 5 6 7
    /// │ │ │ │ │ │ │
    /// └─f └─f └─f │
    ///   │   │   │ │
    ///   └───f   └─f
    ///       │     │
    ///       └─────f
    /// ```
    ///
    /// Which, for non-associative functions, will typically produce a different
    /// result than the linear call tree used by [`Iterator::reduce`]:
    ///
    /// ```text
    /// 1 2 3 4 5 6 7
    /// │ │ │ │ │ │ │
    /// └─f─f─f─f─f─f
    /// ```
    ///
    /// If `f` is associative you should also decide carefully:
    ///
    /// For an iterator producing `n` elements, both [`Iterator::reduce`] and `tree_reduce` will
    /// call `f` `n - 1` times. However, `tree_reduce` will call `f` on earlier intermediate
    /// results, which is beneficial for `f` that allocate and produce longer results for longer
    /// arguments. For example if `f` combines arguments using `format!`, then `tree_reduce` will
    /// operate on average on shorter arguments resulting in less bytes being allocated overall.
    ///
    /// Moreover, the output of `tree_reduce` is preferable to that of [`Iterator::reduce`] in
    /// certain cases. For example, building a binary search tree using `tree_reduce` will result in
    /// a balanced tree with height `O(ln(n))`, while [`Iterator::reduce`] will output a tree with
    /// height `O(n)`, essentially a linked list.
    ///
    /// If `f` does not benefit from such a reordering, like `u32::wrapping_add`, prefer the
    /// normal [`Iterator::reduce`] instead since it will most likely result in the generation of
    /// simpler code because the compiler is able to optimize it.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let f = |a: String, b: String| {
    ///     format!("f({a}, {b})")
    /// };
    ///
    /// // The same tree as above
    /// assert_eq!((1..8).map(|x| x.to_string()).tree_reduce(f),
    ///            Some(String::from("f(f(f(1, 2), f(3, 4)), f(f(5, 6), 7))")));
    ///
    /// // Like reduce, an empty iterator produces None
    /// assert_eq!((0..0).tree_reduce(|x, y| x * y), None);
    ///
    /// // tree_reduce matches reduce for associative operations...
    /// assert_eq!((0..10).tree_reduce(|x, y| x + y),
    ///     (0..10).reduce(|x, y| x + y));
    ///
    /// // ...but not for non-associative ones
    /// assert_ne!((0..10).tree_reduce(|x, y| x - y),
    ///     (0..10).reduce(|x, y| x - y));
    ///
    /// let mut total_len_reduce = 0;
    /// let reduce_res = (1..100).map(|x| x.to_string())
    ///     .reduce(|a, b| {
    ///         let r = f(a, b);
    ///         total_len_reduce += r.len();
    ///         r
    ///     })
    ///     .unwrap();
    ///
    /// let mut total_len_tree_reduce = 0;
    /// let tree_reduce_res = (1..100).map(|x| x.to_string())
    ///     .tree_reduce(|a, b| {
    ///         let r = f(a, b);
    ///         total_len_tree_reduce += r.len();
    ///         r
    ///     })
    ///     .unwrap();
    ///
    /// assert_eq!(total_len_reduce, 33299);
    /// assert_eq!(total_len_tree_reduce, 4228);
    /// assert_eq!(reduce_res.len(), tree_reduce_res.len());
    /// ```
    fn tree_reduce<F>(mut self, mut f: F) -> Option<Self::Item>
    where
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
        Self: Sized,
    {
        type State<T> = Result<T, Option<T>>;
        fn inner0<T, II, FF>(it: &mut II, f: &mut FF) -> State<T>
        where
            II: Iterator<Item = T>,
            FF: FnMut(T, T) -> T,
        {
            let a = if let Some(v) = it.next() {
                v
            } else {
                return Err(None);
            };
            let b = if let Some(v) = it.next() {
                v
            } else {
                return Err(Some(a));
            };
            Ok(f(a, b))
        }
        fn inner<T, II, FF>(stop: usize, it: &mut II, f: &mut FF) -> State<T>
        where
            II: Iterator<Item = T>,
            FF: FnMut(T, T) -> T,
        {
            let mut x = inner0(it, f)?;
            for height in 0..stop {
                let next = if height == 0 {
                    inner0(it, f)
                } else {
                    inner(height, it, f)
                };
                match next {
                    Ok(y) => x = f(x, y),
                    Err(None) => return Err(Some(x)),
                    Err(Some(y)) => return Err(Some(f(x, y))),
                }
            }
            Ok(x)
        }
        match inner(usize::MAX, &mut self, &mut f) {
            Err(x) => x,
            _ => ::core::panicking::panic("internal error: entered unreachable code"),
        }
    }
    /// See [`.tree_reduce()`](Itertools::tree_reduce).
    #[deprecated(note = "Use .tree_reduce() instead", since = "0.13.0")]
    fn tree_fold1<F>(self, f: F) -> Option<Self::Item>
    where
        F: FnMut(Self::Item, Self::Item) -> Self::Item,
        Self: Sized,
    {
        self.tree_reduce(f)
    }
    /// An iterator method that applies a function, producing a single, final value.
    ///
    /// `fold_while()` is basically equivalent to [`Iterator::fold`] but with additional support for
    /// early exit via short-circuiting.
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::FoldWhile::{Continue, Done};
    ///
    /// let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    ///
    /// let mut result = 0;
    ///
    /// // for loop:
    /// for i in &numbers {
    ///     if *i > 5 {
    ///         break;
    ///     }
    ///     result = result + i;
    /// }
    ///
    /// // fold:
    /// let result2 = numbers.iter().fold(0, |acc, x| {
    ///     if *x > 5 { acc } else { acc + x }
    /// });
    ///
    /// // fold_while:
    /// let result3 = numbers.iter().fold_while(0, |acc, x| {
    ///     if *x > 5 { Done(acc) } else { Continue(acc + x) }
    /// }).into_inner();
    ///
    /// // they're the same
    /// assert_eq!(result, result2);
    /// assert_eq!(result2, result3);
    /// ```
    ///
    /// The big difference between the computations of `result2` and `result3` is that while
    /// `fold()` called the provided closure for every item of the callee iterator,
    /// `fold_while()` actually stopped iterating as soon as it encountered `Fold::Done(_)`.
    fn fold_while<B, F>(&mut self, init: B, mut f: F) -> FoldWhile<B>
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> FoldWhile<B>,
    {
        use Result::{Err as Break, Ok as Continue};
        let result = self
            .try_fold(
                init,
                #[inline(always)]
                |acc, v| match f(acc, v) {
                    FoldWhile::Continue(acc) => Continue(acc),
                    FoldWhile::Done(acc) => Break(acc),
                },
            );
        match result {
            Continue(acc) => FoldWhile::Continue(acc),
            Break(acc) => FoldWhile::Done(acc),
        }
    }
    /// Iterate over the entire iterator and add all the elements.
    ///
    /// An empty iterator returns `None`, otherwise `Some(sum)`.
    ///
    /// # Panics
    ///
    /// When calling `sum1()` and a primitive integer type is being returned, this
    /// method will panic if the computation overflows and debug assertions are
    /// enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let empty_sum = (1..1).sum1::<i32>();
    /// assert_eq!(empty_sum, None);
    ///
    /// let nonempty_sum = (1..11).sum1::<i32>();
    /// assert_eq!(nonempty_sum, Some(55));
    /// ```
    fn sum1<S>(mut self) -> Option<S>
    where
        Self: Sized,
        S: std::iter::Sum<Self::Item>,
    {
        self.next().map(|first| once(first).chain(self).sum())
    }
    /// Iterate over the entire iterator and multiply all the elements.
    ///
    /// An empty iterator returns `None`, otherwise `Some(product)`.
    ///
    /// # Panics
    ///
    /// When calling `product1()` and a primitive integer type is being returned,
    /// method will panic if the computation overflows and debug assertions are
    /// enabled.
    ///
    /// # Examples
    /// ```
    /// use itertools::Itertools;
    ///
    /// let empty_product = (1..1).product1::<i32>();
    /// assert_eq!(empty_product, None);
    ///
    /// let nonempty_product = (1..11).product1::<i32>();
    /// assert_eq!(nonempty_product, Some(3628800));
    /// ```
    fn product1<P>(mut self) -> Option<P>
    where
        Self: Sized,
        P: std::iter::Product<Self::Item>,
    {
        self.next().map(|first| once(first).chain(self).product())
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_unstable`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort the letters of the text in ascending order
    /// let text = "bdacfe";
    /// itertools::assert_equal(text.chars().sorted_unstable(),
    ///                         "abcdef".chars());
    /// ```
    fn sorted_unstable(self) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v = Vec::from_iter(self);
        v.sort_unstable();
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_unstable_by`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort people in descending order by age
    /// let people = vec![("Jane", 20), ("John", 18), ("Jill", 30), ("Jack", 27)];
    ///
    /// let oldest_people_first = people
    ///     .into_iter()
    ///     .sorted_unstable_by(|a, b| Ord::cmp(&b.1, &a.1))
    ///     .map(|(person, _age)| person);
    ///
    /// itertools::assert_equal(oldest_people_first,
    ///                         vec!["Jill", "Jack", "Jane", "John"]);
    /// ```
    fn sorted_unstable_by<F>(self, cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        let mut v = Vec::from_iter(self);
        v.sort_unstable_by(cmp);
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_unstable_by_key`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort people in descending order by age
    /// let people = vec![("Jane", 20), ("John", 18), ("Jill", 30), ("Jack", 27)];
    ///
    /// let oldest_people_first = people
    ///     .into_iter()
    ///     .sorted_unstable_by_key(|x| -x.1)
    ///     .map(|(person, _age)| person);
    ///
    /// itertools::assert_equal(oldest_people_first,
    ///                         vec!["Jill", "Jack", "Jane", "John"]);
    /// ```
    fn sorted_unstable_by_key<K, F>(self, f: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        let mut v = Vec::from_iter(self);
        v.sort_unstable_by_key(f);
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort the letters of the text in ascending order
    /// let text = "bdacfe";
    /// itertools::assert_equal(text.chars().sorted(),
    ///                         "abcdef".chars());
    /// ```
    fn sorted(self) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v = Vec::from_iter(self);
        v.sort();
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_by`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort people in descending order by age
    /// let people = vec![("Jane", 20), ("John", 18), ("Jill", 30), ("Jack", 30)];
    ///
    /// let oldest_people_first = people
    ///     .into_iter()
    ///     .sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
    ///     .map(|(person, _age)| person);
    ///
    /// itertools::assert_equal(oldest_people_first,
    ///                         vec!["Jill", "Jack", "Jane", "John"]);
    /// ```
    fn sorted_by<F>(self, cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        let mut v = Vec::from_iter(self);
        v.sort_by(cmp);
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_by_key`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort people in descending order by age
    /// let people = vec![("Jane", 20), ("John", 18), ("Jill", 30), ("Jack", 30)];
    ///
    /// let oldest_people_first = people
    ///     .into_iter()
    ///     .sorted_by_key(|x| -x.1)
    ///     .map(|(person, _age)| person);
    ///
    /// itertools::assert_equal(oldest_people_first,
    ///                         vec!["Jill", "Jack", "Jane", "John"]);
    /// ```
    fn sorted_by_key<K, F>(self, f: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        let mut v = Vec::from_iter(self);
        v.sort_by_key(f);
        v.into_iter()
    }
    /// Sort all iterator elements into a new iterator in ascending order. The key function is
    /// called exactly once per key.
    ///
    /// **Note:** This consumes the entire iterator, uses the
    /// [`slice::sort_by_cached_key`] method and returns the result as a new
    /// iterator that owns its elements.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // sort people in descending order by age
    /// let people = vec![("Jane", 20), ("John", 18), ("Jill", 30), ("Jack", 30)];
    ///
    /// let oldest_people_first = people
    ///     .into_iter()
    ///     .sorted_by_cached_key(|x| -x.1)
    ///     .map(|(person, _age)| person);
    ///
    /// itertools::assert_equal(oldest_people_first,
    ///                         vec!["Jill", "Jack", "Jane", "John"]);
    /// ```
    fn sorted_by_cached_key<K, F>(self, f: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        let mut v = Vec::from_iter(self);
        v.sort_by_cached_key(f);
        v.into_iter()
    }
    /// Sort the k smallest elements into a new iterator, in ascending order.
    ///
    /// **Note:** This consumes the entire iterator, and returns the result
    /// as a new iterator that owns its elements.  If the input contains
    /// less than k elements, the result is equivalent to `self.sorted()`.
    ///
    /// This is guaranteed to use `k * sizeof(Self::Item) + O(1)` memory
    /// and `O(n log k)` time, with `n` the number of elements in the input.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// **Note:** This is functionally-equivalent to `self.sorted().take(k)`
    /// but much more efficient.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest(5);
    ///
    /// itertools::assert_equal(five_smallest, 0..5);
    /// ```
    fn k_smallest(self, k: usize) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        use alloc::collections::BinaryHeap;
        if k == 0 {
            self.last();
            return Vec::new().into_iter();
        }
        if k == 1 {
            return self.min().into_iter().collect_vec().into_iter();
        }
        let mut iter = self.fuse();
        let mut heap: BinaryHeap<_> = iter.by_ref().take(k).collect();
        iter.for_each(|i| {
            if true {
                match (&heap.len(), &k) {
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
            if *heap.peek().unwrap() > i {
                *heap.peek_mut().unwrap() = i;
            }
        });
        heap.into_sorted_vec().into_iter()
    }
    /// Sort the k smallest elements into a new iterator using the provided comparison.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// This corresponds to `self.sorted_by(cmp).take(k)` in the same way that
    /// [`k_smallest`](Itertools::k_smallest) corresponds to `self.sorted().take(k)`,
    /// in both semantics and complexity.
    ///
    /// Particularly, a custom heap implementation ensures the comparison is not cloned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest_by(5, |a, b| (a % 7).cmp(&(b % 7)).then(a.cmp(b)));
    ///
    /// itertools::assert_equal(five_smallest, vec![0, 7, 14, 1, 8]);
    /// ```
    fn k_smallest_by<F>(self, k: usize, cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        k_smallest::k_smallest_general(self, k, cmp).into_iter()
    }
    /// Return the elements producing the k smallest outputs of the provided function.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// This corresponds to `self.sorted_by_key(key).take(k)` in the same way that
    /// [`k_smallest`](Itertools::k_smallest) corresponds to `self.sorted().take(k)`,
    /// in both semantics and complexity.
    ///
    /// Particularly, a custom heap implementation ensures the comparison is not cloned.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest_by_key(5, |n| (n % 7, *n));
    ///
    /// itertools::assert_equal(five_smallest, vec![0, 7, 14, 1, 8]);
    /// ```
    fn k_smallest_by_key<F, K>(self, k: usize, key: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: Ord,
    {
        self.k_smallest_by(k, k_smallest::key_to_cmp(key))
    }
    /// Sort the k smallest elements into a new iterator, in ascending order, relaxing the amount of memory required.
    ///
    /// **Note:** This consumes the entire iterator, and returns the result
    /// as a new iterator that owns its elements.  If the input contains
    /// less than k elements, the result is equivalent to `self.sorted()`.
    ///
    /// This is guaranteed to use `2 * k * sizeof(Self::Item) + O(1)` memory
    /// and `O(n + k log k)` time, with `n` the number of elements in the input,
    /// meaning it uses more memory than the minimum obtained by [`k_smallest`](Itertools::k_smallest)
    /// but achieves linear time in the number of elements.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// **Note:** This is functionally-equivalent to `self.sorted().take(k)`
    /// but much more efficient.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest_relaxed(5);
    ///
    /// itertools::assert_equal(five_smallest, 0..5);
    /// ```
    fn k_smallest_relaxed(self, k: usize) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.k_smallest_relaxed_by(k, Ord::cmp)
    }
    /// Sort the k smallest elements into a new iterator using the provided comparison, relaxing the amount of memory required.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// This corresponds to `self.sorted_by(cmp).take(k)` in the same way that
    /// [`k_smallest_relaxed`](Itertools::k_smallest_relaxed) corresponds to `self.sorted().take(k)`,
    /// in both semantics and complexity.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest_relaxed_by(5, |a, b| (a % 7).cmp(&(b % 7)).then(a.cmp(b)));
    ///
    /// itertools::assert_equal(five_smallest, vec![0, 7, 14, 1, 8]);
    /// ```
    fn k_smallest_relaxed_by<F>(self, k: usize, cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        k_smallest::k_smallest_relaxed_general(self, k, cmp).into_iter()
    }
    /// Return the elements producing the k smallest outputs of the provided function, relaxing the amount of memory required.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// This corresponds to `self.sorted_by_key(key).take(k)` in the same way that
    /// [`k_smallest_relaxed`](Itertools::k_smallest_relaxed) corresponds to `self.sorted().take(k)`,
    /// in both semantics and complexity.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_smallest = numbers
    ///     .into_iter()
    ///     .k_smallest_relaxed_by_key(5, |n| (n % 7, *n));
    ///
    /// itertools::assert_equal(five_smallest, vec![0, 7, 14, 1, 8]);
    /// ```
    fn k_smallest_relaxed_by_key<F, K>(self, k: usize, key: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: Ord,
    {
        self.k_smallest_relaxed_by(k, k_smallest::key_to_cmp(key))
    }
    /// Sort the k largest elements into a new iterator, in descending order.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// It is semantically equivalent to [`k_smallest`](Itertools::k_smallest)
    /// with a reversed `Ord`.
    /// However, this is implemented with a custom binary heap which does not
    /// have the same performance characteristics for very large `Self::Item`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest(5);
    ///
    /// itertools::assert_equal(five_largest, vec![14, 13, 12, 11, 10]);
    /// ```
    fn k_largest(self, k: usize) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.k_largest_by(k, Self::Item::cmp)
    }
    /// Sort the k largest elements into a new iterator using the provided comparison.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// Functionally equivalent to [`k_smallest_by`](Itertools::k_smallest_by)
    /// with a reversed `Ord`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest_by(5, |a, b| (a % 7).cmp(&(b % 7)).then(a.cmp(b)));
    ///
    /// itertools::assert_equal(five_largest, vec![13, 6, 12, 5, 11]);
    /// ```
    fn k_largest_by<F>(self, k: usize, mut cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        self.k_smallest_by(k, move |a, b| cmp(b, a))
    }
    /// Return the elements producing the k largest outputs of the provided function.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// Functionally equivalent to [`k_smallest_by_key`](Itertools::k_smallest_by_key)
    /// with a reversed `Ord`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest_by_key(5, |n| (n % 7, *n));
    ///
    /// itertools::assert_equal(five_largest, vec![13, 6, 12, 5, 11]);
    /// ```
    fn k_largest_by_key<F, K>(self, k: usize, key: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: Ord,
    {
        self.k_largest_by(k, k_smallest::key_to_cmp(key))
    }
    /// Sort the k largest elements into a new iterator, in descending order, relaxing the amount of memory required.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// It is semantically equivalent to [`k_smallest_relaxed`](Itertools::k_smallest_relaxed)
    /// with a reversed `Ord`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest_relaxed(5);
    ///
    /// itertools::assert_equal(five_largest, vec![14, 13, 12, 11, 10]);
    /// ```
    fn k_largest_relaxed(self, k: usize) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.k_largest_relaxed_by(k, Self::Item::cmp)
    }
    /// Sort the k largest elements into a new iterator using the provided comparison, relaxing the amount of memory required.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// Functionally equivalent to [`k_smallest_relaxed_by`](Itertools::k_smallest_relaxed_by)
    /// with a reversed `Ord`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest_relaxed_by(5, |a, b| (a % 7).cmp(&(b % 7)).then(a.cmp(b)));
    ///
    /// itertools::assert_equal(five_largest, vec![13, 6, 12, 5, 11]);
    /// ```
    fn k_largest_relaxed_by<F>(self, k: usize, mut cmp: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        self.k_smallest_relaxed_by(k, move |a, b| cmp(b, a))
    }
    /// Return the elements producing the k largest outputs of the provided function, relaxing the amount of memory required.
    ///
    /// The sorted iterator, if directly collected to a `Vec`, is converted
    /// without any extra copying or allocation cost.
    ///
    /// Functionally equivalent to [`k_smallest_relaxed_by_key`](Itertools::k_smallest_relaxed_by_key)
    /// with a reversed `Ord`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// // A random permutation of 0..15
    /// let numbers = vec![6, 9, 1, 14, 0, 4, 8, 7, 11, 2, 10, 3, 13, 12, 5];
    ///
    /// let five_largest = numbers
    ///     .into_iter()
    ///     .k_largest_relaxed_by_key(5, |n| (n % 7, *n));
    ///
    /// itertools::assert_equal(five_largest, vec![13, 6, 12, 5, 11]);
    /// ```
    fn k_largest_relaxed_by_key<F, K>(self, k: usize, key: F) -> VecIntoIter<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> K,
        K: Ord,
    {
        self.k_largest_relaxed_by(k, k_smallest::key_to_cmp(key))
    }
    /// Consumes the iterator and return an iterator of the last `n` elements.
    ///
    /// The iterator, if directly collected to a `VecDeque`, is converted
    /// without any extra copying or allocation cost.
    /// If directly collected to a `Vec`, it may need some data movement
    /// but no re-allocation.
    ///
    /// ```
    /// use itertools::{assert_equal, Itertools};
    ///
    /// let v = vec![5, 9, 8, 4, 2, 12, 0];
    /// assert_equal(v.iter().tail(3), &[2, 12, 0]);
    /// assert_equal(v.iter().tail(10), &v);
    ///
    /// assert_equal(v.iter().tail(1), v.iter().last());
    ///
    /// assert_equal((0..100).tail(10), 90..100);
    ///
    /// assert_equal((0..100).filter(|x| x % 3 == 0).tail(10), (72..100).step_by(3));
    /// ```
    ///
    /// For double ended iterators without side-effects, you might prefer
    /// `.rev().take(n).rev()` to have a similar result (lazy and non-allocating)
    /// without consuming the entire iterator.
    fn tail(self, n: usize) -> VecDequeIntoIter<Self::Item>
    where
        Self: Sized,
    {
        match n {
            0 => {
                self.last();
                VecDeque::new()
            }
            1 => self.last().into_iter().collect(),
            _ => {
                let (low, _) = self.size_hint();
                let mut iter = self.fuse().skip(low.saturating_sub(n));
                let mut data: Vec<_> = iter.by_ref().take(n).collect();
                let idx = iter
                    .fold(
                        0,
                        |i, val| {
                            if true {
                                match (&data.len(), &n) {
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
                            data[i] = val;
                            if i + 1 == n { 0 } else { i + 1 }
                        },
                    );
                let mut data = VecDeque::from(data);
                data.rotate_left(idx);
                data
            }
        }
            .into_iter()
    }
    /// Collect all iterator elements into one of two
    /// partitions. Unlike [`Iterator::partition`], each partition may
    /// have a distinct type.
    ///
    /// ```
    /// use itertools::{Itertools, Either};
    ///
    /// let successes_and_failures = vec![Ok(1), Err(false), Err(true), Ok(2)];
    ///
    /// let (successes, failures): (Vec<_>, Vec<_>) = successes_and_failures
    ///     .into_iter()
    ///     .partition_map(|r| {
    ///         match r {
    ///             Ok(v) => Either::Left(v),
    ///             Err(v) => Either::Right(v),
    ///         }
    ///     });
    ///
    /// assert_eq!(successes, [1, 2]);
    /// assert_eq!(failures, [false, true]);
    /// ```
    fn partition_map<A, B, F, L, R>(self, mut predicate: F) -> (A, B)
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Either<L, R>,
        A: Default + Extend<L>,
        B: Default + Extend<R>,
    {
        let mut left = A::default();
        let mut right = B::default();
        self.for_each(|val| match predicate(val) {
            Either::Left(v) => left.extend(Some(v)),
            Either::Right(v) => right.extend(Some(v)),
        });
        (left, right)
    }
    /// Partition a sequence of `Result`s into one list of all the `Ok` elements
    /// and another list of all the `Err` elements.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let successes_and_failures = vec![Ok(1), Err(false), Err(true), Ok(2)];
    ///
    /// let (successes, failures): (Vec<_>, Vec<_>) = successes_and_failures
    ///     .into_iter()
    ///     .partition_result();
    ///
    /// assert_eq!(successes, [1, 2]);
    /// assert_eq!(failures, [false, true]);
    /// ```
    fn partition_result<A, B, T, E>(self) -> (A, B)
    where
        Self: Iterator<Item = Result<T, E>> + Sized,
        A: Default + Extend<T>,
        B: Default + Extend<E>,
    {
        self.partition_map(|r| match r {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v),
        })
    }
    /// Return a `HashMap` of keys mapped to `Vec`s of values. Keys and values
    /// are taken from `(Key, Value)` tuple pairs yielded by the input iterator.
    ///
    /// Essentially a shorthand for `.into_grouping_map().collect::<Vec<_>>()`.
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let data = vec![(0, 10), (2, 12), (3, 13), (0, 20), (3, 33), (2, 42)];
    /// let lookup = data.into_iter().into_group_map();
    ///
    /// assert_eq!(lookup[&0], vec![10, 20]);
    /// assert_eq!(lookup.get(&1), None);
    /// assert_eq!(lookup[&2], vec![12, 42]);
    /// assert_eq!(lookup[&3], vec![13, 33]);
    /// ```
    fn into_group_map<K, V>(self) -> HashMap<K, Vec<V>>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Hash + Eq,
    {
        group_map::into_group_map(self)
    }
    /// Return a `HashMap` of keys mapped to `Vec`s of values. The key is specified
    /// in the closure. The values are taken from the input iterator.
    ///
    /// Essentially a shorthand for `.into_grouping_map_by(f).collect::<Vec<_>>()`.
    ///
    /// ```
    /// use itertools::Itertools;
    /// use std::collections::HashMap;
    ///
    /// let data = vec![(0, 10), (2, 12), (3, 13), (0, 20), (3, 33), (2, 42)];
    /// let lookup: HashMap<u32,Vec<(u32, u32)>> =
    ///     data.clone().into_iter().into_group_map_by(|a| a.0);
    ///
    /// assert_eq!(lookup[&0], vec![(0,10), (0,20)]);
    /// assert_eq!(lookup.get(&1), None);
    /// assert_eq!(lookup[&2], vec![(2,12), (2,42)]);
    /// assert_eq!(lookup[&3], vec![(3,13), (3,33)]);
    ///
    /// assert_eq!(
    ///     data.into_iter()
    ///         .into_group_map_by(|x| x.0)
    ///         .into_iter()
    ///         .map(|(key, values)| (key, values.into_iter().fold(0,|acc, (_,v)| acc + v )))
    ///         .collect::<HashMap<u32,u32>>()[&0],
    ///     30,
    /// );
    /// ```
    fn into_group_map_by<K, V, F>(self, f: F) -> HashMap<K, Vec<V>>
    where
        Self: Iterator<Item = V> + Sized,
        K: Hash + Eq,
        F: FnMut(&V) -> K,
    {
        group_map::into_group_map_by(self, f)
    }
    /// Constructs a `GroupingMap` to be used later with one of the efficient
    /// group-and-fold operations it allows to perform.
    ///
    /// The input iterator must yield item in the form of `(K, V)` where the
    /// value of type `K` will be used as key to identify the groups and the
    /// value of type `V` as value for the folding operation.
    ///
    /// See [`GroupingMap`] for more informations
    /// on what operations are available.
    fn into_grouping_map<K, V>(self) -> GroupingMap<Self>
    where
        Self: Iterator<Item = (K, V)> + Sized,
        K: Hash + Eq,
    {
        grouping_map::new(self)
    }
    /// Constructs a `GroupingMap` to be used later with one of the efficient
    /// group-and-fold operations it allows to perform.
    ///
    /// The values from this iterator will be used as values for the folding operation
    /// while the keys will be obtained from the values by calling `key_mapper`.
    ///
    /// See [`GroupingMap`] for more informations
    /// on what operations are available.
    fn into_grouping_map_by<K, V, F>(self, key_mapper: F) -> GroupingMapBy<Self, F>
    where
        Self: Iterator<Item = V> + Sized,
        K: Hash + Eq,
        F: FnMut(&V) -> K,
    {
        grouping_map::new(grouping_map::new_map_for_grouping(self, key_mapper))
    }
    /// Return all minimum elements of an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().min_set(), Vec::<&i32>::new());
    ///
    /// let a = [1];
    /// assert_eq!(a.iter().min_set(), vec![&1]);
    ///
    /// let a = [1, 2, 3, 4, 5];
    /// assert_eq!(a.iter().min_set(), vec![&1]);
    ///
    /// let a = [1, 1, 1, 1];
    /// assert_eq!(a.iter().min_set(), vec![&1, &1, &1, &1]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn min_set(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        extrema_set::min_set_impl(self, |_| (), |x, y, _, _| x.cmp(y))
    }
    /// Return all minimum elements of an iterator, as determined by
    /// the specified function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cmp::Ordering;
    /// use itertools::Itertools;
    ///
    /// let a: [(i32, i32); 0] = [];
    /// assert_eq!(a.iter().min_set_by(|_, _| Ordering::Equal), Vec::<&(i32, i32)>::new());
    ///
    /// let a = [(1, 2)];
    /// assert_eq!(a.iter().min_set_by(|&&(k1,_), &&(k2, _)| k1.cmp(&k2)), vec![&(1, 2)]);
    ///
    /// let a = [(1, 2), (2, 2), (3, 9), (4, 8), (5, 9)];
    /// assert_eq!(a.iter().min_set_by(|&&(_,k1), &&(_,k2)| k1.cmp(&k2)), vec![&(1, 2), &(2, 2)]);
    ///
    /// let a = [(1, 2), (1, 3), (1, 4), (1, 5)];
    /// assert_eq!(a.iter().min_set_by(|&&(k1,_), &&(k2, _)| k1.cmp(&k2)), vec![&(1, 2), &(1, 3), &(1, 4), &(1, 5)]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn min_set_by<F>(self, mut compare: F) -> Vec<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        extrema_set::min_set_impl(self, |_| (), |x, y, _, _| compare(x, y))
    }
    /// Return all minimum elements of an iterator, as determined by
    /// the specified function.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [(i32, i32); 0] = [];
    /// assert_eq!(a.iter().min_set_by_key(|_| ()), Vec::<&(i32, i32)>::new());
    ///
    /// let a = [(1, 2)];
    /// assert_eq!(a.iter().min_set_by_key(|&&(k,_)| k), vec![&(1, 2)]);
    ///
    /// let a = [(1, 2), (2, 2), (3, 9), (4, 8), (5, 9)];
    /// assert_eq!(a.iter().min_set_by_key(|&&(_, k)| k), vec![&(1, 2), &(2, 2)]);
    ///
    /// let a = [(1, 2), (1, 3), (1, 4), (1, 5)];
    /// assert_eq!(a.iter().min_set_by_key(|&&(k, _)| k), vec![&(1, 2), &(1, 3), &(1, 4), &(1, 5)]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn min_set_by_key<K, F>(self, key: F) -> Vec<Self::Item>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        extrema_set::min_set_impl(self, key, |_, _, kx, ky| kx.cmp(ky))
    }
    /// Return all maximum elements of an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().max_set(), Vec::<&i32>::new());
    ///
    /// let a = [1];
    /// assert_eq!(a.iter().max_set(), vec![&1]);
    ///
    /// let a = [1, 2, 3, 4, 5];
    /// assert_eq!(a.iter().max_set(), vec![&5]);
    ///
    /// let a = [1, 1, 1, 1];
    /// assert_eq!(a.iter().max_set(), vec![&1, &1, &1, &1]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn max_set(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        extrema_set::max_set_impl(self, |_| (), |x, y, _, _| x.cmp(y))
    }
    /// Return all maximum elements of an iterator, as determined by
    /// the specified function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cmp::Ordering;
    /// use itertools::Itertools;
    ///
    /// let a: [(i32, i32); 0] = [];
    /// assert_eq!(a.iter().max_set_by(|_, _| Ordering::Equal), Vec::<&(i32, i32)>::new());
    ///
    /// let a = [(1, 2)];
    /// assert_eq!(a.iter().max_set_by(|&&(k1,_), &&(k2, _)| k1.cmp(&k2)), vec![&(1, 2)]);
    ///
    /// let a = [(1, 2), (2, 2), (3, 9), (4, 8), (5, 9)];
    /// assert_eq!(a.iter().max_set_by(|&&(_,k1), &&(_,k2)| k1.cmp(&k2)), vec![&(3, 9), &(5, 9)]);
    ///
    /// let a = [(1, 2), (1, 3), (1, 4), (1, 5)];
    /// assert_eq!(a.iter().max_set_by(|&&(k1,_), &&(k2, _)| k1.cmp(&k2)), vec![&(1, 2), &(1, 3), &(1, 4), &(1, 5)]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn max_set_by<F>(self, mut compare: F) -> Vec<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        extrema_set::max_set_impl(self, |_| (), |x, y, _, _| compare(x, y))
    }
    /// Return all maximum elements of an iterator, as determined by
    /// the specified function.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [(i32, i32); 0] = [];
    /// assert_eq!(a.iter().max_set_by_key(|_| ()), Vec::<&(i32, i32)>::new());
    ///
    /// let a = [(1, 2)];
    /// assert_eq!(a.iter().max_set_by_key(|&&(k,_)| k), vec![&(1, 2)]);
    ///
    /// let a = [(1, 2), (2, 2), (3, 9), (4, 8), (5, 9)];
    /// assert_eq!(a.iter().max_set_by_key(|&&(_, k)| k), vec![&(3, 9), &(5, 9)]);
    ///
    /// let a = [(1, 2), (1, 3), (1, 4), (1, 5)];
    /// assert_eq!(a.iter().max_set_by_key(|&&(k, _)| k), vec![&(1, 2), &(1, 3), &(1, 4), &(1, 5)]);
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn max_set_by_key<K, F>(self, key: F) -> Vec<Self::Item>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        extrema_set::max_set_impl(self, key, |_, _, kx, ky| kx.cmp(ky))
    }
    /// Return the minimum and maximum elements in the iterator.
    ///
    /// The return type `MinMaxResult` is an enum of three variants:
    ///
    /// - `NoElements` if the iterator is empty.
    /// - `OneElement(x)` if the iterator has exactly one element.
    /// - `MinMax(x, y)` is returned otherwise, where `x <= y`. Two
    ///    values are equal if and only if there is more than one
    ///    element in the iterator and all elements are equal.
    ///
    /// On an iterator of length `n`, `minmax` does `1.5 * n` comparisons,
    /// and so is faster than calling `min` and `max` separately which does
    /// `2 * n` comparisons.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::MinMaxResult::{NoElements, OneElement, MinMax};
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().minmax(), NoElements);
    ///
    /// let a = [1];
    /// assert_eq!(a.iter().minmax(), OneElement(&1));
    ///
    /// let a = [1, 2, 3, 4, 5];
    /// assert_eq!(a.iter().minmax(), MinMax(&1, &5));
    ///
    /// let a = [1, 1, 1, 1];
    /// assert_eq!(a.iter().minmax(), MinMax(&1, &1));
    /// ```
    ///
    /// The elements can be floats but no particular result is guaranteed
    /// if an element is NaN.
    fn minmax(self) -> MinMaxResult<Self::Item>
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        minmax::minmax_impl(self, |_| (), |x, y, _, _| x < y)
    }
    /// Return the minimum and maximum element of an iterator, as determined by
    /// the specified function.
    ///
    /// The return value is a variant of [`MinMaxResult`] like for [`.minmax()`](Itertools::minmax).
    ///
    /// For the minimum, the first minimal element is returned.  For the maximum,
    /// the last maximal element wins.  This matches the behavior of the standard
    /// [`Iterator::min`] and [`Iterator::max`] methods.
    ///
    /// The keys can be floats but no particular result is guaranteed
    /// if a key is NaN.
    fn minmax_by_key<K, F>(self, key: F) -> MinMaxResult<Self::Item>
    where
        Self: Sized,
        K: PartialOrd,
        F: FnMut(&Self::Item) -> K,
    {
        minmax::minmax_impl(self, key, |_, _, xk, yk| xk < yk)
    }
    /// Return the minimum and maximum element of an iterator, as determined by
    /// the specified comparison function.
    ///
    /// The return value is a variant of [`MinMaxResult`] like for [`.minmax()`](Itertools::minmax).
    ///
    /// For the minimum, the first minimal element is returned.  For the maximum,
    /// the last maximal element wins.  This matches the behavior of the standard
    /// [`Iterator::min`] and [`Iterator::max`] methods.
    fn minmax_by<F>(self, mut compare: F) -> MinMaxResult<Self::Item>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        minmax::minmax_impl(self, |_| (), |x, y, _, _| Ordering::Less == compare(x, y))
    }
    /// Return the position of the maximum element in the iterator.
    ///
    /// If several elements are equally maximum, the position of the
    /// last of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_max(), None);
    ///
    /// let a = [-3, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_max(), Some(3));
    ///
    /// let a = [1, 1, -1, -1];
    /// assert_eq!(a.iter().position_max(), Some(1));
    /// ```
    fn position_max(self) -> Option<usize>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.enumerate().max_by(|x, y| Ord::cmp(&x.1, &y.1)).map(|x| x.0)
    }
    /// Return the position of the maximum element in the iterator, as
    /// determined by the specified function.
    ///
    /// If several elements are equally maximum, the position of the
    /// last of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_max_by_key(|x| x.abs()), None);
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_max_by_key(|x| x.abs()), Some(4));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_max_by_key(|x| x.abs()), Some(3));
    /// ```
    fn position_max_by_key<K, F>(self, mut key: F) -> Option<usize>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.enumerate().max_by(|x, y| Ord::cmp(&key(&x.1), &key(&y.1))).map(|x| x.0)
    }
    /// Return the position of the maximum element in the iterator, as
    /// determined by the specified comparison function.
    ///
    /// If several elements are equally maximum, the position of the
    /// last of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_max_by(|x, y| x.cmp(y)), None);
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_max_by(|x, y| x.cmp(y)), Some(3));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_max_by(|x, y| x.cmp(y)), Some(1));
    /// ```
    fn position_max_by<F>(self, mut compare: F) -> Option<usize>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        self.enumerate().max_by(|x, y| compare(&x.1, &y.1)).map(|x| x.0)
    }
    /// Return the position of the minimum element in the iterator.
    ///
    /// If several elements are equally minimum, the position of the
    /// first of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_min(), None);
    ///
    /// let a = [-3, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_min(), Some(4));
    ///
    /// let a = [1, 1, -1, -1];
    /// assert_eq!(a.iter().position_min(), Some(2));
    /// ```
    fn position_min(self) -> Option<usize>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.enumerate().min_by(|x, y| Ord::cmp(&x.1, &y.1)).map(|x| x.0)
    }
    /// Return the position of the minimum element in the iterator, as
    /// determined by the specified function.
    ///
    /// If several elements are equally minimum, the position of the
    /// first of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_min_by_key(|x| x.abs()), None);
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_min_by_key(|x| x.abs()), Some(1));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_min_by_key(|x| x.abs()), Some(0));
    /// ```
    fn position_min_by_key<K, F>(self, mut key: F) -> Option<usize>
    where
        Self: Sized,
        K: Ord,
        F: FnMut(&Self::Item) -> K,
    {
        self.enumerate().min_by(|x, y| Ord::cmp(&key(&x.1), &key(&y.1))).map(|x| x.0)
    }
    /// Return the position of the minimum element in the iterator, as
    /// determined by the specified comparison function.
    ///
    /// If several elements are equally minimum, the position of the
    /// first of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_min_by(|x, y| x.cmp(y)), None);
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_min_by(|x, y| x.cmp(y)), Some(4));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_min_by(|x, y| x.cmp(y)), Some(2));
    /// ```
    fn position_min_by<F>(self, mut compare: F) -> Option<usize>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        self.enumerate().min_by(|x, y| compare(&x.1, &y.1)).map(|x| x.0)
    }
    /// Return the positions of the minimum and maximum elements in
    /// the iterator.
    ///
    /// The return type [`MinMaxResult`] is an enum of three variants:
    ///
    /// - `NoElements` if the iterator is empty.
    /// - `OneElement(xpos)` if the iterator has exactly one element.
    /// - `MinMax(xpos, ypos)` is returned otherwise, where the
    ///    element at `xpos` ≤ the element at `ypos`. While the
    ///    referenced elements themselves may be equal, `xpos` cannot
    ///    be equal to `ypos`.
    ///
    /// On an iterator of length `n`, `position_minmax` does `1.5 * n`
    /// comparisons, and so is faster than calling `position_min` and
    /// `position_max` separately which does `2 * n` comparisons.
    ///
    /// For the minimum, if several elements are equally minimum, the
    /// position of the first of them is returned. For the maximum, if
    /// several elements are equally maximum, the position of the last
    /// of them is returned.
    ///
    /// The elements can be floats but no particular result is
    /// guaranteed if an element is NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::MinMaxResult::{NoElements, OneElement, MinMax};
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_minmax(), NoElements);
    ///
    /// let a = [10];
    /// assert_eq!(a.iter().position_minmax(), OneElement(0));
    ///
    /// let a = [-3, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_minmax(), MinMax(4, 3));
    ///
    /// let a = [1, 1, -1, -1];
    /// assert_eq!(a.iter().position_minmax(), MinMax(2, 1));
    /// ```
    fn position_minmax(self) -> MinMaxResult<usize>
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        use crate::MinMaxResult::{MinMax, NoElements, OneElement};
        match minmax::minmax_impl(self.enumerate(), |_| (), |x, y, _, _| x.1 < y.1) {
            NoElements => NoElements,
            OneElement(x) => OneElement(x.0),
            MinMax(x, y) => MinMax(x.0, y.0),
        }
    }
    /// Return the postions of the minimum and maximum elements of an
    /// iterator, as determined by the specified function.
    ///
    /// The return value is a variant of [`MinMaxResult`] like for
    /// [`position_minmax`].
    ///
    /// For the minimum, if several elements are equally minimum, the
    /// position of the first of them is returned. For the maximum, if
    /// several elements are equally maximum, the position of the last
    /// of them is returned.
    ///
    /// The keys can be floats but no particular result is guaranteed
    /// if a key is NaN.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::MinMaxResult::{NoElements, OneElement, MinMax};
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_minmax_by_key(|x| x.abs()), NoElements);
    ///
    /// let a = [10_i32];
    /// assert_eq!(a.iter().position_minmax_by_key(|x| x.abs()), OneElement(0));
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_minmax_by_key(|x| x.abs()), MinMax(1, 4));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_minmax_by_key(|x| x.abs()), MinMax(0, 3));
    /// ```
    ///
    /// [`position_minmax`]: Self::position_minmax
    fn position_minmax_by_key<K, F>(self, mut key: F) -> MinMaxResult<usize>
    where
        Self: Sized,
        K: PartialOrd,
        F: FnMut(&Self::Item) -> K,
    {
        use crate::MinMaxResult::{MinMax, NoElements, OneElement};
        match self.enumerate().minmax_by_key(|e| key(&e.1)) {
            NoElements => NoElements,
            OneElement(x) => OneElement(x.0),
            MinMax(x, y) => MinMax(x.0, y.0),
        }
    }
    /// Return the postions of the minimum and maximum elements of an
    /// iterator, as determined by the specified comparison function.
    ///
    /// The return value is a variant of [`MinMaxResult`] like for
    /// [`position_minmax`].
    ///
    /// For the minimum, if several elements are equally minimum, the
    /// position of the first of them is returned. For the maximum, if
    /// several elements are equally maximum, the position of the last
    /// of them is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use itertools::Itertools;
    /// use itertools::MinMaxResult::{NoElements, OneElement, MinMax};
    ///
    /// let a: [i32; 0] = [];
    /// assert_eq!(a.iter().position_minmax_by(|x, y| x.cmp(y)), NoElements);
    ///
    /// let a = [10_i32];
    /// assert_eq!(a.iter().position_minmax_by(|x, y| x.cmp(y)), OneElement(0));
    ///
    /// let a = [-3_i32, 0, 1, 5, -10];
    /// assert_eq!(a.iter().position_minmax_by(|x, y| x.cmp(y)), MinMax(4, 3));
    ///
    /// let a = [1_i32, 1, -1, -1];
    /// assert_eq!(a.iter().position_minmax_by(|x, y| x.cmp(y)), MinMax(2, 1));
    /// ```
    ///
    /// [`position_minmax`]: Self::position_minmax
    fn position_minmax_by<F>(self, mut compare: F) -> MinMaxResult<usize>
    where
        Self: Sized,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        use crate::MinMaxResult::{MinMax, NoElements, OneElement};
        match self.enumerate().minmax_by(|x, y| compare(&x.1, &y.1)) {
            NoElements => NoElements,
            OneElement(x) => OneElement(x.0),
            MinMax(x, y) => MinMax(x.0, y.0),
        }
    }
    /// If the iterator yields exactly one element, that element will be returned, otherwise
    /// an error will be returned containing an iterator that has the same output as the input
    /// iterator.
    ///
    /// This provides an additional layer of validation over just calling `Iterator::next()`.
    /// If your assumption that there should only be one element yielded is false this provides
    /// the opportunity to detect and handle that, preventing errors at a distance.
    ///
    /// # Examples
    /// ```
    /// use itertools::Itertools;
    ///
    /// assert_eq!((0..10).filter(|&x| x == 2).exactly_one().unwrap(), 2);
    /// assert!((0..10).filter(|&x| x > 1 && x < 4).exactly_one().unwrap_err().eq(2..4));
    /// assert!((0..10).filter(|&x| x > 1 && x < 5).exactly_one().unwrap_err().eq(2..5));
    /// assert!((0..10).filter(|&_| false).exactly_one().unwrap_err().eq(0..0));
    /// ```
    fn exactly_one(mut self) -> Result<Self::Item, ExactlyOneError<Self>>
    where
        Self: Sized,
    {
        match self.next() {
            Some(first) => {
                match self.next() {
                    Some(second) => {
                        Err(
                            ExactlyOneError::new(
                                Some(Either::Left([first, second])),
                                self,
                            ),
                        )
                    }
                    None => Ok(first),
                }
            }
            None => Err(ExactlyOneError::new(None, self)),
        }
    }
    /// If the iterator yields no elements, `Ok(None)` will be returned. If the iterator yields
    /// exactly one element, that element will be returned, otherwise an error will be returned
    /// containing an iterator that has the same output as the input iterator.
    ///
    /// This provides an additional layer of validation over just calling `Iterator::next()`.
    /// If your assumption that there should be at most one element yielded is false this provides
    /// the opportunity to detect and handle that, preventing errors at a distance.
    ///
    /// # Examples
    /// ```
    /// use itertools::Itertools;
    ///
    /// assert_eq!((0..10).filter(|&x| x == 2).at_most_one().unwrap(), Some(2));
    /// assert!((0..10).filter(|&x| x > 1 && x < 4).at_most_one().unwrap_err().eq(2..4));
    /// assert!((0..10).filter(|&x| x > 1 && x < 5).at_most_one().unwrap_err().eq(2..5));
    /// assert_eq!((0..10).filter(|&_| false).at_most_one().unwrap(), None);
    /// ```
    fn at_most_one(mut self) -> Result<Option<Self::Item>, ExactlyOneError<Self>>
    where
        Self: Sized,
    {
        match self.next() {
            Some(first) => {
                match self.next() {
                    Some(second) => {
                        Err(
                            ExactlyOneError::new(
                                Some(Either::Left([first, second])),
                                self,
                            ),
                        )
                    }
                    None => Ok(Some(first)),
                }
            }
            None => Ok(None),
        }
    }
    /// An iterator adaptor that allows the user to peek at multiple `.next()`
    /// values without advancing the base iterator.
    ///
    /// # Examples
    /// ```
    /// use itertools::Itertools;
    ///
    /// let mut iter = (0..10).multipeek();
    /// assert_eq!(iter.peek(), Some(&0));
    /// assert_eq!(iter.peek(), Some(&1));
    /// assert_eq!(iter.peek(), Some(&2));
    /// assert_eq!(iter.next(), Some(0));
    /// assert_eq!(iter.peek(), Some(&1));
    /// ```
    fn multipeek(self) -> MultiPeek<Self>
    where
        Self: Sized,
    {
        multipeek_impl::multipeek(self)
    }
    /// Collect the items in this iterator and return a `HashMap` which
    /// contains each item that appears in the iterator and the number
    /// of times it appears.
    ///
    /// # Examples
    /// ```
    /// # use itertools::Itertools;
    /// let counts = [1, 1, 1, 3, 3, 5].iter().counts();
    /// assert_eq!(counts[&1], 3);
    /// assert_eq!(counts[&3], 2);
    /// assert_eq!(counts[&5], 1);
    /// assert_eq!(counts.get(&0), None);
    /// ```
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self: Sized,
        Self::Item: Eq + Hash,
    {
        let mut counts = HashMap::new();
        self.for_each(|item| *counts.entry(item).or_default() += 1);
        counts
    }
    /// Collect the items in this iterator and return a `HashMap` which
    /// contains each item that appears in the iterator and the number
    /// of times it appears,
    /// determining identity using a keying function.
    ///
    /// ```
    /// # use itertools::Itertools;
    /// struct Character {
    ///   first_name: &'static str,
    ///   # #[allow(dead_code)]
    ///   last_name:  &'static str,
    /// }
    ///
    /// let characters =
    ///     vec![
    ///         Character { first_name: "Amy",   last_name: "Pond"      },
    ///         Character { first_name: "Amy",   last_name: "Wong"      },
    ///         Character { first_name: "Amy",   last_name: "Santiago"  },
    ///         Character { first_name: "James", last_name: "Bond"      },
    ///         Character { first_name: "James", last_name: "Sullivan"  },
    ///         Character { first_name: "James", last_name: "Norington" },
    ///         Character { first_name: "James", last_name: "Kirk"      },
    ///     ];
    ///
    /// let first_name_frequency =
    ///     characters
    ///         .into_iter()
    ///         .counts_by(|c| c.first_name);
    ///
    /// assert_eq!(first_name_frequency["Amy"], 3);
    /// assert_eq!(first_name_frequency["James"], 4);
    /// assert_eq!(first_name_frequency.contains_key("Asha"), false);
    /// ```
    fn counts_by<K, F>(self, f: F) -> HashMap<K, usize>
    where
        Self: Sized,
        K: Eq + Hash,
        F: FnMut(Self::Item) -> K,
    {
        self.map(f).counts()
    }
    /// Converts an iterator of tuples into a tuple of containers.
    ///
    /// It consumes an entire iterator of n-ary tuples, producing `n` collections, one for each
    /// column.
    ///
    /// This function is, in some sense, the opposite of [`multizip`].
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// let inputs = vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)];
    ///
    /// let (a, b, c): (Vec<_>, Vec<_>, Vec<_>) = inputs
    ///     .into_iter()
    ///     .multiunzip();
    ///
    /// assert_eq!(a, vec![1, 4, 7]);
    /// assert_eq!(b, vec![2, 5, 8]);
    /// assert_eq!(c, vec![3, 6, 9]);
    /// ```
    fn multiunzip<FromI>(self) -> FromI
    where
        Self: Sized + MultiUnzip<FromI>,
    {
        MultiUnzip::multiunzip(self)
    }
    /// Returns the length of the iterator if one exists.
    /// Otherwise return `self.size_hint()`.
    ///
    /// Fallible [`ExactSizeIterator::len`].
    ///
    /// Inherits guarantees and restrictions from [`Iterator::size_hint`].
    ///
    /// ```
    /// use itertools::Itertools;
    ///
    /// assert_eq!([0; 10].iter().try_len(), Ok(10));
    /// assert_eq!((10..15).try_len(), Ok(5));
    /// assert_eq!((15..10).try_len(), Ok(0));
    /// assert_eq!((10..).try_len(), Err((usize::MAX, None)));
    /// assert_eq!((10..15).filter(|x| x % 2 == 0).try_len(), Err((0, Some(5))));
    /// ```
    fn try_len(&self) -> Result<usize, size_hint::SizeHint> {
        let sh = self.size_hint();
        match sh {
            (lo, Some(hi)) if lo == hi => Ok(lo),
            _ => Err(sh),
        }
    }
}
impl<T> Itertools for T
where
    T: Iterator + ?Sized,
{}
/// Return `true` if both iterables produce equal sequences
/// (elements pairwise equal and sequences of the same length),
/// `false` otherwise.
///
/// [`IntoIterator`] enabled version of [`Iterator::eq`].
///
/// ```
/// assert!(itertools::equal(vec![1, 2, 3], 1..4));
/// assert!(!itertools::equal(&[0, 0], &[0, 0, 0]));
/// ```
pub fn equal<I, J>(a: I, b: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    a.into_iter().eq(b)
}
/// Assert that two iterables produce equal sequences, with the same
/// semantics as [`equal(a, b)`](equal).
///
/// **Panics** on assertion failure with a message that shows the
/// two different elements and the iteration index.
///
/// ```should_panic
/// # use itertools::assert_equal;
/// assert_equal("exceed".split('c'), "excess".split('c'));
/// // ^PANIC: panicked at 'Failed assertion Some("eed") == Some("ess") for iteration 1'.
/// ```
#[track_caller]
pub fn assert_equal<I, J>(a: I, b: J)
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: fmt::Debug + PartialEq<J::Item>,
    J::Item: fmt::Debug,
{
    let mut ia = a.into_iter();
    let mut ib = b.into_iter();
    let mut i: usize = 0;
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return,
            (a, b) => {
                let equal = match (&a, &b) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                };
                if !equal {
                    {
                        ::std::rt::panic_fmt(
                            format_args!(
                                "Failed assertion {1:?} == {2:?} for iteration {0}",
                                i,
                                a,
                                b,
                            ),
                        );
                    }
                }
                i += 1;
            }
        }
    }
}
/// Partition a sequence using predicate `pred` so that elements
/// that map to `true` are placed before elements which map to `false`.
///
/// The order within the partitions is arbitrary.
///
/// Return the index of the split point.
///
/// ```
/// use itertools::partition;
///
/// # // use repeated numbers to not promise any ordering
/// let mut data = [7, 1, 1, 7, 1, 1, 7];
/// let split_index = partition(&mut data, |elt| *elt >= 3);
///
/// assert_eq!(data, [7, 7, 7, 1, 1, 1, 1]);
/// assert_eq!(split_index, 3);
/// ```
pub fn partition<'a, A: 'a, I, F>(iter: I, mut pred: F) -> usize
where
    I: IntoIterator<Item = &'a mut A>,
    I::IntoIter: DoubleEndedIterator,
    F: FnMut(&A) -> bool,
{
    let mut split_index = 0;
    let mut iter = iter.into_iter();
    while let Some(front) = iter.next() {
        if !pred(front) {
            match iter.rfind(|back| pred(back)) {
                Some(back) => std::mem::swap(front, back),
                None => break,
            }
        }
        split_index += 1;
    }
    split_index
}
/// An enum used for controlling the execution of `fold_while`.
///
/// See [`.fold_while()`](Itertools::fold_while) for more information.
pub enum FoldWhile<T> {
    /// Continue folding with this value
    Continue(T),
    /// Fold is complete and will return this value
    Done(T),
}
#[automatically_derived]
impl<T: ::core::marker::Copy> ::core::marker::Copy for FoldWhile<T> {}
#[automatically_derived]
impl<T: ::core::clone::Clone> ::core::clone::Clone for FoldWhile<T> {
    #[inline]
    fn clone(&self) -> FoldWhile<T> {
        match self {
            FoldWhile::Continue(__self_0) => {
                FoldWhile::Continue(::core::clone::Clone::clone(__self_0))
            }
            FoldWhile::Done(__self_0) => {
                FoldWhile::Done(::core::clone::Clone::clone(__self_0))
            }
        }
    }
}
#[automatically_derived]
impl<T: ::core::fmt::Debug> ::core::fmt::Debug for FoldWhile<T> {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            FoldWhile::Continue(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "Continue",
                    &__self_0,
                )
            }
            FoldWhile::Done(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Done", &__self_0)
            }
        }
    }
}
#[automatically_derived]
impl<T: ::core::cmp::Eq> ::core::cmp::Eq for FoldWhile<T> {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<T>;
    }
}
#[automatically_derived]
impl<T> ::core::marker::StructuralPartialEq for FoldWhile<T> {}
#[automatically_derived]
impl<T: ::core::cmp::PartialEq> ::core::cmp::PartialEq for FoldWhile<T> {
    #[inline]
    fn eq(&self, other: &FoldWhile<T>) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (FoldWhile::Continue(__self_0), FoldWhile::Continue(__arg1_0)) => {
                    __self_0 == __arg1_0
                }
                (FoldWhile::Done(__self_0), FoldWhile::Done(__arg1_0)) => {
                    __self_0 == __arg1_0
                }
                _ => unsafe { ::core::intrinsics::unreachable() }
            }
    }
}
impl<T> FoldWhile<T> {
    /// Return the value in the continue or done.
    pub fn into_inner(self) -> T {
        match self {
            Self::Continue(x) | Self::Done(x) => x,
        }
    }
    /// Return true if `self` is `Done`, false if it is `Continue`.
    pub fn is_done(&self) -> bool {
        match *self {
            Self::Continue(_) => false,
            Self::Done(_) => true,
        }
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(
        &[
            &test_checked_binomial,
            &push_4,
            &tracked_drop,
            &zero_len_push,
            &zero_len_take,
            &mul_size_hints,
        ],
    )
}
