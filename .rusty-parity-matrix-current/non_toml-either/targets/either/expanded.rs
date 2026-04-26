#![feature(prelude_import)]
//! The enum [`Either`] with variants `Left` and `Right` is a general purpose
//! sum type with two cases.
//!
//! [`Either`]: enum.Either.html
//!
//! **Crate features:**
//!
//! * `"use_std"`
//!   Enabled by default. Disable to make the library `#![no_std]`.
//!
//! * `"serde"`
//!   Disabled by default. Enable to `#[derive(Serialize, Deserialize)]` for `Either`
//!
#![doc(html_root_url = "https://docs.rs/either/1/")]
#![no_std]
extern crate core;
#[prelude_import]
use core::prelude::rust_2018::*;
extern crate std;
use core::convert::{AsMut, AsRef};
use core::fmt;
use core::future::Future;
use core::ops::Deref;
use core::ops::DerefMut;
use core::pin::Pin;
use std::error::Error;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
pub use crate::Either::{Left, Right};
/// The enum `Either` with variants `Left` and `Right` is a general purpose
/// sum type with two cases.
///
/// The `Either` type is symmetric and treats its variants the same way, without
/// preference.
/// (For representing success or error, use the regular `Result` enum instead.)
pub enum Either<L, R> {
    /// A value of type `L`.
    Left(L),
    /// A value of type `R`.
    Right(R),
}
#[automatically_derived]
impl<L: ::core::marker::Copy, R: ::core::marker::Copy> ::core::marker::Copy
for Either<L, R> {}
#[automatically_derived]
impl<L, R> ::core::marker::StructuralPartialEq for Either<L, R> {}
#[automatically_derived]
impl<L: ::core::cmp::PartialEq, R: ::core::cmp::PartialEq> ::core::cmp::PartialEq
for Either<L, R> {
    #[inline]
    fn eq(&self, other: &Either<L, R>) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (Either::Left(__self_0), Either::Left(__arg1_0)) => __self_0 == __arg1_0,
                (Either::Right(__self_0), Either::Right(__arg1_0)) => {
                    __self_0 == __arg1_0
                }
                _ => unsafe { ::core::intrinsics::unreachable() }
            }
    }
}
#[automatically_derived]
impl<L: ::core::cmp::Eq, R: ::core::cmp::Eq> ::core::cmp::Eq for Either<L, R> {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<L>;
        let _: ::core::cmp::AssertParamIsEq<R>;
    }
}
#[automatically_derived]
impl<L: ::core::cmp::PartialOrd, R: ::core::cmp::PartialOrd> ::core::cmp::PartialOrd
for Either<L, R> {
    #[inline]
    fn partial_cmp(
        &self,
        other: &Either<L, R>,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        match (self, other) {
            (Either::Left(__self_0), Either::Left(__arg1_0)) => {
                ::core::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0)
            }
            (Either::Right(__self_0), Either::Right(__arg1_0)) => {
                ::core::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0)
            }
            _ => ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr),
        }
    }
}
#[automatically_derived]
impl<L: ::core::cmp::Ord, R: ::core::cmp::Ord> ::core::cmp::Ord for Either<L, R> {
    #[inline]
    fn cmp(&self, other: &Either<L, R>) -> ::core::cmp::Ordering {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        match ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr) {
            ::core::cmp::Ordering::Equal => {
                match (self, other) {
                    (Either::Left(__self_0), Either::Left(__arg1_0)) => {
                        ::core::cmp::Ord::cmp(__self_0, __arg1_0)
                    }
                    (Either::Right(__self_0), Either::Right(__arg1_0)) => {
                        ::core::cmp::Ord::cmp(__self_0, __arg1_0)
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
            }
            cmp => cmp,
        }
    }
}
#[automatically_derived]
impl<L: ::core::hash::Hash, R: ::core::hash::Hash> ::core::hash::Hash for Either<L, R> {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        ::core::hash::Hash::hash(&__self_discr, state);
        match self {
            Either::Left(__self_0) => ::core::hash::Hash::hash(__self_0, state),
            Either::Right(__self_0) => ::core::hash::Hash::hash(__self_0, state),
        }
    }
}
#[automatically_derived]
impl<L: ::core::fmt::Debug, R: ::core::fmt::Debug> ::core::fmt::Debug for Either<L, R> {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Either::Left(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Left", &__self_0)
            }
            Either::Right(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Right", &__self_0)
            }
        }
    }
}
mod iterator {
    use super::{for_both, Either, Left, Right};
    use core::iter;
    /// Iterator that maps left or right iterators to corresponding `Either`-wrapped items.
    ///
    /// This struct is created by the [`Either::factor_into_iter`],
    /// [`factor_iter`][Either::factor_iter],
    /// and [`factor_iter_mut`][Either::factor_iter_mut] methods.
    pub struct IterEither<L, R> {
        inner: Either<L, R>,
    }
    #[automatically_derived]
    impl<L: ::core::clone::Clone, R: ::core::clone::Clone> ::core::clone::Clone
    for IterEither<L, R> {
        #[inline]
        fn clone(&self) -> IterEither<L, R> {
            IterEither {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    #[automatically_derived]
    impl<L: ::core::fmt::Debug, R: ::core::fmt::Debug> ::core::fmt::Debug
    for IterEither<L, R> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "IterEither",
                "inner",
                &&self.inner,
            )
        }
    }
    impl<L, R> IterEither<L, R> {
        pub(crate) fn new(inner: Either<L, R>) -> Self {
            IterEither { inner }
        }
    }
    impl<L, R, A> Extend<A> for Either<L, R>
    where
        L: Extend<A>,
        R: Extend<A>,
    {
        fn extend<T>(&mut self, iter: T)
        where
            T: IntoIterator<Item = A>,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.extend(iter),
                crate::Either::Right(ref mut inner) => inner.extend(iter),
            }
        }
    }
    /// `Either<L, R>` is an iterator if both `L` and `R` are iterators.
    impl<L, R> Iterator for Either<L, R>
    where
        L: Iterator,
        R: Iterator<Item = L::Item>,
    {
        type Item = L::Item;
        fn next(&mut self) -> Option<Self::Item> {
            match *self {
                crate::Either::Left(ref mut inner) => inner.next(),
                crate::Either::Right(ref mut inner) => inner.next(),
            }
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            match *self {
                crate::Either::Left(ref inner) => inner.size_hint(),
                crate::Either::Right(ref inner) => inner.size_hint(),
            }
        }
        fn fold<Acc, G>(self, init: Acc, f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            match self {
                crate::Either::Left(inner) => inner.fold(init, f),
                crate::Either::Right(inner) => inner.fold(init, f),
            }
        }
        fn for_each<F>(self, f: F)
        where
            F: FnMut(Self::Item),
        {
            match self {
                crate::Either::Left(inner) => inner.for_each(f),
                crate::Either::Right(inner) => inner.for_each(f),
            }
        }
        fn count(self) -> usize {
            match self {
                crate::Either::Left(inner) => inner.count(),
                crate::Either::Right(inner) => inner.count(),
            }
        }
        fn last(self) -> Option<Self::Item> {
            match self {
                crate::Either::Left(inner) => inner.last(),
                crate::Either::Right(inner) => inner.last(),
            }
        }
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            match *self {
                crate::Either::Left(ref mut inner) => inner.nth(n),
                crate::Either::Right(ref mut inner) => inner.nth(n),
            }
        }
        fn collect<B>(self) -> B
        where
            B: iter::FromIterator<Self::Item>,
        {
            match self {
                crate::Either::Left(inner) => inner.collect(),
                crate::Either::Right(inner) => inner.collect(),
            }
        }
        fn partition<B, F>(self, f: F) -> (B, B)
        where
            B: Default + Extend<Self::Item>,
            F: FnMut(&Self::Item) -> bool,
        {
            match self {
                crate::Either::Left(inner) => inner.partition(f),
                crate::Either::Right(inner) => inner.partition(f),
            }
        }
        fn all<F>(&mut self, f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.all(f),
                crate::Either::Right(ref mut inner) => inner.all(f),
            }
        }
        fn any<F>(&mut self, f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.any(f),
                crate::Either::Right(ref mut inner) => inner.any(f),
            }
        }
        fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
        where
            P: FnMut(&Self::Item) -> bool,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.find(predicate),
                crate::Either::Right(ref mut inner) => inner.find(predicate),
            }
        }
        fn find_map<B, F>(&mut self, f: F) -> Option<B>
        where
            F: FnMut(Self::Item) -> Option<B>,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.find_map(f),
                crate::Either::Right(ref mut inner) => inner.find_map(f),
            }
        }
        fn position<P>(&mut self, predicate: P) -> Option<usize>
        where
            P: FnMut(Self::Item) -> bool,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.position(predicate),
                crate::Either::Right(ref mut inner) => inner.position(predicate),
            }
        }
    }
    impl<L, R> DoubleEndedIterator for Either<L, R>
    where
        L: DoubleEndedIterator,
        R: DoubleEndedIterator<Item = L::Item>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            match *self {
                crate::Either::Left(ref mut inner) => inner.next_back(),
                crate::Either::Right(ref mut inner) => inner.next_back(),
            }
        }
        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            match *self {
                crate::Either::Left(ref mut inner) => inner.nth_back(n),
                crate::Either::Right(ref mut inner) => inner.nth_back(n),
            }
        }
        fn rfold<Acc, G>(self, init: Acc, f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            match self {
                crate::Either::Left(inner) => inner.rfold(init, f),
                crate::Either::Right(inner) => inner.rfold(init, f),
            }
        }
        fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
        where
            P: FnMut(&Self::Item) -> bool,
        {
            match *self {
                crate::Either::Left(ref mut inner) => inner.rfind(predicate),
                crate::Either::Right(ref mut inner) => inner.rfind(predicate),
            }
        }
    }
    impl<L, R> ExactSizeIterator for Either<L, R>
    where
        L: ExactSizeIterator,
        R: ExactSizeIterator<Item = L::Item>,
    {
        fn len(&self) -> usize {
            match *self {
                crate::Either::Left(ref inner) => inner.len(),
                crate::Either::Right(ref inner) => inner.len(),
            }
        }
    }
    impl<L, R> iter::FusedIterator for Either<L, R>
    where
        L: iter::FusedIterator,
        R: iter::FusedIterator<Item = L::Item>,
    {}
    impl<L, R> Iterator for IterEither<L, R>
    where
        L: Iterator,
        R: Iterator,
    {
        type Item = Either<L::Item, R::Item>;
        fn next(&mut self) -> Option<Self::Item> {
            Some(
                match self.inner {
                    Left(ref mut inner) => Left(inner.next()?),
                    Right(ref mut inner) => Right(inner.next()?),
                },
            )
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            match self.inner {
                crate::Either::Left(ref inner) => inner.size_hint(),
                crate::Either::Right(ref inner) => inner.size_hint(),
            }
        }
        fn fold<Acc, G>(self, init: Acc, f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            match self.inner {
                Left(inner) => inner.map(Left).fold(init, f),
                Right(inner) => inner.map(Right).fold(init, f),
            }
        }
        fn for_each<F>(self, f: F)
        where
            F: FnMut(Self::Item),
        {
            match self.inner {
                Left(inner) => inner.map(Left).for_each(f),
                Right(inner) => inner.map(Right).for_each(f),
            }
        }
        fn count(self) -> usize {
            match self.inner {
                crate::Either::Left(inner) => inner.count(),
                crate::Either::Right(inner) => inner.count(),
            }
        }
        fn last(self) -> Option<Self::Item> {
            Some(
                match self.inner {
                    Left(inner) => Left(inner.last()?),
                    Right(inner) => Right(inner.last()?),
                },
            )
        }
        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            Some(
                match self.inner {
                    Left(ref mut inner) => Left(inner.nth(n)?),
                    Right(ref mut inner) => Right(inner.nth(n)?),
                },
            )
        }
        fn collect<B>(self) -> B
        where
            B: iter::FromIterator<Self::Item>,
        {
            match self.inner {
                Left(inner) => inner.map(Left).collect(),
                Right(inner) => inner.map(Right).collect(),
            }
        }
        fn partition<B, F>(self, f: F) -> (B, B)
        where
            B: Default + Extend<Self::Item>,
            F: FnMut(&Self::Item) -> bool,
        {
            match self.inner {
                Left(inner) => inner.map(Left).partition(f),
                Right(inner) => inner.map(Right).partition(f),
            }
        }
        fn all<F>(&mut self, f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).all(f),
                Right(inner) => inner.map(Right).all(f),
            }
        }
        fn any<F>(&mut self, f: F) -> bool
        where
            F: FnMut(Self::Item) -> bool,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).any(f),
                Right(inner) => inner.map(Right).any(f),
            }
        }
        fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
        where
            P: FnMut(&Self::Item) -> bool,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).find(predicate),
                Right(inner) => inner.map(Right).find(predicate),
            }
        }
        fn find_map<B, F>(&mut self, f: F) -> Option<B>
        where
            F: FnMut(Self::Item) -> Option<B>,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).find_map(f),
                Right(inner) => inner.map(Right).find_map(f),
            }
        }
        fn position<P>(&mut self, predicate: P) -> Option<usize>
        where
            P: FnMut(Self::Item) -> bool,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).position(predicate),
                Right(inner) => inner.map(Right).position(predicate),
            }
        }
    }
    impl<L, R> DoubleEndedIterator for IterEither<L, R>
    where
        L: DoubleEndedIterator,
        R: DoubleEndedIterator,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            Some(
                match self.inner {
                    Left(ref mut inner) => Left(inner.next_back()?),
                    Right(ref mut inner) => Right(inner.next_back()?),
                },
            )
        }
        fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
            Some(
                match self.inner {
                    Left(ref mut inner) => Left(inner.nth_back(n)?),
                    Right(ref mut inner) => Right(inner.nth_back(n)?),
                },
            )
        }
        fn rfold<Acc, G>(self, init: Acc, f: G) -> Acc
        where
            G: FnMut(Acc, Self::Item) -> Acc,
        {
            match self.inner {
                Left(inner) => inner.map(Left).rfold(init, f),
                Right(inner) => inner.map(Right).rfold(init, f),
            }
        }
        fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
        where
            P: FnMut(&Self::Item) -> bool,
        {
            match &mut self.inner {
                Left(inner) => inner.map(Left).rfind(predicate),
                Right(inner) => inner.map(Right).rfind(predicate),
            }
        }
    }
    impl<L, R> ExactSizeIterator for IterEither<L, R>
    where
        L: ExactSizeIterator,
        R: ExactSizeIterator,
    {
        fn len(&self) -> usize {
            match self.inner {
                crate::Either::Left(ref inner) => inner.len(),
                crate::Either::Right(ref inner) => inner.len(),
            }
        }
    }
    impl<L, R> iter::FusedIterator for IterEither<L, R>
    where
        L: iter::FusedIterator,
        R: iter::FusedIterator,
    {}
}
pub use self::iterator::IterEither;
mod into_either {
    //! The trait [`IntoEither`] provides methods for converting a type `Self`, whose
    //! size is constant and known at compile-time, into an [`Either`] variant.
    use super::{Either, Left, Right};
    /// Provides methods for converting a type `Self` into either a [`Left`] or [`Right`]
    /// variant of [`Either<Self, Self>`](Either).
    ///
    /// The [`into_either`](IntoEither::into_either) method takes a [`bool`] to determine
    /// whether to convert to [`Left`] or [`Right`].
    ///
    /// The [`into_either_with`](IntoEither::into_either_with) method takes a
    /// [predicate function](FnOnce) to determine whether to convert to [`Left`] or [`Right`].
    pub trait IntoEither: Sized {
        /// Converts `self` into a [`Left`] variant of [`Either<Self, Self>`](Either)
        /// if `into_left` is `true`.
        /// Converts `self` into a [`Right`] variant of [`Either<Self, Self>`](Either)
        /// otherwise.
        ///
        /// # Examples
        ///
        /// ```
        /// use either::{IntoEither, Left, Right};
        ///
        /// let x = 0;
        /// assert_eq!(x.into_either(true), Left(x));
        /// assert_eq!(x.into_either(false), Right(x));
        /// ```
        fn into_either(self, into_left: bool) -> Either<Self, Self> {
            if into_left { Left(self) } else { Right(self) }
        }
        /// Converts `self` into a [`Left`] variant of [`Either<Self, Self>`](Either)
        /// if `into_left(&self)` returns `true`.
        /// Converts `self` into a [`Right`] variant of [`Either<Self, Self>`](Either)
        /// otherwise.
        ///
        /// # Examples
        ///
        /// ```
        /// use either::{IntoEither, Left, Right};
        ///
        /// fn is_even(x: &u8) -> bool {
        ///     x % 2 == 0
        /// }
        ///
        /// let x = 0;
        /// assert_eq!(x.into_either_with(is_even), Left(x));
        /// assert_eq!(x.into_either_with(|x| !is_even(x)), Right(x));
        /// ```
        fn into_either_with<F>(self, into_left: F) -> Either<Self, Self>
        where
            F: FnOnce(&Self) -> bool,
        {
            let into_left = into_left(&self);
            self.into_either(into_left)
        }
    }
    impl<T> IntoEither for T {}
}
pub use self::into_either::IntoEither;
impl<L: Clone, R: Clone> Clone for Either<L, R> {
    fn clone(&self) -> Self {
        match self {
            Left(inner) => Left(inner.clone()),
            Right(inner) => Right(inner.clone()),
        }
    }
    fn clone_from(&mut self, source: &Self) {
        match (self, source) {
            (Left(dest), Left(source)) => dest.clone_from(source),
            (Right(dest), Right(source)) => dest.clone_from(source),
            (dest, source) => *dest = source.clone(),
        }
    }
}
impl<L, R> Either<L, R> {
    /// Return true if the value is the `Left` variant.
    ///
    /// ```
    /// use either::*;
    ///
    /// let values = [Left(1), Right("the right value")];
    /// assert_eq!(values[0].is_left(), true);
    /// assert_eq!(values[1].is_left(), false);
    /// ```
    pub fn is_left(&self) -> bool {
        match *self {
            Left(_) => true,
            Right(_) => false,
        }
    }
    /// Return true if the value is the `Right` variant.
    ///
    /// ```
    /// use either::*;
    ///
    /// let values = [Left(1), Right("the right value")];
    /// assert_eq!(values[0].is_right(), false);
    /// assert_eq!(values[1].is_right(), true);
    /// ```
    pub fn is_right(&self) -> bool {
        !self.is_left()
    }
    /// Convert the left side of `Either<L, R>` to an `Option<L>`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, ()> = Left("some value");
    /// assert_eq!(left.left(),  Some("some value"));
    ///
    /// let right: Either<(), _> = Right(321);
    /// assert_eq!(right.left(), None);
    /// ```
    pub fn left(self) -> Option<L> {
        match self {
            Left(l) => Some(l),
            Right(_) => None,
        }
    }
    /// Convert the right side of `Either<L, R>` to an `Option<R>`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, ()> = Left("some value");
    /// assert_eq!(left.right(),  None);
    ///
    /// let right: Either<(), _> = Right(321);
    /// assert_eq!(right.right(), Some(321));
    /// ```
    pub fn right(self) -> Option<R> {
        match self {
            Left(_) => None,
            Right(r) => Some(r),
        }
    }
    /// Convert `&Either<L, R>` to `Either<&L, &R>`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, ()> = Left("some value");
    /// assert_eq!(left.as_ref(), Left(&"some value"));
    ///
    /// let right: Either<(), _> = Right("some value");
    /// assert_eq!(right.as_ref(), Right(&"some value"));
    /// ```
    pub fn as_ref(&self) -> Either<&L, &R> {
        match *self {
            Left(ref inner) => Left(inner),
            Right(ref inner) => Right(inner),
        }
    }
    /// Convert `&mut Either<L, R>` to `Either<&mut L, &mut R>`.
    ///
    /// ```
    /// use either::*;
    ///
    /// fn mutate_left(value: &mut Either<u32, u32>) {
    ///     if let Some(l) = value.as_mut().left() {
    ///         *l = 999;
    ///     }
    /// }
    ///
    /// let mut left = Left(123);
    /// let mut right = Right(123);
    /// mutate_left(&mut left);
    /// mutate_left(&mut right);
    /// assert_eq!(left, Left(999));
    /// assert_eq!(right, Right(123));
    /// ```
    pub fn as_mut(&mut self) -> Either<&mut L, &mut R> {
        match *self {
            Left(ref mut inner) => Left(inner),
            Right(ref mut inner) => Right(inner),
        }
    }
    /// Convert `Pin<&Either<L, R>>` to `Either<Pin<&L>, Pin<&R>>`,
    /// pinned projections of the inner variants.
    pub fn as_pin_ref(self: Pin<&Self>) -> Either<Pin<&L>, Pin<&R>> {
        unsafe {
            match *Pin::get_ref(self) {
                Left(ref inner) => Left(Pin::new_unchecked(inner)),
                Right(ref inner) => Right(Pin::new_unchecked(inner)),
            }
        }
    }
    /// Convert `Pin<&mut Either<L, R>>` to `Either<Pin<&mut L>, Pin<&mut R>>`,
    /// pinned projections of the inner variants.
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Either<Pin<&mut L>, Pin<&mut R>> {
        unsafe {
            match *Pin::get_unchecked_mut(self) {
                Left(ref mut inner) => Left(Pin::new_unchecked(inner)),
                Right(ref mut inner) => Right(Pin::new_unchecked(inner)),
            }
        }
    }
    /// Convert `Either<L, R>` to `Either<R, L>`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, ()> = Left(123);
    /// assert_eq!(left.flip(), Right(123));
    ///
    /// let right: Either<(), _> = Right("some value");
    /// assert_eq!(right.flip(), Left("some value"));
    /// ```
    pub fn flip(self) -> Either<R, L> {
        match self {
            Left(l) => Right(l),
            Right(r) => Left(r),
        }
    }
    /// Apply the function `f` on the value in the `Left` variant if it is present rewrapping the
    /// result in `Left`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, u32> = Left(123);
    /// assert_eq!(left.map_left(|x| x * 2), Left(246));
    ///
    /// let right: Either<u32, _> = Right(123);
    /// assert_eq!(right.map_left(|x| x * 2), Right(123));
    /// ```
    pub fn map_left<F, M>(self, f: F) -> Either<M, R>
    where
        F: FnOnce(L) -> M,
    {
        match self {
            Left(l) => Left(f(l)),
            Right(r) => Right(r),
        }
    }
    /// Apply the function `f` on the value in the `Right` variant if it is present rewrapping the
    /// result in `Right`.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, u32> = Left(123);
    /// assert_eq!(left.map_right(|x| x * 2), Left(123));
    ///
    /// let right: Either<u32, _> = Right(123);
    /// assert_eq!(right.map_right(|x| x * 2), Right(246));
    /// ```
    pub fn map_right<F, S>(self, f: F) -> Either<L, S>
    where
        F: FnOnce(R) -> S,
    {
        match self {
            Left(l) => Left(l),
            Right(r) => Right(f(r)),
        }
    }
    /// Apply the functions `f` and `g` to the `Left` and `Right` variants
    /// respectively. This is equivalent to
    /// [bimap](https://hackage.haskell.org/package/bifunctors-5/docs/Data-Bifunctor.html)
    /// in functional programming.
    ///
    /// ```
    /// use either::*;
    ///
    /// let f = |s: String| s.len();
    /// let g = |u: u8| u.to_string();
    ///
    /// let left: Either<String, u8> = Left("loopy".into());
    /// assert_eq!(left.map_either(f, g), Left(5));
    ///
    /// let right: Either<String, u8> = Right(42);
    /// assert_eq!(right.map_either(f, g), Right("42".into()));
    /// ```
    pub fn map_either<F, G, M, S>(self, f: F, g: G) -> Either<M, S>
    where
        F: FnOnce(L) -> M,
        G: FnOnce(R) -> S,
    {
        match self {
            Left(l) => Left(f(l)),
            Right(r) => Right(g(r)),
        }
    }
    /// Similar to [`map_either`][Self::map_either], with an added context `ctx` accessible to
    /// both functions.
    ///
    /// ```
    /// use either::*;
    ///
    /// let mut sum = 0;
    ///
    /// // Both closures want to update the same value, so pass it as context.
    /// let mut f = |sum: &mut usize, s: String| { *sum += s.len(); s.to_uppercase() };
    /// let mut g = |sum: &mut usize, u: usize| { *sum += u; u.to_string() };
    ///
    /// let left: Either<String, usize> = Left("loopy".into());
    /// assert_eq!(left.map_either_with(&mut sum, &mut f, &mut g), Left("LOOPY".into()));
    ///
    /// let right: Either<String, usize> = Right(42);
    /// assert_eq!(right.map_either_with(&mut sum, &mut f, &mut g), Right("42".into()));
    ///
    /// assert_eq!(sum, 47);
    /// ```
    pub fn map_either_with<Ctx, F, G, M, S>(self, ctx: Ctx, f: F, g: G) -> Either<M, S>
    where
        F: FnOnce(Ctx, L) -> M,
        G: FnOnce(Ctx, R) -> S,
    {
        match self {
            Left(l) => Left(f(ctx, l)),
            Right(r) => Right(g(ctx, r)),
        }
    }
    /// Apply one of two functions depending on contents, unifying their result. If the value is
    /// `Left(L)` then the first function `f` is applied; if it is `Right(R)` then the second
    /// function `g` is applied.
    ///
    /// ```
    /// use either::*;
    ///
    /// fn square(n: u32) -> i32 { (n * n) as i32 }
    /// fn negate(n: i32) -> i32 { -n }
    ///
    /// let left: Either<u32, i32> = Left(4);
    /// assert_eq!(left.either(square, negate), 16);
    ///
    /// let right: Either<u32, i32> = Right(-4);
    /// assert_eq!(right.either(square, negate), 4);
    /// ```
    pub fn either<F, G, T>(self, f: F, g: G) -> T
    where
        F: FnOnce(L) -> T,
        G: FnOnce(R) -> T,
    {
        match self {
            Left(l) => f(l),
            Right(r) => g(r),
        }
    }
    /// Like [`either`][Self::either], but provide some context to whichever of the
    /// functions ends up being called.
    ///
    /// ```
    /// // In this example, the context is a mutable reference
    /// use either::*;
    ///
    /// let mut result = Vec::new();
    ///
    /// let values = vec![Left(2), Right(2.7)];
    ///
    /// for value in values {
    ///     value.either_with(&mut result,
    ///                       |ctx, integer| ctx.push(integer),
    ///                       |ctx, real| ctx.push(f64::round(real) as i32));
    /// }
    ///
    /// assert_eq!(result, vec![2, 3]);
    /// ```
    pub fn either_with<Ctx, F, G, T>(self, ctx: Ctx, f: F, g: G) -> T
    where
        F: FnOnce(Ctx, L) -> T,
        G: FnOnce(Ctx, R) -> T,
    {
        match self {
            Left(l) => f(ctx, l),
            Right(r) => g(ctx, r),
        }
    }
    /// Apply the function `f` on the value in the `Left` variant if it is present.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, u32> = Left(123);
    /// assert_eq!(left.left_and_then::<_,()>(|x| Right(x * 2)), Right(246));
    ///
    /// let right: Either<u32, _> = Right(123);
    /// assert_eq!(right.left_and_then(|x| Right::<(), _>(x * 2)), Right(123));
    /// ```
    pub fn left_and_then<F, S>(self, f: F) -> Either<S, R>
    where
        F: FnOnce(L) -> Either<S, R>,
    {
        match self {
            Left(l) => f(l),
            Right(r) => Right(r),
        }
    }
    /// Apply the function `f` on the value in the `Right` variant if it is present.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, u32> = Left(123);
    /// assert_eq!(left.right_and_then(|x| Right(x * 2)), Left(123));
    ///
    /// let right: Either<u32, _> = Right(123);
    /// assert_eq!(right.right_and_then(|x| Right(x * 2)), Right(246));
    /// ```
    pub fn right_and_then<F, S>(self, f: F) -> Either<L, S>
    where
        F: FnOnce(R) -> Either<L, S>,
    {
        match self {
            Left(l) => Left(l),
            Right(r) => f(r),
        }
    }
    /// Convert the inner value to an iterator.
    ///
    /// This requires the `Left` and `Right` iterators to have the same item type.
    /// See [`factor_into_iter`][Either::factor_into_iter] to iterate different types.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, Vec<u32>> = Left(vec![1, 2, 3, 4, 5]);
    /// let mut right: Either<Vec<u32>, _> = Right(vec![]);
    /// right.extend(left.into_iter());
    /// assert_eq!(right, Right(vec![1, 2, 3, 4, 5]));
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> Either<L::IntoIter, R::IntoIter>
    where
        L: IntoIterator,
        R: IntoIterator<Item = L::Item>,
    {
        match self {
            Left(inner) => Left(inner.into_iter()),
            Right(inner) => Right(inner.into_iter()),
        }
    }
    /// Borrow the inner value as an iterator.
    ///
    /// This requires the `Left` and `Right` iterators to have the same item type.
    /// See [`factor_iter`][Either::factor_iter] to iterate different types.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, &[u32]> = Left(vec![2, 3]);
    /// let mut right: Either<Vec<u32>, _> = Right(&[4, 5][..]);
    /// let mut all = vec![1];
    /// all.extend(left.iter());
    /// all.extend(right.iter());
    /// assert_eq!(all, vec![1, 2, 3, 4, 5]);
    /// ```
    pub fn iter(
        &self,
    ) -> Either<<&L as IntoIterator>::IntoIter, <&R as IntoIterator>::IntoIter>
    where
        for<'a> &'a L: IntoIterator,
        for<'a> &'a R: IntoIterator<Item = <&'a L as IntoIterator>::Item>,
    {
        match self {
            Left(inner) => Left(inner.into_iter()),
            Right(inner) => Right(inner.into_iter()),
        }
    }
    /// Mutably borrow the inner value as an iterator.
    ///
    /// This requires the `Left` and `Right` iterators to have the same item type.
    /// See [`factor_iter_mut`][Either::factor_iter_mut] to iterate different types.
    ///
    /// ```
    /// use either::*;
    ///
    /// let mut left: Either<_, &mut [u32]> = Left(vec![2, 3]);
    /// for l in left.iter_mut() {
    ///     *l *= *l
    /// }
    /// assert_eq!(left, Left(vec![4, 9]));
    ///
    /// let mut inner = [4, 5];
    /// let mut right: Either<Vec<u32>, _> = Right(&mut inner[..]);
    /// for r in right.iter_mut() {
    ///     *r *= *r
    /// }
    /// assert_eq!(inner, [16, 25]);
    /// ```
    pub fn iter_mut(
        &mut self,
    ) -> Either<<&mut L as IntoIterator>::IntoIter, <&mut R as IntoIterator>::IntoIter>
    where
        for<'a> &'a mut L: IntoIterator,
        for<'a> &'a mut R: IntoIterator<Item = <&'a mut L as IntoIterator>::Item>,
    {
        match self {
            Left(inner) => Left(inner.into_iter()),
            Right(inner) => Right(inner.into_iter()),
        }
    }
    /// Converts an `Either` of `Iterator`s to be an `Iterator` of `Either`s
    ///
    /// Unlike [`into_iter`][Either::into_iter], this does not require the
    /// `Left` and `Right` iterators to have the same item type.
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, Vec<u8>> = Left(&["hello"]);
    /// assert_eq!(left.factor_into_iter().next(), Some(Left(&"hello")));
    /// let right: Either<&[&str], _> = Right(vec![0, 1]);
    /// assert_eq!(right.factor_into_iter().collect::<Vec<_>>(), vec![Right(0), Right(1)]);
    ///
    /// ```
    pub fn factor_into_iter(self) -> IterEither<L::IntoIter, R::IntoIter>
    where
        L: IntoIterator,
        R: IntoIterator,
    {
        IterEither::new(
            match self {
                Left(inner) => Left(inner.into_iter()),
                Right(inner) => Right(inner.into_iter()),
            },
        )
    }
    /// Borrows an `Either` of `Iterator`s to be an `Iterator` of `Either`s
    ///
    /// Unlike [`iter`][Either::iter], this does not require the
    /// `Left` and `Right` iterators to have the same item type.
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, Vec<u8>> = Left(["hello"]);
    /// assert_eq!(left.factor_iter().next(), Some(Left(&"hello")));
    /// let right: Either<[&str; 2], _> = Right(vec![0, 1]);
    /// assert_eq!(right.factor_iter().collect::<Vec<_>>(), vec![Right(&0), Right(&1)]);
    ///
    /// ```
    pub fn factor_iter(
        &self,
    ) -> IterEither<<&L as IntoIterator>::IntoIter, <&R as IntoIterator>::IntoIter>
    where
        for<'a> &'a L: IntoIterator,
        for<'a> &'a R: IntoIterator,
    {
        IterEither::new(
            match self {
                Left(inner) => Left(inner.into_iter()),
                Right(inner) => Right(inner.into_iter()),
            },
        )
    }
    /// Mutably borrows an `Either` of `Iterator`s to be an `Iterator` of `Either`s
    ///
    /// Unlike [`iter_mut`][Either::iter_mut], this does not require the
    /// `Left` and `Right` iterators to have the same item type.
    ///
    /// ```
    /// use either::*;
    /// let mut left: Either<_, Vec<u8>> = Left(["hello"]);
    /// left.factor_iter_mut().for_each(|x| *x.unwrap_left() = "goodbye");
    /// assert_eq!(left, Left(["goodbye"]));
    /// let mut right: Either<[&str; 2], _> = Right(vec![0, 1, 2]);
    /// right.factor_iter_mut().for_each(|x| if let Right(r) = x { *r = -*r; });
    /// assert_eq!(right, Right(vec![0, -1, -2]));
    ///
    /// ```
    pub fn factor_iter_mut(
        &mut self,
    ) -> IterEither<
        <&mut L as IntoIterator>::IntoIter,
        <&mut R as IntoIterator>::IntoIter,
    >
    where
        for<'a> &'a mut L: IntoIterator,
        for<'a> &'a mut R: IntoIterator,
    {
        IterEither::new(
            match self {
                Left(inner) => Left(inner.into_iter()),
                Right(inner) => Right(inner.into_iter()),
            },
        )
    }
    /// Return left value or given value
    ///
    /// Arguments passed to `left_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`left_or_else`][Self::left_or_else], which is lazily evaluated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<&str, &str> = Left("left");
    /// assert_eq!(left.left_or("foo"), "left");
    ///
    /// let right: Either<&str, &str> = Right("right");
    /// assert_eq!(right.left_or("left"), "left");
    /// ```
    pub fn left_or(self, other: L) -> L {
        match self {
            Either::Left(l) => l,
            Either::Right(_) => other,
        }
    }
    /// Return left or a default
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<String, u32> = Left("left".to_string());
    /// assert_eq!(left.left_or_default(), "left");
    ///
    /// let right: Either<String, u32> = Right(42);
    /// assert_eq!(right.left_or_default(), String::default());
    /// ```
    pub fn left_or_default(self) -> L
    where
        L: Default,
    {
        match self {
            Either::Left(l) => l,
            Either::Right(_) => L::default(),
        }
    }
    /// Returns left value or computes it from a closure
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<String, u32> = Left("3".to_string());
    /// assert_eq!(left.left_or_else(|_| unreachable!()), "3");
    ///
    /// let right: Either<String, u32> = Right(3);
    /// assert_eq!(right.left_or_else(|x| x.to_string()), "3");
    /// ```
    pub fn left_or_else<F>(self, f: F) -> L
    where
        F: FnOnce(R) -> L,
    {
        match self {
            Either::Left(l) => l,
            Either::Right(r) => f(r),
        }
    }
    /// Return right value or given value
    ///
    /// Arguments passed to `right_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`right_or_else`][Self::right_or_else], which is lazily evaluated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let right: Either<&str, &str> = Right("right");
    /// assert_eq!(right.right_or("foo"), "right");
    ///
    /// let left: Either<&str, &str> = Left("left");
    /// assert_eq!(left.right_or("right"), "right");
    /// ```
    pub fn right_or(self, other: R) -> R {
        match self {
            Either::Left(_) => other,
            Either::Right(r) => r,
        }
    }
    /// Return right or a default
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<String, u32> = Left("left".to_string());
    /// assert_eq!(left.right_or_default(), u32::default());
    ///
    /// let right: Either<String, u32> = Right(42);
    /// assert_eq!(right.right_or_default(), 42);
    /// ```
    pub fn right_or_default(self) -> R
    where
        R: Default,
    {
        match self {
            Either::Left(_) => R::default(),
            Either::Right(r) => r,
        }
    }
    /// Returns right value or computes it from a closure
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<String, u32> = Left("3".to_string());
    /// assert_eq!(left.right_or_else(|x| x.parse().unwrap()), 3);
    ///
    /// let right: Either<String, u32> = Right(3);
    /// assert_eq!(right.right_or_else(|_| unreachable!()), 3);
    /// ```
    pub fn right_or_else<F>(self, f: F) -> R
    where
        F: FnOnce(L) -> R,
    {
        match self {
            Either::Left(l) => f(l),
            Either::Right(r) => r,
        }
    }
    /// Returns the left value
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<_, ()> = Left(3);
    /// assert_eq!(left.unwrap_left(), 3);
    /// ```
    ///
    /// # Panics
    ///
    /// When `Either` is a `Right` value
    ///
    /// ```should_panic
    /// # use either::*;
    /// let right: Either<(), _> = Right(3);
    /// right.unwrap_left();
    /// ```
    pub fn unwrap_left(self) -> L
    where
        R: core::fmt::Debug,
    {
        match self {
            Either::Left(l) => l,
            Either::Right(r) => {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "called `Either::unwrap_left()` on a `Right` value: {0:?}",
                        r,
                    ),
                );
            }
        }
    }
    /// Returns the right value
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let right: Either<(), _> = Right(3);
    /// assert_eq!(right.unwrap_right(), 3);
    /// ```
    ///
    /// # Panics
    ///
    /// When `Either` is a `Left` value
    ///
    /// ```should_panic
    /// # use either::*;
    /// let left: Either<_, ()> = Left(3);
    /// left.unwrap_right();
    /// ```
    pub fn unwrap_right(self) -> R
    where
        L: core::fmt::Debug,
    {
        match self {
            Either::Right(r) => r,
            Either::Left(l) => {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "called `Either::unwrap_right()` on a `Left` value: {0:?}",
                        l,
                    ),
                );
            }
        }
    }
    /// Returns the left value
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let left: Either<_, ()> = Left(3);
    /// assert_eq!(left.expect_left("value was Right"), 3);
    /// ```
    ///
    /// # Panics
    ///
    /// When `Either` is a `Right` value
    ///
    /// ```should_panic
    /// # use either::*;
    /// let right: Either<(), _> = Right(3);
    /// right.expect_left("value was Right");
    /// ```
    pub fn expect_left(self, msg: &str) -> L
    where
        R: core::fmt::Debug,
    {
        match self {
            Either::Left(l) => l,
            Either::Right(r) => {
                ::core::panicking::panic_fmt(format_args!("{0}: {1:?}", msg, r));
            }
        }
    }
    /// Returns the right value
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// let right: Either<(), _> = Right(3);
    /// assert_eq!(right.expect_right("value was Left"), 3);
    /// ```
    ///
    /// # Panics
    ///
    /// When `Either` is a `Left` value
    ///
    /// ```should_panic
    /// # use either::*;
    /// let left: Either<_, ()> = Left(3);
    /// left.expect_right("value was Right");
    /// ```
    pub fn expect_right(self, msg: &str) -> R
    where
        L: core::fmt::Debug,
    {
        match self {
            Either::Right(r) => r,
            Either::Left(l) => {
                ::core::panicking::panic_fmt(format_args!("{0}: {1:?}", msg, l));
            }
        }
    }
    /// Convert the contained value into `T`
    ///
    /// # Examples
    ///
    /// ```
    /// # use either::*;
    /// // Both u16 and u32 can be converted to u64.
    /// let left: Either<u16, u32> = Left(3u16);
    /// assert_eq!(left.either_into::<u64>(), 3u64);
    /// let right: Either<u16, u32> = Right(7u32);
    /// assert_eq!(right.either_into::<u64>(), 7u64);
    /// ```
    pub fn either_into<T>(self) -> T
    where
        L: Into<T>,
        R: Into<T>,
    {
        match self {
            Either::Left(l) => l.into(),
            Either::Right(r) => r.into(),
        }
    }
}
impl<L, R> Either<Option<L>, Option<R>> {
    /// Factors out `None` from an `Either` of [`Option`].
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, Option<String>> = Left(Some(vec![0]));
    /// assert_eq!(left.factor_none(), Some(Left(vec![0])));
    ///
    /// let right: Either<Option<Vec<u8>>, _> = Right(Some(String::new()));
    /// assert_eq!(right.factor_none(), Some(Right(String::new())));
    /// ```
    pub fn factor_none(self) -> Option<Either<L, R>> {
        match self {
            Left(l) => l.map(Either::Left),
            Right(r) => r.map(Either::Right),
        }
    }
}
impl<L, R, E> Either<Result<L, E>, Result<R, E>> {
    /// Factors out a homogenous type from an `Either` of [`Result`].
    ///
    /// Here, the homogeneous type is the `Err` type of the [`Result`].
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, Result<String, u32>> = Left(Ok(vec![0]));
    /// assert_eq!(left.factor_err(), Ok(Left(vec![0])));
    ///
    /// let right: Either<Result<Vec<u8>, u32>, _> = Right(Ok(String::new()));
    /// assert_eq!(right.factor_err(), Ok(Right(String::new())));
    /// ```
    pub fn factor_err(self) -> Result<Either<L, R>, E> {
        match self {
            Left(l) => l.map(Either::Left),
            Right(r) => r.map(Either::Right),
        }
    }
}
impl<T, L, R> Either<Result<T, L>, Result<T, R>> {
    /// Factors out a homogenous type from an `Either` of [`Result`].
    ///
    /// Here, the homogeneous type is the `Ok` type of the [`Result`].
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, Result<u32, String>> = Left(Err(vec![0]));
    /// assert_eq!(left.factor_ok(), Err(Left(vec![0])));
    ///
    /// let right: Either<Result<u32, Vec<u8>>, _> = Right(Err(String::new()));
    /// assert_eq!(right.factor_ok(), Err(Right(String::new())));
    /// ```
    pub fn factor_ok(self) -> Result<T, Either<L, R>> {
        match self {
            Left(l) => l.map_err(Either::Left),
            Right(r) => r.map_err(Either::Right),
        }
    }
}
impl<T, L, R> Either<(T, L), (T, R)> {
    /// Factor out a homogeneous type from an either of pairs.
    ///
    /// Here, the homogeneous type is the first element of the pairs.
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, (u32, String)> = Left((123, vec![0]));
    /// assert_eq!(left.factor_first().0, 123);
    ///
    /// let right: Either<(u32, Vec<u8>), _> = Right((123, String::new()));
    /// assert_eq!(right.factor_first().0, 123);
    /// ```
    pub fn factor_first(self) -> (T, Either<L, R>) {
        match self {
            Left((t, l)) => (t, Left(l)),
            Right((t, r)) => (t, Right(r)),
        }
    }
}
impl<T, L, R> Either<(L, T), (R, T)> {
    /// Factor out a homogeneous type from an either of pairs.
    ///
    /// Here, the homogeneous type is the second element of the pairs.
    ///
    /// ```
    /// use either::*;
    /// let left: Either<_, (String, u32)> = Left((vec![0], 123));
    /// assert_eq!(left.factor_second().1, 123);
    ///
    /// let right: Either<(Vec<u8>, u32), _> = Right((String::new(), 123));
    /// assert_eq!(right.factor_second().1, 123);
    /// ```
    pub fn factor_second(self) -> (Either<L, R>, T) {
        match self {
            Left((l, t)) => (Left(l), t),
            Right((r, t)) => (Right(r), t),
        }
    }
}
impl<T> Either<T, T> {
    /// Extract the value of an either over two equivalent types.
    ///
    /// ```
    /// use either::*;
    ///
    /// let left: Either<_, u32> = Left(123);
    /// assert_eq!(left.into_inner(), 123);
    ///
    /// let right: Either<u32, _> = Right(123);
    /// assert_eq!(right.into_inner(), 123);
    /// ```
    pub fn into_inner(self) -> T {
        match self {
            crate::Either::Left(inner) => inner,
            crate::Either::Right(inner) => inner,
        }
    }
    /// Map `f` over the contained value and return the result in the
    /// corresponding variant.
    ///
    /// ```
    /// use either::*;
    ///
    /// let value: Either<_, i32> = Right(42);
    ///
    /// let other = value.map(|x| x * 2);
    /// assert_eq!(other, Right(84));
    /// ```
    pub fn map<F, M>(self, f: F) -> Either<M, M>
    where
        F: FnOnce(T) -> M,
    {
        match self {
            Left(l) => Left(f(l)),
            Right(r) => Right(f(r)),
        }
    }
}
impl<L, R> Either<&L, &R> {
    /// Maps an `Either<&L, &R>` to an `Either<L, R>` by cloning the contents of
    /// either branch.
    pub fn cloned(self) -> Either<L, R>
    where
        L: Clone,
        R: Clone,
    {
        match self {
            Self::Left(l) => Either::Left(l.clone()),
            Self::Right(r) => Either::Right(r.clone()),
        }
    }
    /// Maps an `Either<&L, &R>` to an `Either<L, R>` by copying the contents of
    /// either branch.
    pub fn copied(self) -> Either<L, R>
    where
        L: Copy,
        R: Copy,
    {
        match self {
            Self::Left(l) => Either::Left(*l),
            Self::Right(r) => Either::Right(*r),
        }
    }
}
impl<L, R> Either<&mut L, &mut R> {
    /// Maps an `Either<&mut L, &mut R>` to an `Either<L, R>` by cloning the contents of
    /// either branch.
    pub fn cloned(self) -> Either<L, R>
    where
        L: Clone,
        R: Clone,
    {
        match self {
            Self::Left(l) => Either::Left(l.clone()),
            Self::Right(r) => Either::Right(r.clone()),
        }
    }
    /// Maps an `Either<&mut L, &mut R>` to an `Either<L, R>` by copying the contents of
    /// either branch.
    pub fn copied(self) -> Either<L, R>
    where
        L: Copy,
        R: Copy,
    {
        match self {
            Self::Left(l) => Either::Left(*l),
            Self::Right(r) => Either::Right(*r),
        }
    }
}
/// Convert from `Result` to `Either` with `Ok => Right` and `Err => Left`.
impl<L, R> From<Result<R, L>> for Either<L, R> {
    fn from(r: Result<R, L>) -> Self {
        match r {
            Err(e) => Left(e),
            Ok(o) => Right(o),
        }
    }
}
/// Convert from `Either` to `Result` with `Right => Ok` and `Left => Err`.
#[allow(clippy::from_over_into)]
impl<L, R> Into<Result<R, L>> for Either<L, R> {
    fn into(self) -> Result<R, L> {
        match self {
            Left(l) => Err(l),
            Right(r) => Ok(r),
        }
    }
}
/// `Either<L, R>` is a future if both `L` and `R` are futures.
impl<L, R> Future for Either<L, R>
where
    L: Future,
    R: Future<Output = L::Output>,
{
    type Output = L::Output;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        match self.as_pin_mut() {
            crate::Either::Left(inner) => inner.poll(cx),
            crate::Either::Right(inner) => inner.poll(cx),
        }
    }
}
/// `Either<L, R>` implements `Read` if both `L` and `R` do.
///
/// Requires crate feature `"use_std"`
impl<L, R> Read for Either<L, R>
where
    L: Read,
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read(buf),
            crate::Either::Right(ref mut inner) => inner.read(buf),
        }
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read_exact(buf),
            crate::Either::Right(ref mut inner) => inner.read_exact(buf),
        }
    }
    fn read_to_end(&mut self, buf: &mut std::vec::Vec<u8>) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read_to_end(buf),
            crate::Either::Right(ref mut inner) => inner.read_to_end(buf),
        }
    }
    fn read_to_string(&mut self, buf: &mut std::string::String) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read_to_string(buf),
            crate::Either::Right(ref mut inner) => inner.read_to_string(buf),
        }
    }
}
/// `Either<L, R>` implements `Seek` if both `L` and `R` do.
///
/// Requires crate feature `"use_std"`
impl<L, R> Seek for Either<L, R>
where
    L: Seek,
    R: Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.seek(pos),
            crate::Either::Right(ref mut inner) => inner.seek(pos),
        }
    }
}
/// Requires crate feature `"use_std"`
impl<L, R> BufRead for Either<L, R>
where
    L: BufRead,
    R: BufRead,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.fill_buf(),
            crate::Either::Right(ref mut inner) => inner.fill_buf(),
        }
    }
    fn consume(&mut self, amt: usize) {
        match *self {
            crate::Either::Left(ref mut inner) => inner.consume(amt),
            crate::Either::Right(ref mut inner) => inner.consume(amt),
        }
    }
    fn read_until(
        &mut self,
        byte: u8,
        buf: &mut std::vec::Vec<u8>,
    ) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read_until(byte, buf),
            crate::Either::Right(ref mut inner) => inner.read_until(byte, buf),
        }
    }
    fn read_line(&mut self, buf: &mut std::string::String) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.read_line(buf),
            crate::Either::Right(ref mut inner) => inner.read_line(buf),
        }
    }
}
/// `Either<L, R>` implements `Write` if both `L` and `R` do.
///
/// Requires crate feature `"use_std"`
impl<L, R> Write for Either<L, R>
where
    L: Write,
    R: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.write(buf),
            crate::Either::Right(ref mut inner) => inner.write(buf),
        }
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.write_all(buf),
            crate::Either::Right(ref mut inner) => inner.write_all(buf),
        }
    }
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.write_fmt(fmt),
            crate::Either::Right(ref mut inner) => inner.write_fmt(fmt),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            crate::Either::Left(ref mut inner) => inner.flush(),
            crate::Either::Right(ref mut inner) => inner.flush(),
        }
    }
}
impl<L, R, Target> AsRef<Target> for Either<L, R>
where
    L: AsRef<Target>,
    R: AsRef<Target>,
{
    fn as_ref(&self) -> &Target {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
impl<L, R> AsRef<str> for Either<L, R>
where
    L: AsRef<str>,
    R: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
impl<L, R> AsMut<str> for Either<L, R>
where
    L: AsMut<str>,
    R: AsMut<str>,
{
    fn as_mut(&mut self) -> &mut str {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsRef<::std::path::Path> for Either<L, R>
where
    L: AsRef<::std::path::Path>,
    R: AsRef<::std::path::Path>,
{
    fn as_ref(&self) -> &::std::path::Path {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsMut<::std::path::Path> for Either<L, R>
where
    L: AsMut<::std::path::Path>,
    R: AsMut<::std::path::Path>,
{
    fn as_mut(&mut self) -> &mut ::std::path::Path {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsRef<::std::ffi::OsStr> for Either<L, R>
where
    L: AsRef<::std::ffi::OsStr>,
    R: AsRef<::std::ffi::OsStr>,
{
    fn as_ref(&self) -> &::std::ffi::OsStr {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsMut<::std::ffi::OsStr> for Either<L, R>
where
    L: AsMut<::std::ffi::OsStr>,
    R: AsMut<::std::ffi::OsStr>,
{
    fn as_mut(&mut self) -> &mut ::std::ffi::OsStr {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsRef<::std::ffi::CStr> for Either<L, R>
where
    L: AsRef<::std::ffi::CStr>,
    R: AsRef<::std::ffi::CStr>,
{
    fn as_ref(&self) -> &::std::ffi::CStr {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
///Requires crate feature `use_std`.
impl<L, R> AsMut<::std::ffi::CStr> for Either<L, R>
where
    L: AsMut<::std::ffi::CStr>,
    R: AsMut<::std::ffi::CStr>,
{
    fn as_mut(&mut self) -> &mut ::std::ffi::CStr {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
impl<L, R, Target> AsRef<[Target]> for Either<L, R>
where
    L: AsRef<[Target]>,
    R: AsRef<[Target]>,
{
    fn as_ref(&self) -> &[Target] {
        match *self {
            crate::Either::Left(ref inner) => inner.as_ref(),
            crate::Either::Right(ref inner) => inner.as_ref(),
        }
    }
}
impl<L, R, Target> AsMut<Target> for Either<L, R>
where
    L: AsMut<Target>,
    R: AsMut<Target>,
{
    fn as_mut(&mut self) -> &mut Target {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
impl<L, R, Target> AsMut<[Target]> for Either<L, R>
where
    L: AsMut<[Target]>,
    R: AsMut<[Target]>,
{
    fn as_mut(&mut self) -> &mut [Target] {
        match *self {
            crate::Either::Left(ref mut inner) => inner.as_mut(),
            crate::Either::Right(ref mut inner) => inner.as_mut(),
        }
    }
}
impl<L, R> Deref for Either<L, R>
where
    L: Deref,
    R: Deref<Target = L::Target>,
{
    type Target = L::Target;
    fn deref(&self) -> &Self::Target {
        match *self {
            crate::Either::Left(ref inner) => &**inner,
            crate::Either::Right(ref inner) => &**inner,
        }
    }
}
impl<L, R> DerefMut for Either<L, R>
where
    L: DerefMut,
    R: DerefMut<Target = L::Target>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match *self {
            crate::Either::Left(ref mut inner) => &mut *inner,
            crate::Either::Right(ref mut inner) => &mut *inner,
        }
    }
}
/// `Either` implements `Error` if *both* `L` and `R` implement it.
///
/// Requires crate feature `"use_std"`
impl<L, R> Error for Either<L, R>
where
    L: Error,
    R: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            crate::Either::Left(ref inner) => inner.source(),
            crate::Either::Right(ref inner) => inner.source(),
        }
    }
    #[allow(deprecated)]
    fn description(&self) -> &str {
        match *self {
            crate::Either::Left(ref inner) => inner.description(),
            crate::Either::Right(ref inner) => inner.description(),
        }
    }
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            crate::Either::Left(ref inner) => inner.cause(),
            crate::Either::Right(ref inner) => inner.cause(),
        }
    }
}
impl<L, R> fmt::Display for Either<L, R>
where
    L: fmt::Display,
    R: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            crate::Either::Left(ref inner) => inner.fmt(f),
            crate::Either::Right(ref inner) => inner.fmt(f),
        }
    }
}
extern crate test;
#[rustc_test_marker = "basic"]
#[doc(hidden)]
pub const basic: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("basic"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1415usize,
        start_col: 4usize,
        end_line: 1415usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(basic())),
};
fn basic() {
    let mut e = Left(2);
    let r = Right(2);
    match (&e, &Left(2)) {
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
    e = r;
    match (&e, &Right(2)) {
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
    match (&e.left(), &None) {
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
    match (&e.right(), &Some(2)) {
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
    match (&e.as_ref().right(), &Some(&2)) {
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
    match (&e.as_mut().right(), &Some(&mut 2)) {
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
#[rustc_test_marker = "macros"]
#[doc(hidden)]
pub const macros: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("macros"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1428usize,
        start_col: 4usize,
        end_line: 1428usize,
        end_col: 10usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(macros())),
};
fn macros() {
    use std::string::String;
    fn a() -> Either<u32, u32> {
        let x: u32 = match Right(1337u32) {
            crate::Left(val) => val,
            crate::Right(err) => return crate::Right(::core::convert::From::from(err)),
        };
        Left(x * 2)
    }
    match (&a(), &Right(1337)) {
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
    fn b() -> Either<String, &'static str> {
        Right(
            match Left("foo bar") {
                crate::Left(err) => return crate::Left(::core::convert::From::from(err)),
                crate::Right(val) => val,
            },
        )
    }
    match (&b(), &Left(String::from("foo bar"))) {
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
#[rustc_test_marker = "deref"]
#[doc(hidden)]
pub const deref: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("deref"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1444usize,
        start_col: 4usize,
        end_line: 1444usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(deref())),
};
fn deref() {
    use std::string::String;
    fn is_str(_: &str) {}
    let value: Either<String, &str> = Left(String::from("test"));
    is_str(&*value);
}
extern crate test;
#[rustc_test_marker = "iter"]
#[doc(hidden)]
pub const iter: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("iter"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1453usize,
        start_col: 4usize,
        end_line: 1453usize,
        end_col: 8usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(iter())),
};
fn iter() {
    let x = 3;
    let mut iter = match x {
        3 => Left(0..10),
        _ => Right(17..),
    };
    match (&iter.next(), &Some(0)) {
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
    match (&iter.count(), &9) {
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
#[rustc_test_marker = "seek"]
#[doc(hidden)]
pub const seek: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("seek"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1465usize,
        start_col: 4usize,
        end_line: 1465usize,
        end_col: 8usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(seek())),
};
fn seek() {
    use std::io;
    let use_empty = false;
    let mut mockdata = [0x00; 256];
    for i in 0..256 {
        mockdata[i] = i as u8;
    }
    let mut reader = if use_empty {
        Left(io::Cursor::new([]))
    } else {
        Right(io::Cursor::new(&mockdata[..]))
    };
    let mut buf = [0u8; 16];
    match (&reader.read(&mut buf).unwrap(), &buf.len()) {
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
    match (&buf, &mockdata[..buf.len()]) {
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
    match (&reader.read(&mut buf).unwrap(), &buf.len()) {
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
    match (&buf, &mockdata[..buf.len()]) {
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
    reader.seek(io::SeekFrom::Start(0)).unwrap();
    match (&reader.read(&mut buf).unwrap(), &buf.len()) {
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
    match (&buf, &mockdata[..buf.len()]) {
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
#[rustc_test_marker = "read_write"]
#[doc(hidden)]
pub const read_write: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("read_write"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1496usize,
        start_col: 4usize,
        end_line: 1496usize,
        end_col: 14usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(read_write()),
    ),
};
fn read_write() {
    use std::io;
    let use_stdio = false;
    let mockdata = [0xff; 256];
    let mut reader = if use_stdio { Left(io::stdin()) } else { Right(&mockdata[..]) };
    let mut buf = [0u8; 16];
    match (&reader.read(&mut buf).unwrap(), &buf.len()) {
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
    match (&&buf, &&mockdata[..buf.len()]) {
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
    let mut mockbuf = [0u8; 256];
    let mut writer = if use_stdio {
        Left(io::stdout())
    } else {
        Right(&mut mockbuf[..])
    };
    let buf = [1u8; 16];
    match (&writer.write(&buf).unwrap(), &buf.len()) {
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
#[rustc_test_marker = "error"]
#[doc(hidden)]
pub const error: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("error"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "src/lib.rs",
        start_line: 1524usize,
        start_col: 4usize,
        end_line: 1524usize,
        end_col: 9usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::UnitTest,
    },
    testfn: test::StaticTestFn(#[coverage(off)] || test::assert_test_result(error())),
};
fn error() {
    let invalid_utf8 = b"\xff";
    #[allow(invalid_from_utf8)]
    let res = if let Err(error) = ::std::str::from_utf8(invalid_utf8) {
        Err(Left(error))
    } else if let Err(error) = "x".parse::<i32>() {
        Err(Right(error))
    } else {
        Ok(())
    };
    if !res.is_err() {
        ::core::panicking::panic("assertion failed: res.is_err()")
    }
    #[allow(deprecated)] res.unwrap_err().description();
}
fn _unsized_ref_propagation() {
    {
        fn check_ref<T: AsRef<str>>() {}
        fn propagate_ref<T1: AsRef<str>, T2: AsRef<str>>() {
            check_ref::<Either<T1, T2>>()
        }
        fn check_mut<T: AsMut<str>>() {}
        fn propagate_mut<T1: AsMut<str>, T2: AsMut<str>>() {
            check_mut::<Either<T1, T2>>()
        }
    };
    fn check_array_ref<T: AsRef<[Item]>, Item>() {}
    fn check_array_mut<T: AsMut<[Item]>, Item>() {}
    fn propagate_array_ref<T1: AsRef<[Item]>, T2: AsRef<[Item]>, Item>() {
        check_array_ref::<Either<T1, T2>, _>()
    }
    fn propagate_array_mut<T1: AsMut<[Item]>, T2: AsMut<[Item]>, Item>() {
        check_array_mut::<Either<T1, T2>, _>()
    }
}
fn _unsized_std_propagation() {
    {
        fn check_ref<T: AsRef<::std::path::Path>>() {}
        fn propagate_ref<T1: AsRef<::std::path::Path>, T2: AsRef<::std::path::Path>>() {
            check_ref::<Either<T1, T2>>()
        }
        fn check_mut<T: AsMut<::std::path::Path>>() {}
        fn propagate_mut<T1: AsMut<::std::path::Path>, T2: AsMut<::std::path::Path>>() {
            check_mut::<Either<T1, T2>>()
        }
    };
    {
        fn check_ref<T: AsRef<::std::ffi::OsStr>>() {}
        fn propagate_ref<T1: AsRef<::std::ffi::OsStr>, T2: AsRef<::std::ffi::OsStr>>() {
            check_ref::<Either<T1, T2>>()
        }
        fn check_mut<T: AsMut<::std::ffi::OsStr>>() {}
        fn propagate_mut<T1: AsMut<::std::ffi::OsStr>, T2: AsMut<::std::ffi::OsStr>>() {
            check_mut::<Either<T1, T2>>()
        }
    };
    {
        fn check_ref<T: AsRef<::std::ffi::CStr>>() {}
        fn propagate_ref<T1: AsRef<::std::ffi::CStr>, T2: AsRef<::std::ffi::CStr>>() {
            check_ref::<Either<T1, T2>>()
        }
        fn check_mut<T: AsMut<::std::ffi::CStr>>() {}
        fn propagate_mut<T1: AsMut<::std::ffi::CStr>, T2: AsMut<::std::ffi::CStr>>() {
            check_mut::<Either<T1, T2>>()
        }
    };
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&basic, &deref, &error, &iter, &macros, &read_write, &seek])
}
