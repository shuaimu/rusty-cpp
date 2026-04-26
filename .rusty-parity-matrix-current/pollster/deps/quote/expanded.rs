#![feature(prelude_import)]
//! [![github]](https://github.com/dtolnay/quote)&ensp;[![crates-io]](https://crates.io/crates/quote)&ensp;[![docs-rs]](https://docs.rs/quote)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! This crate provides the [`quote!`] macro for turning Rust syntax tree data
//! structures into tokens of source code.
//!
//! Procedural macros in Rust receive a stream of tokens as input, execute
//! arbitrary Rust code to determine how to manipulate those tokens, and produce
//! a stream of tokens to hand back to the compiler to compile into the caller's
//! crate. Quasi-quoting is a solution to one piece of that &mdash; producing
//! tokens to return to the compiler.
//!
//! The idea of quasi-quoting is that we write *code* that we treat as *data*.
//! Within the `quote!` macro, we can write what looks like code to our text
//! editor or IDE. We get all the benefits of the editor's brace matching,
//! syntax highlighting, indentation, and maybe autocompletion. But rather than
//! compiling that as code into the current crate, we can treat it as data, pass
//! it around, mutate it, and eventually hand it back to the compiler as tokens
//! to compile into the macro caller's crate.
//!
//! This crate is motivated by the procedural macro use case, but is a
//! general-purpose Rust quasi-quoting library and is not specific to procedural
//! macros.
//!
//! ```toml
//! [dependencies]
//! quote = "1.0"
//! ```
//!
//! <br>
//!
//! # Example
//!
//! The following quasi-quoted block of code is something you might find in [a]
//! procedural macro having to do with data structure serialization. The `#var`
//! syntax performs interpolation of runtime variables into the quoted tokens.
//! Check out the documentation of the [`quote!`] macro for more detail about
//! the syntax. See also the [`quote_spanned!`] macro which is important for
//! implementing hygienic procedural macros.
//!
//! [a]: https://serde.rs/
//!
//! ```
//! # use quote::quote;
//! #
//! # let generics = "";
//! # let where_clause = "";
//! # let field_ty = "";
//! # let item_ty = "";
//! # let path = "";
//! # let value = "";
//! #
//! let tokens = quote! {
//!     struct SerializeWith #generics #where_clause {
//!         value: &'a #field_ty,
//!         phantom: core::marker::PhantomData<#item_ty>,
//!     }
//!
//!     impl #generics serde::Serialize for SerializeWith #generics #where_clause {
//!         fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//!         where
//!             S: serde::Serializer,
//!         {
//!             #path(self.value, serializer)
//!         }
//!     }
//!
//!     SerializeWith {
//!         value: #value,
//!         phantom: core::marker::PhantomData::<#item_ty>,
//!     }
//! };
//! ```
//!
//! <br>
//!
//! # Non-macro code generators
//!
//! When using `quote` in a build.rs or main.rs and writing the output out to a
//! file, consider having the code generator pass the tokens through
//! [prettyplease] before writing. This way if an error occurs in the generated
//! code it is convenient for a human to read and debug.
//!
//! [prettyplease]: https://github.com/dtolnay/prettyplease
#![no_std]
#![doc(html_root_url = "https://docs.rs/quote/1.0.45")]
#![allow(
    clippy::doc_markdown,
    clippy::elidable_lifetime_names,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::needless_lifetimes,
    clippy::wrong_self_convention,
)]
extern crate core;
#[prelude_import]
use core::prelude::rust_2021::*;
extern crate alloc;
extern crate std;
mod ext {
    use super::ToTokens;
    use core::iter;
    use proc_macro2::{TokenStream, TokenTree};
    /// TokenStream extension trait with methods for appending tokens.
    ///
    /// This trait is sealed and cannot be implemented outside of the `quote` crate.
    pub trait TokenStreamExt: private::Sealed {
        /// For use by `ToTokens` implementations.
        ///
        /// Appends the token specified to this list of tokens.
        fn append<U>(&mut self, token: U)
        where
            U: Into<TokenTree>;
        /// For use by `ToTokens` implementations.
        ///
        /// ```
        /// # use quote::{quote, TokenStreamExt, ToTokens};
        /// # use proc_macro2::TokenStream;
        /// #
        /// struct X;
        ///
        /// impl ToTokens for X {
        ///     fn to_tokens(&self, tokens: &mut TokenStream) {
        ///         tokens.append_all(&[true, false]);
        ///     }
        /// }
        ///
        /// let tokens = quote!(#X);
        /// assert_eq!(tokens.to_string(), "true false");
        /// ```
        fn append_all<I>(&mut self, iter: I)
        where
            I: IntoIterator,
            I::Item: ToTokens;
        /// For use by `ToTokens` implementations.
        ///
        /// Appends all of the items in the iterator `I`, separated by the tokens
        /// `U`.
        fn append_separated<I, U>(&mut self, iter: I, op: U)
        where
            I: IntoIterator,
            I::Item: ToTokens,
            U: ToTokens;
        /// For use by `ToTokens` implementations.
        ///
        /// Appends all tokens in the iterator `I`, appending `U` after each
        /// element, including after the last element of the iterator.
        fn append_terminated<I, U>(&mut self, iter: I, term: U)
        where
            I: IntoIterator,
            I::Item: ToTokens,
            U: ToTokens;
    }
    impl TokenStreamExt for TokenStream {
        fn append<U>(&mut self, token: U)
        where
            U: Into<TokenTree>,
        {
            self.extend(iter::once(token.into()));
        }
        fn append_all<I>(&mut self, iter: I)
        where
            I: IntoIterator,
            I::Item: ToTokens,
        {
            do_append_all(self, iter.into_iter());
            fn do_append_all<I>(stream: &mut TokenStream, iter: I)
            where
                I: Iterator,
                I::Item: ToTokens,
            {
                for token in iter {
                    token.to_tokens(stream);
                }
            }
        }
        fn append_separated<I, U>(&mut self, iter: I, op: U)
        where
            I: IntoIterator,
            I::Item: ToTokens,
            U: ToTokens,
        {
            do_append_separated(self, iter.into_iter(), op);
            fn do_append_separated<I, U>(stream: &mut TokenStream, iter: I, op: U)
            where
                I: Iterator,
                I::Item: ToTokens,
                U: ToTokens,
            {
                let mut first = true;
                for token in iter {
                    if !first {
                        op.to_tokens(stream);
                    }
                    first = false;
                    token.to_tokens(stream);
                }
            }
        }
        fn append_terminated<I, U>(&mut self, iter: I, term: U)
        where
            I: IntoIterator,
            I::Item: ToTokens,
            U: ToTokens,
        {
            do_append_terminated(self, iter.into_iter(), term);
            fn do_append_terminated<I, U>(stream: &mut TokenStream, iter: I, term: U)
            where
                I: Iterator,
                I::Item: ToTokens,
                U: ToTokens,
            {
                for token in iter {
                    token.to_tokens(stream);
                    term.to_tokens(stream);
                }
            }
        }
    }
    mod private {
        use proc_macro2::TokenStream;
        pub trait Sealed {}
        impl Sealed for TokenStream {}
    }
}
mod format {}
mod ident_fragment {
    use alloc::borrow::{Cow, ToOwned};
    use alloc::string::{String, ToString};
    use core::fmt;
    use proc_macro2::{Ident, Span};
    /// Specialized formatting trait used by `format_ident!`.
    ///
    /// [`Ident`] arguments formatted using this trait will have their `r#` prefix
    /// stripped, if present.
    ///
    /// See [`format_ident!`] for more information.
    ///
    /// [`format_ident!`]: crate::format_ident
    pub trait IdentFragment {
        /// Format this value as an identifier fragment.
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
        /// Span associated with this `IdentFragment`.
        ///
        /// If non-`None`, may be inherited by formatted identifiers.
        fn span(&self) -> Option<Span> {
            None
        }
    }
    impl<T: IdentFragment + ?Sized> IdentFragment for &T {
        fn span(&self) -> Option<Span> {
            <T as IdentFragment>::span(*self)
        }
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            IdentFragment::fmt(*self, f)
        }
    }
    impl<T: IdentFragment + ?Sized> IdentFragment for &mut T {
        fn span(&self) -> Option<Span> {
            <T as IdentFragment>::span(*self)
        }
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            IdentFragment::fmt(*self, f)
        }
    }
    impl IdentFragment for Ident {
        fn span(&self) -> Option<Span> {
            Some(self.span())
        }
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let id = self.to_string();
            if let Some(id) = id.strip_prefix("r#") {
                fmt::Display::fmt(id, f)
            } else {
                fmt::Display::fmt(&id[..], f)
            }
        }
    }
    impl<T> IdentFragment for Cow<'_, T>
    where
        T: IdentFragment + ToOwned + ?Sized,
    {
        fn span(&self) -> Option<Span> {
            T::span(self)
        }
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            T::fmt(self, f)
        }
    }
    impl IdentFragment for bool {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for str {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for String {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for char {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for u8 {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for u16 {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for u32 {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for u64 {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for u128 {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
    impl IdentFragment for usize {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Display::fmt(self, f)
        }
    }
}
mod to_tokens {
    use super::TokenStreamExt;
    use std::sync::Arc;
    use alloc::borrow::{Cow, ToOwned};
    use alloc::boxed::Box;
    use alloc::ffi::CString;
    use alloc::rc::Rc;
    use alloc::string::String;
    use core::ffi::CStr;
    use core::iter;
    use proc_macro2::{Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};
    /// Types that can be interpolated inside a `quote!` invocation.
    pub trait ToTokens {
        /// Write `self` to the given `TokenStream`.
        ///
        /// The token append methods provided by the [`TokenStreamExt`] extension
        /// trait may be useful for implementing `ToTokens`.
        ///
        /// # Example
        ///
        /// Example implementation for a struct representing Rust paths like
        /// `std::cmp::PartialEq`:
        ///
        /// ```
        /// use proc_macro2::{TokenTree, Spacing, Span, Punct, TokenStream};
        /// use quote::{TokenStreamExt, ToTokens};
        ///
        /// pub struct Path {
        ///     pub global: bool,
        ///     pub segments: Vec<PathSegment>,
        /// }
        ///
        /// impl ToTokens for Path {
        ///     fn to_tokens(&self, tokens: &mut TokenStream) {
        ///         for (i, segment) in self.segments.iter().enumerate() {
        ///             if i > 0 || self.global {
        ///                 // Double colon `::`
        ///                 tokens.append(Punct::new(':', Spacing::Joint));
        ///                 tokens.append(Punct::new(':', Spacing::Alone));
        ///             }
        ///             segment.to_tokens(tokens);
        ///         }
        ///     }
        /// }
        /// #
        /// # pub struct PathSegment;
        /// #
        /// # impl ToTokens for PathSegment {
        /// #     fn to_tokens(&self, tokens: &mut TokenStream) {
        /// #         unimplemented!()
        /// #     }
        /// # }
        /// ```
        fn to_tokens(&self, tokens: &mut TokenStream);
        /// Convert `self` directly into a `TokenStream` object.
        ///
        /// This method is implicitly implemented using `to_tokens`, and acts as a
        /// convenience method for consumers of the `ToTokens` trait.
        fn to_token_stream(&self) -> TokenStream {
            let mut tokens = TokenStream::new();
            self.to_tokens(&mut tokens);
            tokens
        }
        /// Convert `self` directly into a `TokenStream` object.
        ///
        /// This method is implicitly implemented using `to_tokens`, and acts as a
        /// convenience method for consumers of the `ToTokens` trait.
        fn into_token_stream(self) -> TokenStream
        where
            Self: Sized,
        {
            self.to_token_stream()
        }
    }
    impl<T: ?Sized + ToTokens> ToTokens for &T {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<T: ?Sized + ToTokens> ToTokens for &mut T {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<'a, T: ?Sized + ToOwned + ToTokens> ToTokens for Cow<'a, T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<T: ?Sized + ToTokens> ToTokens for Box<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<T: ?Sized + ToTokens> ToTokens for Rc<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<T: ?Sized + ToTokens> ToTokens for Arc<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            (**self).to_tokens(tokens);
        }
    }
    impl<T: ToTokens> ToTokens for Option<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            if let Some(t) = self {
                t.to_tokens(tokens);
            }
        }
    }
    impl ToTokens for str {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::string(self));
        }
    }
    impl ToTokens for String {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.as_str().to_tokens(tokens);
        }
    }
    impl ToTokens for i8 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::i8_suffixed(*self));
        }
    }
    impl ToTokens for i16 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::i16_suffixed(*self));
        }
    }
    impl ToTokens for i32 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::i32_suffixed(*self));
        }
    }
    impl ToTokens for i64 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::i64_suffixed(*self));
        }
    }
    impl ToTokens for i128 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::i128_suffixed(*self));
        }
    }
    impl ToTokens for isize {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::isize_suffixed(*self));
        }
    }
    impl ToTokens for u8 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::u8_suffixed(*self));
        }
    }
    impl ToTokens for u16 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::u16_suffixed(*self));
        }
    }
    impl ToTokens for u32 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::u32_suffixed(*self));
        }
    }
    impl ToTokens for u64 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::u64_suffixed(*self));
        }
    }
    impl ToTokens for u128 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::u128_suffixed(*self));
        }
    }
    impl ToTokens for usize {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::usize_suffixed(*self));
        }
    }
    impl ToTokens for f32 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::f32_suffixed(*self));
        }
    }
    impl ToTokens for f64 {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::f64_suffixed(*self));
        }
    }
    impl ToTokens for char {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::character(*self));
        }
    }
    impl ToTokens for bool {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let word = if *self { "true" } else { "false" };
            tokens.append(Ident::new(word, Span::call_site()));
        }
    }
    impl ToTokens for CStr {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::c_string(self));
        }
    }
    impl ToTokens for CString {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(Literal::c_string(self));
        }
    }
    impl ToTokens for Group {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(self.clone());
        }
    }
    impl ToTokens for Ident {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(self.clone());
        }
    }
    impl ToTokens for Punct {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(self.clone());
        }
    }
    impl ToTokens for Literal {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(self.clone());
        }
    }
    impl ToTokens for TokenTree {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append(self.clone());
        }
    }
    impl ToTokens for TokenStream {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.extend(iter::once(self.clone()));
        }
        fn into_token_stream(self) -> TokenStream {
            self
        }
    }
}
#[doc(hidden)]
#[path = "runtime.rs"]
pub mod __private {
    use self::get_span::{GetSpan, GetSpanBase, GetSpanInner};
    use crate::{IdentFragment, ToTokens, TokenStreamExt};
    use core::fmt;
    use core::iter;
    use core::ops::BitOr;
    use proc_macro2::{Group, Ident, Punct, Spacing, TokenTree};
    #[doc(hidden)]
    pub use alloc::format;
    #[doc(hidden)]
    pub use core::option::Option;
    #[doc(hidden)]
    pub use core::stringify;
    #[doc(hidden)]
    pub type Delimiter = proc_macro2::Delimiter;
    #[doc(hidden)]
    pub type Span = proc_macro2::Span;
    #[doc(hidden)]
    pub type TokenStream = proc_macro2::TokenStream;
    #[doc(hidden)]
    pub struct HasIterator<const B: bool>;
    impl BitOr<HasIterator<false>> for HasIterator<false> {
        type Output = HasIterator<false>;
        fn bitor(self, _rhs: HasIterator<false>) -> HasIterator<false> {
            HasIterator::<false>
        }
    }
    impl BitOr<HasIterator<false>> for HasIterator<true> {
        type Output = HasIterator<true>;
        fn bitor(self, _rhs: HasIterator<false>) -> HasIterator<true> {
            HasIterator::<true>
        }
    }
    impl BitOr<HasIterator<true>> for HasIterator<false> {
        type Output = HasIterator<true>;
        fn bitor(self, _rhs: HasIterator<true>) -> HasIterator<true> {
            HasIterator::<true>
        }
    }
    impl BitOr<HasIterator<true>> for HasIterator<true> {
        type Output = HasIterator<true>;
        fn bitor(self, _rhs: HasIterator<true>) -> HasIterator<true> {
            HasIterator::<true>
        }
    }
    #[doc(hidden)]
    #[diagnostic::on_unimplemented(
        message = "repetition contains no interpolated value that is an iterator",
        label = "none of the values interpolated inside this repetition are iterable"
    )]
    pub trait CheckHasIterator<const B: bool>: Sized {
        fn check(self) {}
    }
    impl CheckHasIterator<true> for HasIterator<true> {}
    /// Extension traits used by the implementation of `quote!`. These are defined
    /// in separate traits, rather than as a single trait due to ambiguity issues.
    ///
    /// These traits expose a `quote_into_iter` method which should allow calling
    /// whichever impl happens to be applicable. Calling that method repeatedly on
    /// the returned value should be idempotent.
    #[doc(hidden)]
    pub mod ext {
        use super::{HasIterator, RepInterp};
        use crate::ToTokens;
        use alloc::collections::btree_set::{self, BTreeSet};
        use alloc::vec::Vec;
        use core::slice;
        /// Extension trait providing the `quote_into_iter` method on iterators.
        #[doc(hidden)]
        pub trait RepIteratorExt: Iterator + Sized {
            fn quote_into_iter(self) -> (Self, HasIterator<true>) {
                (self, HasIterator::<true>)
            }
        }
        impl<T: Iterator> RepIteratorExt for T {}
        /// Extension trait providing the `quote_into_iter` method for
        /// non-iterable types. These types interpolate the same value in each
        /// iteration of the repetition.
        #[doc(hidden)]
        pub trait RepToTokensExt {
            /// Pretend to be an iterator for the purposes of `quote_into_iter`.
            /// This allows repeated calls to `quote_into_iter` to continue
            /// correctly returning HasIterator<false>.
            fn next(&self) -> Option<&Self> {
                Some(self)
            }
            fn quote_into_iter(&self) -> (&Self, HasIterator<false>) {
                (self, HasIterator::<false>)
            }
        }
        impl<T: ToTokens + ?Sized> RepToTokensExt for T {}
        /// Extension trait providing the `quote_into_iter` method for types that
        /// can be referenced as an iterator.
        #[doc(hidden)]
        pub trait RepAsIteratorExt<'q> {
            type Iter: Iterator;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>);
        }
        impl<'q, T: RepAsIteratorExt<'q> + ?Sized> RepAsIteratorExt<'q> for &T {
            type Iter = T::Iter;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                <T as RepAsIteratorExt>::quote_into_iter(*self)
            }
        }
        impl<'q, T: RepAsIteratorExt<'q> + ?Sized> RepAsIteratorExt<'q> for &mut T {
            type Iter = T::Iter;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                <T as RepAsIteratorExt>::quote_into_iter(*self)
            }
        }
        impl<'q, T: 'q> RepAsIteratorExt<'q> for [T] {
            type Iter = slice::Iter<'q, T>;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                (self.iter(), HasIterator::<true>)
            }
        }
        impl<'q, T: 'q, const N: usize> RepAsIteratorExt<'q> for [T; N] {
            type Iter = slice::Iter<'q, T>;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                (self.iter(), HasIterator::<true>)
            }
        }
        impl<'q, T: 'q> RepAsIteratorExt<'q> for Vec<T> {
            type Iter = slice::Iter<'q, T>;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                (self.iter(), HasIterator::<true>)
            }
        }
        impl<'q, T: 'q> RepAsIteratorExt<'q> for BTreeSet<T> {
            type Iter = btree_set::Iter<'q, T>;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                (self.iter(), HasIterator::<true>)
            }
        }
        impl<'q, T: RepAsIteratorExt<'q>> RepAsIteratorExt<'q> for RepInterp<T> {
            type Iter = T::Iter;
            fn quote_into_iter(&'q self) -> (Self::Iter, HasIterator<true>) {
                self.0.quote_into_iter()
            }
        }
    }
    #[doc(hidden)]
    pub struct RepInterp<T>(pub T);
    #[automatically_derived]
    impl<T: ::core::marker::Copy> ::core::marker::Copy for RepInterp<T> {}
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for RepInterp<T> {
        #[inline]
        fn clone(&self) -> RepInterp<T> {
            RepInterp(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<T> RepInterp<T> {
        pub fn next(self) -> Option<T> {
            Some(self.0)
        }
    }
    impl<T: Iterator> Iterator for RepInterp<T> {
        type Item = T::Item;
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
    }
    impl<T: ToTokens> ToTokens for RepInterp<T> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            self.0.to_tokens(tokens);
        }
    }
    #[doc(hidden)]
    #[inline]
    pub fn get_span<T>(span: T) -> GetSpan<T> {
        GetSpan(GetSpanInner(GetSpanBase(span)))
    }
    mod get_span {
        use core::ops::Deref;
        use proc_macro2::extra::DelimSpan;
        use proc_macro2::Span;
        pub struct GetSpan<T>(pub(crate) GetSpanInner<T>);
        pub struct GetSpanInner<T>(pub(crate) GetSpanBase<T>);
        pub struct GetSpanBase<T>(pub(crate) T);
        impl GetSpan<Span> {
            #[inline]
            pub fn __into_span(self) -> Span {
                ((self.0).0).0
            }
        }
        impl GetSpanInner<DelimSpan> {
            #[inline]
            pub fn __into_span(&self) -> Span {
                (self.0).0.join()
            }
        }
        impl<T> GetSpanBase<T> {
            #[allow(clippy::unused_self)]
            pub fn __into_span(&self) -> T {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
        impl<T> Deref for GetSpan<T> {
            type Target = GetSpanInner<T>;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl<T> Deref for GetSpanInner<T> {
            type Target = GetSpanBase<T>;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    }
    #[doc(hidden)]
    pub fn push_group(
        tokens: &mut TokenStream,
        delimiter: Delimiter,
        inner: TokenStream,
    ) {
        tokens.append(Group::new(delimiter, inner));
    }
    #[doc(hidden)]
    pub fn push_group_spanned(
        tokens: &mut TokenStream,
        span: Span,
        delimiter: Delimiter,
        inner: TokenStream,
    ) {
        let mut g = Group::new(delimiter, inner);
        g.set_span(span);
        tokens.append(g);
    }
    #[doc(hidden)]
    pub fn parse(tokens: &mut TokenStream, s: &str) {
        let s: TokenStream = s.parse().expect("invalid token stream");
        tokens.extend(iter::once(s));
    }
    #[doc(hidden)]
    pub fn parse_spanned(tokens: &mut TokenStream, span: Span, s: &str) {
        let s: TokenStream = s.parse().expect("invalid token stream");
        for token in s {
            tokens.append(respan_token_tree(token, span));
        }
    }
    fn respan_token_tree(mut token: TokenTree, span: Span) -> TokenTree {
        match &mut token {
            TokenTree::Group(g) => {
                let mut tokens = TokenStream::new();
                for token in g.stream() {
                    tokens.append(respan_token_tree(token, span));
                }
                *g = Group::new(g.delimiter(), tokens);
                g.set_span(span);
            }
            other => other.set_span(span),
        }
        token
    }
    #[doc(hidden)]
    pub fn push_ident(tokens: &mut TokenStream, s: &str) {
        let span = Span::call_site();
        push_ident_spanned(tokens, span, s);
    }
    #[doc(hidden)]
    pub fn push_ident_spanned(tokens: &mut TokenStream, span: Span, s: &str) {
        tokens.append(ident_maybe_raw(s, span));
    }
    #[doc(hidden)]
    pub fn push_lifetime(tokens: &mut TokenStream, lifetime: &str) {
        tokens.append(TokenTree::Punct(Punct::new('\'', Spacing::Joint)));
        tokens
            .append(
                TokenTree::Ident(ident_maybe_raw(&lifetime[1..], Span::call_site())),
            );
    }
    #[doc(hidden)]
    pub fn push_lifetime_spanned(tokens: &mut TokenStream, span: Span, lifetime: &str) {
        tokens
            .append(
                TokenTree::Punct({
                    let mut apostrophe = Punct::new('\'', Spacing::Joint);
                    apostrophe.set_span(span);
                    apostrophe
                }),
            );
        tokens.append(TokenTree::Ident(ident_maybe_raw(&lifetime[1..], span)));
    }
    #[doc(hidden)]
    pub fn push_add(tokens: &mut TokenStream) {
        tokens.append(Punct::new('+', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_add_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('+', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_add_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('+', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_add_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('+', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_and(tokens: &mut TokenStream) {
        tokens.append(Punct::new('&', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_and_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('&', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_and_and(tokens: &mut TokenStream) {
        tokens.append(Punct::new('&', Spacing::Joint));
        tokens.append(Punct::new('&', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_and_and_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('&', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('&', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_and_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('&', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_and_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('&', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_at(tokens: &mut TokenStream) {
        tokens.append(Punct::new('@', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_at_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('@', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_bang(tokens: &mut TokenStream) {
        tokens.append(Punct::new('!', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_bang_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('!', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_caret(tokens: &mut TokenStream) {
        tokens.append(Punct::new('^', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_caret_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('^', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_caret_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('^', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_caret_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('^', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_colon(tokens: &mut TokenStream) {
        tokens.append(Punct::new(':', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_colon_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new(':', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_colon2(tokens: &mut TokenStream) {
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_colon2_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new(':', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new(':', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_comma(tokens: &mut TokenStream) {
        tokens.append(Punct::new(',', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_comma_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new(',', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_div(tokens: &mut TokenStream) {
        tokens.append(Punct::new('/', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_div_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('/', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_div_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('/', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_div_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('/', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_dot(tokens: &mut TokenStream) {
        tokens.append(Punct::new('.', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_dot_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('.', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_dot2(tokens: &mut TokenStream) {
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Punct::new('.', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_dot2_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('.', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('.', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_dot3(tokens: &mut TokenStream) {
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Punct::new('.', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_dot3_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('.', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('.', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('.', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_dot_dot_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_dot_dot_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('.', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('.', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_eq_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('=', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_eq_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('=', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_ge(tokens: &mut TokenStream) {
        tokens.append(Punct::new('>', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_ge_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('>', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_gt(tokens: &mut TokenStream) {
        tokens.append(Punct::new('>', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_gt_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('>', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_le(tokens: &mut TokenStream) {
        tokens.append(Punct::new('<', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_le_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('<', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_lt(tokens: &mut TokenStream) {
        tokens.append(Punct::new('<', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_lt_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('<', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_mul_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('*', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_mul_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('*', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_ne(tokens: &mut TokenStream) {
        tokens.append(Punct::new('!', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_ne_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('!', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_or(tokens: &mut TokenStream) {
        tokens.append(Punct::new('|', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_or_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('|', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_or_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('|', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_or_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('|', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_or_or(tokens: &mut TokenStream) {
        tokens.append(Punct::new('|', Spacing::Joint));
        tokens.append(Punct::new('|', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_or_or_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('|', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('|', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_pound(tokens: &mut TokenStream) {
        tokens.append(Punct::new('#', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_pound_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('#', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_question(tokens: &mut TokenStream) {
        tokens.append(Punct::new('?', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_question_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('?', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_rarrow(tokens: &mut TokenStream) {
        tokens.append(Punct::new('-', Spacing::Joint));
        tokens.append(Punct::new('>', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_rarrow_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('-', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('>', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_larrow(tokens: &mut TokenStream) {
        tokens.append(Punct::new('<', Spacing::Joint));
        tokens.append(Punct::new('-', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_larrow_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('<', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('-', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_rem(tokens: &mut TokenStream) {
        tokens.append(Punct::new('%', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_rem_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('%', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_rem_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('%', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_rem_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('%', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_fat_arrow(tokens: &mut TokenStream) {
        tokens.append(Punct::new('=', Spacing::Joint));
        tokens.append(Punct::new('>', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_fat_arrow_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('=', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('>', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_semi(tokens: &mut TokenStream) {
        tokens.append(Punct::new(';', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_semi_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new(';', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_shl(tokens: &mut TokenStream) {
        tokens.append(Punct::new('<', Spacing::Joint));
        tokens.append(Punct::new('<', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_shl_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('<', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('<', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_shl_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('<', Spacing::Joint));
        tokens.append(Punct::new('<', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_shl_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('<', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('<', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_shr(tokens: &mut TokenStream) {
        tokens.append(Punct::new('>', Spacing::Joint));
        tokens.append(Punct::new('>', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_shr_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('>', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('>', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_shr_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('>', Spacing::Joint));
        tokens.append(Punct::new('>', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_shr_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('>', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('>', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_star(tokens: &mut TokenStream) {
        tokens.append(Punct::new('*', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_star_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('*', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_sub(tokens: &mut TokenStream) {
        tokens.append(Punct::new('-', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_sub_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('-', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_sub_eq(tokens: &mut TokenStream) {
        tokens.append(Punct::new('-', Spacing::Joint));
        tokens.append(Punct::new('=', Spacing::Alone));
    }
    #[doc(hidden)]
    pub fn push_sub_eq_spanned(tokens: &mut TokenStream, span: Span) {
        let mut punct = Punct::new('-', Spacing::Joint);
        punct.set_span(span);
        tokens.append(punct);
        let mut punct = Punct::new('=', Spacing::Alone);
        punct.set_span(span);
        tokens.append(punct);
    }
    #[doc(hidden)]
    pub fn push_underscore(tokens: &mut TokenStream) {
        push_underscore_spanned(tokens, Span::call_site());
    }
    #[doc(hidden)]
    pub fn push_underscore_spanned(tokens: &mut TokenStream, span: Span) {
        tokens.append(Ident::new("_", span));
    }
    #[doc(hidden)]
    pub fn mk_ident(id: &str, span: Option<Span>) -> Ident {
        let span = span.unwrap_or_else(Span::call_site);
        ident_maybe_raw(id, span)
    }
    fn ident_maybe_raw(id: &str, span: Span) -> Ident {
        if let Some(id) = id.strip_prefix("r#") {
            Ident::new_raw(id, span)
        } else {
            Ident::new(id, span)
        }
    }
    #[doc(hidden)]
    pub struct IdentFragmentAdapter<T: IdentFragment>(pub T);
    #[automatically_derived]
    impl<T: ::core::marker::Copy + IdentFragment> ::core::marker::Copy
    for IdentFragmentAdapter<T> {}
    #[automatically_derived]
    impl<T: ::core::clone::Clone + IdentFragment> ::core::clone::Clone
    for IdentFragmentAdapter<T> {
        #[inline]
        fn clone(&self) -> IdentFragmentAdapter<T> {
            IdentFragmentAdapter(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<T: IdentFragment> IdentFragmentAdapter<T> {
        pub fn span(&self) -> Option<Span> {
            self.0.span()
        }
    }
    impl<T: IdentFragment> fmt::Display for IdentFragmentAdapter<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            IdentFragment::fmt(&self.0, f)
        }
    }
    impl<T: IdentFragment + fmt::Octal> fmt::Octal for IdentFragmentAdapter<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Octal::fmt(&self.0, f)
        }
    }
    impl<T: IdentFragment + fmt::LowerHex> fmt::LowerHex for IdentFragmentAdapter<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::LowerHex::fmt(&self.0, f)
        }
    }
    impl<T: IdentFragment + fmt::UpperHex> fmt::UpperHex for IdentFragmentAdapter<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::UpperHex::fmt(&self.0, f)
        }
    }
    impl<T: IdentFragment + fmt::Binary> fmt::Binary for IdentFragmentAdapter<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Binary::fmt(&self.0, f)
        }
    }
}
pub use crate::ext::TokenStreamExt;
pub use crate::ident_fragment::IdentFragment;
pub use crate::to_tokens::ToTokens;
#[doc(hidden)]
pub mod spanned {
    use crate::ToTokens;
    use proc_macro2::extra::DelimSpan;
    use proc_macro2::{Span, TokenStream};
    pub trait Spanned: private::Sealed {
        fn __span(&self) -> Span;
    }
    impl Spanned for Span {
        fn __span(&self) -> Span {
            *self
        }
    }
    impl Spanned for DelimSpan {
        fn __span(&self) -> Span {
            self.join()
        }
    }
    impl<T: ?Sized + ToTokens> Spanned for T {
        fn __span(&self) -> Span {
            join_spans(self.into_token_stream())
        }
    }
    fn join_spans(tokens: TokenStream) -> Span {
        let mut iter = tokens.into_iter().map(|tt| tt.span());
        let Some(first) = iter.next() else {
            return Span::call_site();
        };
        iter.fold(None, |_prev, next| Some(next))
            .and_then(|last| first.join(last))
            .unwrap_or(first)
    }
    mod private {
        use crate::ToTokens;
        use proc_macro2::extra::DelimSpan;
        use proc_macro2::Span;
        pub trait Sealed {}
        impl Sealed for Span {}
        impl Sealed for DelimSpan {}
        impl<T: ?Sized + ToTokens> Sealed for T {}
    }
}
