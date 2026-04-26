#![feature(prelude_import)]
//! > winnow, making parsing a breeze
//!
//! `winnow` is a parser combinator library
//!
//! Quick links:
//! - [List of combinators][crate::combinator]
//! - [Tutorial][_tutorial::chapter_0]
//! - [Special Topics][_topic]
//! - [Discussions](https://github.com/winnow-rs/winnow/discussions)
//! - [CHANGELOG](https://github.com/winnow-rs/winnow/blob/v1.0.0/CHANGELOG.md) (includes major version migration
//!   guides)
//!
//! ## Aspirations
//!
//! `winnow` aims to be your "do everything" parser, much like people treat regular expressions.
//!
//! In roughly priority order:
//! 1. Support writing parser declaratively while not getting in the way of imperative-style
//!    parsing when needed, working as an open-ended toolbox rather than a close-ended framework.
//! 2. Flexible enough to be used for any application, including parsing strings, binary data,
//!    or separate [lexing and parsing phases][_topic::lexing]
//! 3. Zero-cost abstractions, making it easy to write high performance parsers
//! 4. Easy to use, making it trivial for one-off uses
//!
//! In addition:
//! - Resilient maintainership, including
//!   - Willing to break compatibility rather than batching up breaking changes in large releases
//!   - Leverage feature flags to keep one active branch
//! - We will support the last 6 months of rust releases (MSRV)
//!
//! See also [Special Topic: Why winnow?][crate::_topic::why]
//!
//! ## Example
//!
//! Run
//! ```console
//! $ cargo add winnow
//! ```
//!
//! Then use it to parse:
//! ```rust
//! # #[cfg(all(feature = "alloc", feature = "parser"))] {
/*!use winnow::combinator::seq;
use winnow::prelude::*;
use winnow::token::take_while;
use winnow::Result;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Color {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
}

impl std::str::FromStr for Color {
    // The error must be owned
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex_color.parse(s).map_err(|e| e.to_string())
    }
}

pub(crate) fn hex_color(input: &mut &str) -> Result<Color> {
    seq!(Color {
        _: '#',
        red: hex_primary,
        green: hex_primary,
        blue: hex_primary
    })
    .parse_next(input)
}

fn hex_primary(input: &mut &str) -> Result<u8> {
    take_while(2, |c: char| c.is_ascii_hexdigit())
        .try_map(|input| u8::from_str_radix(input, 16))
        .parse_next(input)
}
*/
//! # }
//! ```
//!
//! See also the [Tutorial][_tutorial::chapter_0] and [Special Topics][_topic]
#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
#[allow(unused_extern_crates)]
extern crate alloc;
pub(crate) mod util {
    #[allow(dead_code)]
    pub(crate) fn from_fn<F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result>(
        f: F,
    ) -> FromFn<F> {
        FromFn(f)
    }
    pub(crate) struct FromFn<F>(
        F,
    )
    where
        F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result;
    impl<F> core::fmt::Debug for FromFn<F>
    where
        F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            (self.0)(f)
        }
    }
    impl<F> core::fmt::Display for FromFn<F>
    where
        F: Fn(&mut core::fmt::Formatter<'_>) -> core::fmt::Result,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            (self.0)(f)
        }
    }
}
#[macro_use]
mod macros {
    mod dispatch {}
    mod seq {}
    mod unordered_seq {}
}
#[macro_use]
pub mod error {
    //! # Error management
    //!
    //! Errors are designed with multiple needs in mind:
    //! - Accumulate more [context][Parser::context] as the error goes up the parser chain
    //! - Distinguish between [recoverable errors,
    //!   unrecoverable errors, and more data is needed][ErrMode]
    //! - Have a very low overhead, as errors are often discarded by the calling parser (examples: `repeat`, `alt`)
    //! - Can be modified according to the user's needs, because some languages need a lot more information
    //! - Help thread-through the [stream][crate::stream]
    //!
    //! To abstract these needs away from the user, generally `winnow` parsers use the [`ModalResult`]
    //! alias, rather than [`Result`].  [`Parser::parse`] is a top-level operation
    //! that can help convert to a `Result` for integrating with your application's error reporting.
    //!
    //! Error types include:
    //! - [`EmptyError`] when the reason for failure doesn't matter
    //! - [`ContextError`]
    //! - [`InputError`] (mostly for testing)
    //! - [`TreeError`] (mostly for testing)
    //! - [Custom errors][crate::_topic::error]
    use alloc::borrow::ToOwned;
    use core::fmt;
    use crate::stream::AsBStr;
    use crate::stream::Stream;
    #[allow(unused_imports)]
    use crate::Parser;
    pub use crate::stream::Needed;
    /// By default, the error type (`E`) is [`ContextError`].
    ///
    /// When integrating into the result of the application, see
    /// - [`Parser::parse`]
    /// - [`ParserError::into_inner`]
    pub type Result<O, E = ContextError> = core::result::Result<O, E>;
    /// [Modal error reporting][ErrMode] for [`Parser::parse_next`]
    ///
    /// - `Ok(O)` is the parsed value
    /// - [`Err(ErrMode<E>)`][ErrMode] is the error along with how to respond to it
    ///
    /// By default, the error type (`E`) is [`ContextError`].
    ///
    /// When integrating into the result of the application, see
    /// - [`Parser::parse`]
    /// - [`ParserError::into_inner`]
    pub type ModalResult<O, E = ContextError> = Result<O, ErrMode<E>>;
    /// Add parse error state to [`ParserError`]s
    ///
    /// Needed for
    /// - [`Partial`][crate::stream::Partial] to track whether the [`Stream`] is [`ErrMode::Incomplete`].
    ///   See also [`crate::_topic::partial`]
    /// - Marking errors as unrecoverable ([`ErrMode::Cut`]) and not retrying alternative parsers.
    ///   See also [`crate::_tutorial::chapter_7#error-cuts`]
    pub enum ErrMode<E> {
        /// There was not enough data to determine the appropriate action
        ///
        /// More data needs to be buffered before retrying the parse.
        ///
        /// This must only be set when the [`Stream`] is [partial][`crate::stream::StreamIsPartial`], like with
        /// [`Partial`][crate::Partial]
        ///
        /// Convert this into an `Backtrack` with [`Parser::complete_err`]
        Incomplete(Needed),
        /// The parser failed with a recoverable error (the default).
        ///
        /// For example, a parser for json values might include a
        /// [`dec_uint`][crate::ascii::dec_uint] as one case in an [`alt`][crate::combinator::alt]
        /// combinator. If it fails, the next case should be tried.
        Backtrack(E),
        /// The parser had an unrecoverable error.
        ///
        /// The parser was on the right branch, so directly report it to the user rather than trying
        /// other branches. You can use [`cut_err()`][crate::combinator::cut_err] combinator to switch
        /// from `ErrMode::Backtrack` to `ErrMode::Cut`.
        ///
        /// For example, one case in an [`alt`][crate::combinator::alt] combinator found a unique prefix
        /// and you want any further errors parsing the case to be reported to the user.
        Cut(E),
    }
    #[automatically_derived]
    impl<E: ::core::fmt::Debug> ::core::fmt::Debug for ErrMode<E> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                ErrMode::Incomplete(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Incomplete",
                        &__self_0,
                    )
                }
                ErrMode::Backtrack(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Backtrack",
                        &__self_0,
                    )
                }
                ErrMode::Cut(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Cut",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl<E: ::core::clone::Clone> ::core::clone::Clone for ErrMode<E> {
        #[inline]
        fn clone(&self) -> ErrMode<E> {
            match self {
                ErrMode::Incomplete(__self_0) => {
                    ErrMode::Incomplete(::core::clone::Clone::clone(__self_0))
                }
                ErrMode::Backtrack(__self_0) => {
                    ErrMode::Backtrack(::core::clone::Clone::clone(__self_0))
                }
                ErrMode::Cut(__self_0) => {
                    ErrMode::Cut(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[automatically_derived]
    impl<E> ::core::marker::StructuralPartialEq for ErrMode<E> {}
    #[automatically_derived]
    impl<E: ::core::cmp::PartialEq> ::core::cmp::PartialEq for ErrMode<E> {
        #[inline]
        fn eq(&self, other: &ErrMode<E>) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (ErrMode::Incomplete(__self_0), ErrMode::Incomplete(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (ErrMode::Backtrack(__self_0), ErrMode::Backtrack(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (ErrMode::Cut(__self_0), ErrMode::Cut(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    impl<E> ErrMode<E> {
        /// Tests if the result is Incomplete
        #[inline]
        pub fn is_incomplete(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                ErrMode::Incomplete(_) => true,
                _ => false,
            }
        }
        /// Prevent backtracking, bubbling the error up to the top
        pub fn cut(self) -> Self {
            match self {
                ErrMode::Backtrack(e) => ErrMode::Cut(e),
                rest => rest,
            }
        }
        /// Enable backtracking support
        pub fn backtrack(self) -> Self {
            match self {
                ErrMode::Cut(e) => ErrMode::Backtrack(e),
                rest => rest,
            }
        }
        /// Applies the given function to the inner error
        pub fn map<E2, F>(self, f: F) -> ErrMode<E2>
        where
            F: FnOnce(E) -> E2,
        {
            match self {
                ErrMode::Incomplete(n) => ErrMode::Incomplete(n),
                ErrMode::Cut(t) => ErrMode::Cut(f(t)),
                ErrMode::Backtrack(t) => ErrMode::Backtrack(f(t)),
            }
        }
        /// Automatically converts between errors if the underlying type supports it
        pub fn convert<F>(self) -> ErrMode<F>
        where
            E: ErrorConvert<F>,
        {
            ErrorConvert::convert(self)
        }
        /// Unwrap the mode, returning the underlying error
        ///
        /// Returns `Err(self)` for [`ErrMode::Incomplete`]
        #[inline(always)]
        pub fn into_inner(self) -> Result<E, Self> {
            match self {
                ErrMode::Backtrack(e) | ErrMode::Cut(e) => Ok(e),
                err @ ErrMode::Incomplete(_) => Err(err),
            }
        }
    }
    impl<I: Stream, E: ParserError<I>> ParserError<I> for ErrMode<E> {
        type Inner = E;
        #[inline(always)]
        fn from_input(input: &I) -> Self {
            ErrMode::Backtrack(E::from_input(input))
        }
        #[inline(always)]
        fn assert(input: &I, message: &'static str) -> Self
        where
            I: core::fmt::Debug,
        {
            ErrMode::Cut(E::assert(input, message))
        }
        #[inline(always)]
        fn incomplete(_input: &I, needed: Needed) -> Self {
            ErrMode::Incomplete(needed)
        }
        #[inline]
        fn append(self, input: &I, token_start: &<I as Stream>::Checkpoint) -> Self {
            match self {
                ErrMode::Backtrack(e) => ErrMode::Backtrack(e.append(input, token_start)),
                e => e,
            }
        }
        fn or(self, other: Self) -> Self {
            match (self, other) {
                (ErrMode::Backtrack(e), ErrMode::Backtrack(o)) => {
                    ErrMode::Backtrack(e.or(o))
                }
                (ErrMode::Incomplete(e), _) | (_, ErrMode::Incomplete(e)) => {
                    ErrMode::Incomplete(e)
                }
                (ErrMode::Cut(e), _) | (_, ErrMode::Cut(e)) => ErrMode::Cut(e),
            }
        }
        #[inline(always)]
        fn is_backtrack(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                ErrMode::Backtrack(_) => true,
                _ => false,
            }
        }
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            match self {
                ErrMode::Backtrack(e) | ErrMode::Cut(e) => Ok(e),
                err @ ErrMode::Incomplete(_) => Err(err),
            }
        }
        #[inline(always)]
        fn is_incomplete(&self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                ErrMode::Incomplete(_) => true,
                _ => false,
            }
        }
        #[inline(always)]
        fn needed(&self) -> Option<Needed> {
            match self {
                ErrMode::Incomplete(needed) => Some(*needed),
                _ => None,
            }
        }
    }
    impl<E> ModalError for ErrMode<E> {
        fn cut(self) -> Self {
            self.cut()
        }
        fn backtrack(self) -> Self {
            self.backtrack()
        }
    }
    impl<E1, E2> ErrorConvert<ErrMode<E2>> for ErrMode<E1>
    where
        E1: ErrorConvert<E2>,
    {
        #[inline(always)]
        fn convert(self) -> ErrMode<E2> {
            self.map(|e| e.convert())
        }
    }
    impl<I, EXT, E> FromExternalError<I, EXT> for ErrMode<E>
    where
        E: FromExternalError<I, EXT>,
    {
        #[inline(always)]
        fn from_external_error(input: &I, e: EXT) -> Self {
            ErrMode::Backtrack(E::from_external_error(input, e))
        }
    }
    impl<I: Stream, C, E: AddContext<I, C>> AddContext<I, C> for ErrMode<E> {
        #[inline(always)]
        fn add_context(
            self,
            input: &I,
            token_start: &<I as Stream>::Checkpoint,
            context: C,
        ) -> Self {
            self.map(|err| err.add_context(input, token_start, context))
        }
    }
    impl<T: Clone> ErrMode<InputError<T>> {
        /// Maps `ErrMode<InputError<T>>` to `ErrMode<InputError<U>>` with the given `F: T -> U`
        pub fn map_input<U: Clone, F>(self, f: F) -> ErrMode<InputError<U>>
        where
            F: FnOnce(T) -> U,
        {
            match self {
                ErrMode::Incomplete(n) => ErrMode::Incomplete(n),
                ErrMode::Cut(InputError { input }) => {
                    ErrMode::Cut(InputError { input: f(input) })
                }
                ErrMode::Backtrack(InputError { input }) => {
                    ErrMode::Backtrack(InputError { input: f(input) })
                }
            }
        }
    }
    impl<E: Eq> Eq for ErrMode<E> {}
    impl<E> fmt::Display for ErrMode<E>
    where
        E: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ErrMode::Incomplete(Needed::Size(u)) => {
                    f.write_fmt(format_args!("Parsing requires {0} more data", u))
                }
                ErrMode::Incomplete(Needed::Unknown) => {
                    f.write_fmt(format_args!("Parsing requires more data"))
                }
                ErrMode::Cut(c) => f.write_fmt(format_args!("Parsing Failure: {0:?}", c)),
                ErrMode::Backtrack(c) => {
                    f.write_fmt(format_args!("Parsing Error: {0:?}", c))
                }
            }
        }
    }
    /// The basic [`Parser`] trait for errors
    ///
    /// It provides methods to create an error from some combinators,
    /// and combine existing errors in combinators like `alt`.
    pub trait ParserError<I: Stream>: Sized {
        /// Generally, `Self`
        ///
        /// Mostly used for [`ErrMode`]
        type Inner;
        /// Creates an error from the input position
        fn from_input(input: &I) -> Self;
        /// Process a parser assertion
        #[inline(always)]
        fn assert(input: &I, _message: &'static str) -> Self
        where
            I: core::fmt::Debug,
        {
            {
                ::core::panicking::panic_fmt(
                    format_args!("assert `{0}` failed at {1:#?}", _message, input),
                );
            };
        }
        /// There was not enough data to determine the appropriate action
        ///
        /// More data needs to be buffered before retrying the parse.
        ///
        /// This must only be set when the [`Stream`] is [partial][`crate::stream::StreamIsPartial`], like with
        /// [`Partial`][crate::Partial]
        ///
        /// Convert this into an `Backtrack` with [`Parser::complete_err`]
        #[inline(always)]
        fn incomplete(input: &I, _needed: Needed) -> Self {
            Self::from_input(input)
        }
        /// Like [`ParserError::from_input`] but merges it with the existing error.
        ///
        /// This is useful when backtracking through a parse tree, accumulating error context on the
        /// way.
        #[inline]
        fn append(self, _input: &I, _token_start: &<I as Stream>::Checkpoint) -> Self {
            self
        }
        /// Combines errors from two different parse branches.
        ///
        /// For example, this would be used by [`alt`][crate::combinator::alt] to report the error from
        /// each case.
        #[inline]
        fn or(self, other: Self) -> Self {
            other
        }
        /// Is backtracking and trying new parse branches allowed?
        #[inline(always)]
        fn is_backtrack(&self) -> bool {
            true
        }
        /// Unwrap the mode, returning the underlying error, if present
        fn into_inner(self) -> Result<Self::Inner, Self>;
        /// Is more data [`Needed`]
        ///
        /// This must be the same as [`err.needed().is_some()`][ParserError::needed]
        #[inline(always)]
        fn is_incomplete(&self) -> bool {
            false
        }
        /// Extract the [`Needed`] data, if present
        ///
        /// `Self::needed().is_some()` must be the same as
        /// [`err.is_incomplete()`][ParserError::is_incomplete]
        #[inline(always)]
        fn needed(&self) -> Option<Needed> {
            None
        }
    }
    /// Manipulate the how parsers respond to this error
    pub trait ModalError {
        /// Prevent backtracking, bubbling the error up to the top
        fn cut(self) -> Self;
        /// Enable backtracking support
        fn backtrack(self) -> Self;
    }
    /// Used by [`Parser::context`] to add custom data to error while backtracking
    ///
    /// May be implemented multiple times for different kinds of context.
    pub trait AddContext<I: Stream, C = &'static str>: Sized {
        /// Append to an existing error custom data
        ///
        /// This is used mainly by [`Parser::context`], to add user friendly information
        /// to errors when backtracking through a parse tree
        #[inline]
        fn add_context(
            self,
            _input: &I,
            _token_start: &<I as Stream>::Checkpoint,
            _context: C,
        ) -> Self {
            self
        }
    }
    /// Create a new error with an external error, from [`std::str::FromStr`]
    ///
    /// This trait is required by the [`Parser::try_map`] combinator.
    pub trait FromExternalError<I, E> {
        /// Like [`ParserError::from_input`] but also include an external error.
        fn from_external_error(input: &I, e: E) -> Self;
    }
    /// Equivalent of `From` implementation to avoid orphan rules in bits parsers
    pub trait ErrorConvert<E> {
        /// Transform to another error type
        fn convert(self) -> E;
    }
    /// Capture input on error
    ///
    /// This is useful for testing of generic parsers to ensure the error happens at the right
    /// location.
    ///
    /// <div class="warning">
    ///
    /// **Note:** [context][Parser::context] and inner errors (like from [`Parser::try_map`]) will be
    /// dropped.
    ///
    /// </div>
    pub struct InputError<I: Clone> {
        /// The input stream, pointing to the location where the error occurred
        pub input: I,
    }
    #[automatically_derived]
    impl<I: ::core::marker::Copy + Clone> ::core::marker::Copy for InputError<I> {}
    #[automatically_derived]
    impl<I: ::core::clone::Clone + Clone> ::core::clone::Clone for InputError<I> {
        #[inline]
        fn clone(&self) -> InputError<I> {
            InputError {
                input: ::core::clone::Clone::clone(&self.input),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug + Clone> ::core::fmt::Debug for InputError<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "InputError",
                "input",
                &&self.input,
            )
        }
    }
    #[automatically_derived]
    impl<I: ::core::cmp::Eq + Clone> ::core::cmp::Eq for InputError<I> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<I>;
        }
    }
    #[automatically_derived]
    impl<I: Clone> ::core::marker::StructuralPartialEq for InputError<I> {}
    #[automatically_derived]
    impl<I: ::core::cmp::PartialEq + Clone> ::core::cmp::PartialEq for InputError<I> {
        #[inline]
        fn eq(&self, other: &InputError<I>) -> bool {
            self.input == other.input
        }
    }
    impl<I: Clone> InputError<I> {
        /// Creates a new basic error
        #[inline]
        pub fn at(input: I) -> Self {
            Self { input }
        }
        /// Translate the input type
        #[inline]
        pub fn map_input<I2: Clone, O: Fn(I) -> I2>(self, op: O) -> InputError<I2> {
            InputError {
                input: op(self.input),
            }
        }
    }
    impl<I: ToOwned> InputError<&I>
    where
        <I as ToOwned>::Owned: Clone,
    {
        /// Obtaining ownership
        pub fn into_owned(self) -> InputError<<I as ToOwned>::Owned> {
            self.map_input(ToOwned::to_owned)
        }
    }
    impl<I: Stream + Clone> ParserError<I> for InputError<I> {
        type Inner = Self;
        #[inline]
        fn from_input(input: &I) -> Self {
            Self { input: input.clone() }
        }
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            Ok(self)
        }
    }
    impl<I: Stream + Clone, C> AddContext<I, C> for InputError<I> {}
    impl<I: Clone, E> FromExternalError<I, E> for InputError<I> {
        /// Create a new error from an input position and an external error
        #[inline]
        fn from_external_error(input: &I, _e: E) -> Self {
            Self { input: input.clone() }
        }
    }
    /// The Display implementation allows the `std::error::Error` implementation
    impl<I: Clone + fmt::Display> fmt::Display for InputError<I> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_fmt(format_args!("failed to parse starting at: {0}", self.input))
        }
    }
    impl<I: Clone + fmt::Debug + fmt::Display + Sync + Send + 'static> std::error::Error
    for InputError<I> {}
    /// Track an error occurred without any other [`StrContext`]
    pub struct EmptyError;
    #[automatically_derived]
    impl ::core::marker::Copy for EmptyError {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for EmptyError {}
    #[automatically_derived]
    impl ::core::clone::Clone for EmptyError {
        #[inline]
        fn clone(&self) -> EmptyError {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for EmptyError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "EmptyError")
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for EmptyError {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for EmptyError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for EmptyError {
        #[inline]
        fn eq(&self, other: &EmptyError) -> bool {
            true
        }
    }
    impl<I: Stream> ParserError<I> for EmptyError {
        type Inner = Self;
        #[inline(always)]
        fn from_input(_: &I) -> Self {
            Self
        }
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            Ok(self)
        }
    }
    impl<I: Stream, C> AddContext<I, C> for EmptyError {}
    impl<I, E> FromExternalError<I, E> for EmptyError {
        #[inline(always)]
        fn from_external_error(_input: &I, _e: E) -> Self {
            Self
        }
    }
    impl ErrorConvert<EmptyError> for EmptyError {
        #[inline(always)]
        fn convert(self) -> EmptyError {
            self
        }
    }
    impl core::fmt::Display for EmptyError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            "failed to parse".fmt(f)
        }
    }
    impl<I: Stream> ParserError<I> for () {
        type Inner = Self;
        #[inline]
        fn from_input(_: &I) -> Self {}
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            Ok(self)
        }
    }
    impl<I: Stream, C> AddContext<I, C> for () {}
    impl<I, E> FromExternalError<I, E> for () {
        #[inline]
        fn from_external_error(_input: &I, _e: E) -> Self {}
    }
    impl ErrorConvert<()> for () {
        #[inline]
        fn convert(self) {}
    }
    /// Accumulate context while backtracking errors
    ///
    /// See the [tutorial][crate::_tutorial::chapter_7#error-adaptation-and-rendering]
    /// for an example of how to adapt this to an application error with custom rendering.
    pub struct ContextError<C = StrContext> {
        context: alloc::vec::Vec<C>,
        cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    }
    #[automatically_derived]
    impl<C: ::core::fmt::Debug> ::core::fmt::Debug for ContextError<C> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "ContextError",
                "context",
                &self.context,
                "cause",
                &&self.cause,
            )
        }
    }
    impl<C> ContextError<C> {
        /// Create an empty error
        #[inline]
        pub fn new() -> Self {
            Self {
                context: Default::default(),
                cause: None,
            }
        }
        /// Add more context
        #[inline]
        pub fn push(&mut self, context: C) {
            self.context.push(context);
        }
        /// Add more context
        #[inline]
        pub fn extend<I: IntoIterator<Item = C>>(&mut self, context: I) {
            self.context.extend(context);
        }
        /// Access context from [`Parser::context`]
        #[inline]
        pub fn context(&self) -> impl Iterator<Item = &C> {
            self.context.iter()
        }
        /// Originating [`std::error::Error`]
        #[inline]
        pub fn cause(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
            self.cause.as_deref()
        }
    }
    impl<C: Clone> Clone for ContextError<C> {
        fn clone(&self) -> Self {
            Self {
                context: self.context.clone(),
                cause: self.cause.as_ref().map(|e| e.to_string().into()),
            }
        }
    }
    impl<C> Default for ContextError<C> {
        #[inline]
        fn default() -> Self {
            Self::new()
        }
    }
    impl<I: Stream, C> ParserError<I> for ContextError<C> {
        type Inner = Self;
        #[inline]
        fn from_input(_input: &I) -> Self {
            Self::new()
        }
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            Ok(self)
        }
    }
    impl<C, I: Stream> AddContext<I, C> for ContextError<C> {
        #[inline]
        fn add_context(
            mut self,
            _input: &I,
            _token_start: &<I as Stream>::Checkpoint,
            context: C,
        ) -> Self {
            self.push(context);
            self
        }
    }
    impl<C, I, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E>
    for ContextError<C> {
        #[inline]
        fn from_external_error(_input: &I, e: E) -> Self {
            let mut err = Self::new();
            {
                err.cause = Some(Box::new(e));
            }
            err
        }
    }
    impl<C: core::cmp::PartialEq> core::cmp::PartialEq for ContextError<C> {
        fn eq(&self, other: &Self) -> bool {
            {
                if self.context != other.context {
                    return false;
                }
            }
            {
                if self.cause.as_ref().map(ToString::to_string)
                    != other.cause.as_ref().map(ToString::to_string)
                {
                    return false;
                }
            }
            true
        }
    }
    impl core::fmt::Display for ContextError<StrContext> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            {
                let expression = self
                    .context()
                    .find_map(|c| match c {
                        StrContext::Label(c) => Some(c),
                        _ => None,
                    });
                let expected = self
                    .context()
                    .filter_map(|c| match c {
                        StrContext::Expected(c) => Some(c),
                        _ => None,
                    })
                    .collect::<alloc::vec::Vec<_>>();
                let mut newline = false;
                if let Some(expression) = expression {
                    newline = true;
                    f.write_fmt(format_args!("invalid {0}", expression))?;
                }
                if !expected.is_empty() {
                    if newline {
                        f.write_fmt(format_args!("\n"))?;
                    }
                    newline = true;
                    f.write_fmt(format_args!("expected "))?;
                    for (i, expected) in expected.iter().enumerate() {
                        if i != 0 {
                            f.write_fmt(format_args!(", "))?;
                        }
                        f.write_fmt(format_args!("{0}", expected))?;
                    }
                }
                {
                    if let Some(cause) = self.cause() {
                        if newline {
                            f.write_fmt(format_args!("\n"))?;
                        }
                        f.write_fmt(format_args!("{0}", cause))?;
                    }
                }
            }
            Ok(())
        }
    }
    impl<C> ErrorConvert<ContextError<C>> for ContextError<C> {
        #[inline]
        fn convert(self) -> ContextError<C> {
            self
        }
    }
    /// Additional parse context for [`ContextError`] added via [`Parser::context`]
    #[non_exhaustive]
    pub enum StrContext {
        /// Description of what is currently being parsed
        Label(&'static str),
        /// Grammar item that was expected
        Expected(StrContextValue),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for StrContext {
        #[inline]
        fn clone(&self) -> StrContext {
            match self {
                StrContext::Label(__self_0) => {
                    StrContext::Label(::core::clone::Clone::clone(__self_0))
                }
                StrContext::Expected(__self_0) => {
                    StrContext::Expected(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for StrContext {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                StrContext::Label(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Label",
                        &__self_0,
                    )
                }
                StrContext::Expected(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Expected",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for StrContext {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for StrContext {
        #[inline]
        fn eq(&self, other: &StrContext) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (StrContext::Label(__self_0), StrContext::Label(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (StrContext::Expected(__self_0), StrContext::Expected(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for StrContext {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'static str>;
            let _: ::core::cmp::AssertParamIsEq<StrContextValue>;
        }
    }
    impl core::fmt::Display for StrContext {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Label(name) => f.write_fmt(format_args!("invalid {0}", name)),
                Self::Expected(value) => f.write_fmt(format_args!("expected {0}", value)),
            }
        }
    }
    /// See [`StrContext`]
    #[non_exhaustive]
    pub enum StrContextValue {
        /// A [`char`] token
        CharLiteral(char),
        /// A [`&str`] token
        StringLiteral(&'static str),
        /// A description of what was being parsed
        Description(&'static str),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for StrContextValue {
        #[inline]
        fn clone(&self) -> StrContextValue {
            match self {
                StrContextValue::CharLiteral(__self_0) => {
                    StrContextValue::CharLiteral(::core::clone::Clone::clone(__self_0))
                }
                StrContextValue::StringLiteral(__self_0) => {
                    StrContextValue::StringLiteral(::core::clone::Clone::clone(__self_0))
                }
                StrContextValue::Description(__self_0) => {
                    StrContextValue::Description(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for StrContextValue {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                StrContextValue::CharLiteral(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "CharLiteral",
                        &__self_0,
                    )
                }
                StrContextValue::StringLiteral(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "StringLiteral",
                        &__self_0,
                    )
                }
                StrContextValue::Description(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Description",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for StrContextValue {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for StrContextValue {
        #[inline]
        fn eq(&self, other: &StrContextValue) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        StrContextValue::CharLiteral(__self_0),
                        StrContextValue::CharLiteral(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    (
                        StrContextValue::StringLiteral(__self_0),
                        StrContextValue::StringLiteral(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    (
                        StrContextValue::Description(__self_0),
                        StrContextValue::Description(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for StrContextValue {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<char>;
            let _: ::core::cmp::AssertParamIsEq<&'static str>;
            let _: ::core::cmp::AssertParamIsEq<&'static str>;
        }
    }
    impl From<char> for StrContextValue {
        #[inline]
        fn from(inner: char) -> Self {
            Self::CharLiteral(inner)
        }
    }
    impl From<&'static str> for StrContextValue {
        #[inline]
        fn from(inner: &'static str) -> Self {
            Self::StringLiteral(inner)
        }
    }
    impl core::fmt::Display for StrContextValue {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::CharLiteral('\n') => "newline".fmt(f),
                Self::CharLiteral('`') => "'`'".fmt(f),
                Self::CharLiteral(c) if c.is_ascii_control() => {
                    f.write_fmt(format_args!("`{0}`", c.escape_debug()))
                }
                Self::CharLiteral(c) => f.write_fmt(format_args!("`{0}`", c)),
                Self::StringLiteral(c) => f.write_fmt(format_args!("`{0}`", c)),
                Self::Description(c) => f.write_fmt(format_args!("{0}", c)),
            }
        }
    }
    /// Trace all error paths, particularly for tests
    pub enum TreeError<I, C = StrContext> {
        /// Initial error that kicked things off
        Base(TreeErrorBase<I>),
        /// Traces added to the error while walking back up the stack
        Stack {
            /// Initial error that kicked things off
            base: Box<Self>,
            /// Traces added to the error while walking back up the stack
            stack: Vec<TreeErrorFrame<I, C>>,
        },
        /// All failed branches of an `alt`
        Alt(Vec<Self>),
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, C: ::core::fmt::Debug> ::core::fmt::Debug
    for TreeError<I, C> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                TreeError::Base(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Base",
                        &__self_0,
                    )
                }
                TreeError::Stack { base: __self_0, stack: __self_1 } => {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Stack",
                        "base",
                        __self_0,
                        "stack",
                        &__self_1,
                    )
                }
                TreeError::Alt(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Alt",
                        &__self_0,
                    )
                }
            }
        }
    }
    /// See [`TreeError::Stack`]
    pub enum TreeErrorFrame<I, C = StrContext> {
        /// See [`ParserError::append`]
        Kind(TreeErrorBase<I>),
        /// See [`AddContext::add_context`]
        Context(TreeErrorContext<I, C>),
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, C: ::core::fmt::Debug> ::core::fmt::Debug
    for TreeErrorFrame<I, C> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                TreeErrorFrame::Kind(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Kind",
                        &__self_0,
                    )
                }
                TreeErrorFrame::Context(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Context",
                        &__self_0,
                    )
                }
            }
        }
    }
    /// See [`TreeErrorFrame::Kind`], [`ParserError::append`]
    pub struct TreeErrorBase<I> {
        /// Parsed input, at the location where the error occurred
        pub input: I,
        /// See [`FromExternalError::from_external_error`]
        pub cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug> ::core::fmt::Debug for TreeErrorBase<I> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TreeErrorBase",
                "input",
                &self.input,
                "cause",
                &&self.cause,
            )
        }
    }
    /// See [`TreeErrorFrame::Context`], [`AddContext::add_context`]
    pub struct TreeErrorContext<I, C = StrContext> {
        /// Parsed input, at the location where the error occurred
        pub input: I,
        /// See [`AddContext::add_context`]
        pub context: C,
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, C: ::core::fmt::Debug> ::core::fmt::Debug
    for TreeErrorContext<I, C> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "TreeErrorContext",
                "input",
                &self.input,
                "context",
                &&self.context,
            )
        }
    }
    impl<I: ToOwned, C> TreeError<&I, C> {
        /// Obtaining ownership
        pub fn into_owned(self) -> TreeError<<I as ToOwned>::Owned, C> {
            self.map_input(ToOwned::to_owned)
        }
    }
    impl<I, C> TreeError<I, C> {
        /// Translate the input type
        pub fn map_input<I2, O: Clone + Fn(I) -> I2>(self, op: O) -> TreeError<I2, C> {
            match self {
                TreeError::Base(base) => {
                    TreeError::Base(TreeErrorBase {
                        input: op(base.input),
                        cause: base.cause,
                    })
                }
                TreeError::Stack { base, stack } => {
                    let base = Box::new(base.map_input(op.clone()));
                    let stack = stack
                        .into_iter()
                        .map(|frame| match frame {
                            TreeErrorFrame::Kind(kind) => {
                                TreeErrorFrame::Kind(TreeErrorBase {
                                    input: op(kind.input),
                                    cause: kind.cause,
                                })
                            }
                            TreeErrorFrame::Context(context) => {
                                TreeErrorFrame::Context(TreeErrorContext {
                                    input: op(context.input),
                                    context: context.context,
                                })
                            }
                        })
                        .collect();
                    TreeError::Stack { base, stack }
                }
                TreeError::Alt(alt) => {
                    TreeError::Alt(
                        alt.into_iter().map(|e| e.map_input(op.clone())).collect(),
                    )
                }
            }
        }
        fn append_frame(self, frame: TreeErrorFrame<I, C>) -> Self {
            match self {
                TreeError::Stack { base, mut stack } => {
                    stack.push(frame);
                    TreeError::Stack { base, stack }
                }
                base => {
                    TreeError::Stack {
                        base: Box::new(base),
                        stack: <[_]>::into_vec(::alloc::boxed::box_new([frame])),
                    }
                }
            }
        }
    }
    impl<I, C> ParserError<I> for TreeError<I, C>
    where
        I: Stream + Clone,
    {
        type Inner = Self;
        fn from_input(input: &I) -> Self {
            TreeError::Base(TreeErrorBase {
                input: input.clone(),
                cause: None,
            })
        }
        fn append(self, input: &I, token_start: &<I as Stream>::Checkpoint) -> Self {
            let mut input = input.clone();
            input.reset(token_start);
            let frame = TreeErrorFrame::Kind(TreeErrorBase {
                input,
                cause: None,
            });
            self.append_frame(frame)
        }
        fn or(self, other: Self) -> Self {
            match (self, other) {
                (TreeError::Alt(mut first), TreeError::Alt(second)) => {
                    first.extend(second);
                    TreeError::Alt(first)
                }
                (TreeError::Alt(mut alt), new) | (new, TreeError::Alt(mut alt)) => {
                    alt.push(new);
                    TreeError::Alt(alt)
                }
                (first, second) => {
                    TreeError::Alt(
                        <[_]>::into_vec(::alloc::boxed::box_new([first, second])),
                    )
                }
            }
        }
        #[inline(always)]
        fn into_inner(self) -> Result<Self::Inner, Self> {
            Ok(self)
        }
    }
    impl<I, C> AddContext<I, C> for TreeError<I, C>
    where
        I: Stream + Clone,
    {
        fn add_context(
            self,
            input: &I,
            token_start: &<I as Stream>::Checkpoint,
            context: C,
        ) -> Self {
            let mut input = input.clone();
            input.reset(token_start);
            let frame = TreeErrorFrame::Context(TreeErrorContext { input, context });
            self.append_frame(frame)
        }
    }
    impl<I, C, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E>
    for TreeError<I, C>
    where
        I: Clone,
    {
        fn from_external_error(input: &I, e: E) -> Self {
            TreeError::Base(TreeErrorBase {
                input: input.clone(),
                cause: Some(Box::new(e)),
            })
        }
    }
    impl<I, C> ErrorConvert<TreeError<(I, usize), C>> for TreeError<I, C> {
        #[inline]
        fn convert(self) -> TreeError<(I, usize), C> {
            self.map_input(|i| (i, 0))
        }
    }
    impl<I, C> ErrorConvert<TreeError<I, C>> for TreeError<(I, usize), C> {
        #[inline]
        fn convert(self) -> TreeError<I, C> {
            self.map_input(|(i, _o)| i)
        }
    }
    impl<I, C> TreeError<I, C>
    where
        I: core::fmt::Display,
        C: fmt::Display,
    {
        fn write(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
            let child_indent = indent + 2;
            match self {
                TreeError::Base(base) => {
                    f.write_fmt(format_args!("{0:1$}{2}\n", "", indent, base))?;
                }
                TreeError::Stack { base, stack } => {
                    base.write(f, indent)?;
                    for (level, frame) in stack.iter().enumerate() {
                        match frame {
                            TreeErrorFrame::Kind(frame) => {
                                f.write_fmt(
                                    format_args!(
                                        "{0:1$}{2}: {3}\n",
                                        "",
                                        child_indent,
                                        level,
                                        frame,
                                    ),
                                )?;
                            }
                            TreeErrorFrame::Context(frame) => {
                                f.write_fmt(
                                    format_args!(
                                        "{0:1$}{2}: {3}\n",
                                        "",
                                        child_indent,
                                        level,
                                        frame,
                                    ),
                                )?;
                            }
                        }
                    }
                }
                TreeError::Alt(alt) => {
                    f.write_fmt(format_args!("{0:1$}during one of:\n", "", indent))?;
                    for child in alt {
                        child.write(f, child_indent)?;
                    }
                }
            }
            Ok(())
        }
    }
    impl<I: fmt::Display> fmt::Display for TreeErrorBase<I> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(cause) = self.cause.as_ref() {
                f.write_fmt(format_args!("caused by {0}", cause))?;
            }
            let input = abbreviate(self.input.to_string());
            f.write_fmt(format_args!(" at \'{0}\'", input))?;
            Ok(())
        }
    }
    impl<I: fmt::Display, C: fmt::Display> fmt::Display for TreeErrorContext<I, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let context = &self.context;
            let input = abbreviate(self.input.to_string());
            f.write_fmt(format_args!("{0} at \'{1}\'", context, input))?;
            Ok(())
        }
    }
    impl<
        I: fmt::Debug + fmt::Display + Sync + Send + 'static,
        C: fmt::Display + fmt::Debug,
    > std::error::Error for TreeError<I, C> {}
    fn abbreviate(input: String) -> String {
        let mut abbrev = None;
        if let Some((line, _)) = input.split_once('\n') {
            abbrev = Some(line);
        }
        let max_len = 20;
        let current = abbrev.unwrap_or(&input);
        if max_len < current.len() {
            if let Some((index, _)) = current.char_indices().nth(max_len) {
                abbrev = Some(&current[..index]);
            }
        }
        if let Some(abbrev) = abbrev {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(format_args!("{0}...", abbrev))
            })
        } else {
            input
        }
    }
    impl<I: fmt::Display, C: fmt::Display> fmt::Display for TreeError<I, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.write(f, 0)
        }
    }
    /// See [`Parser::parse`]
    pub struct ParseError<I, E> {
        input: I,
        offset: usize,
        inner: E,
    }
    #[automatically_derived]
    impl<I: ::core::clone::Clone, E: ::core::clone::Clone> ::core::clone::Clone
    for ParseError<I, E> {
        #[inline]
        fn clone(&self) -> ParseError<I, E> {
            ParseError {
                input: ::core::clone::Clone::clone(&self.input),
                offset: ::core::clone::Clone::clone(&self.offset),
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    #[automatically_derived]
    impl<I: ::core::fmt::Debug, E: ::core::fmt::Debug> ::core::fmt::Debug
    for ParseError<I, E> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "ParseError",
                "input",
                &self.input,
                "offset",
                &self.offset,
                "inner",
                &&self.inner,
            )
        }
    }
    #[automatically_derived]
    impl<I, E> ::core::marker::StructuralPartialEq for ParseError<I, E> {}
    #[automatically_derived]
    impl<I: ::core::cmp::PartialEq, E: ::core::cmp::PartialEq> ::core::cmp::PartialEq
    for ParseError<I, E> {
        #[inline]
        fn eq(&self, other: &ParseError<I, E>) -> bool {
            self.input == other.input && self.offset == other.offset
                && self.inner == other.inner
        }
    }
    #[automatically_derived]
    impl<I: ::core::cmp::Eq, E: ::core::cmp::Eq> ::core::cmp::Eq for ParseError<I, E> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<I>;
            let _: ::core::cmp::AssertParamIsEq<usize>;
            let _: ::core::cmp::AssertParamIsEq<E>;
        }
    }
    impl<I: Stream, E: ParserError<I>> ParseError<I, E> {
        pub(crate) fn new(mut input: I, start: I::Checkpoint, inner: E) -> Self {
            let offset = input.offset_from(&start);
            input.reset(&start);
            Self { input, offset, inner }
        }
    }
    impl<I, E> ParseError<I, E> {
        /// The [`Stream`] at the initial location when parsing started
        #[inline]
        pub fn input(&self) -> &I {
            &self.input
        }
        /// The location in [`ParseError::input`] where parsing failed
        ///
        /// To get the span for the `char` this points to, see [`ParseError::char_span`].
        ///
        /// <div class="warning">
        ///
        /// **Note:** This is an offset, not an index, and may point to the end of input
        /// (`input.len()`) on eof errors.
        ///
        /// </div>
        #[inline]
        pub fn offset(&self) -> usize {
            self.offset
        }
        /// The original [`ParserError`]
        #[inline]
        pub fn inner(&self) -> &E {
            &self.inner
        }
        /// The original [`ParserError`]
        #[inline]
        pub fn into_inner(self) -> E {
            self.inner
        }
    }
    impl<I: AsBStr, E> ParseError<I, E> {
        /// The byte indices for the `char` at [`ParseError::offset`]
        #[inline]
        pub fn char_span(&self) -> core::ops::Range<usize> {
            char_boundary(self.input.as_bstr(), self.offset())
        }
    }
    fn char_boundary(input: &[u8], offset: usize) -> core::ops::Range<usize> {
        let len = input.len();
        if offset == len {
            return offset..offset;
        }
        let start = (0..(offset + 1).min(len))
            .rev()
            .find(|i| {
                input.get(*i).copied().map(is_utf8_char_boundary).unwrap_or(false)
            })
            .unwrap_or(0);
        let end = (offset + 1..len)
            .find(|i| {
                input.get(*i).copied().map(is_utf8_char_boundary).unwrap_or(false)
            })
            .unwrap_or(len);
        start..end
    }
    /// Taken from `core::num`
    const fn is_utf8_char_boundary(b: u8) -> bool {
        (b as i8) >= -0x40
    }
    impl<I, E> core::fmt::Display for ParseError<I, E>
    where
        I: AsBStr,
        E: core::fmt::Display,
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let input = self.input.as_bstr();
            let span_start = self.offset;
            let span_end = span_start;
            if input.contains(&b'\n') {
                let (line_idx, col_idx) = translate_position(input, span_start);
                let line_num = line_idx + 1;
                let col_num = col_idx + 1;
                let gutter = line_num.to_string().len();
                let content = input
                    .split(|c| *c == b'\n')
                    .nth(line_idx)
                    .expect("valid line number");
                f.write_fmt(
                    format_args!(
                        "parse error at line {0}, column {1}\n",
                        line_num,
                        col_num,
                    ),
                )?;
                for _ in 0..gutter {
                    f.write_fmt(format_args!(" "))?;
                }
                f.write_fmt(format_args!(" |\n"))?;
                f.write_fmt(format_args!("{0} | ", line_num))?;
                f.write_fmt(format_args!("{0}\n", String::from_utf8_lossy(content)))?;
                for _ in 0..gutter {
                    f.write_fmt(format_args!(" "))?;
                }
                f.write_fmt(format_args!(" | "))?;
                for _ in 0..col_idx {
                    f.write_fmt(format_args!(" "))?;
                }
                f.write_fmt(format_args!("^"))?;
                for _ in (span_start + 1)..(span_end.min(span_start + content.len())) {
                    f.write_fmt(format_args!("^"))?;
                }
                f.write_fmt(format_args!("\n"))?;
            } else {
                let content = input;
                f.write_fmt(format_args!("{0}\n", String::from_utf8_lossy(content)))?;
                for _ in 0..span_start {
                    f.write_fmt(format_args!(" "))?;
                }
                f.write_fmt(format_args!("^"))?;
                for _ in (span_start + 1)..(span_end.min(span_start + content.len())) {
                    f.write_fmt(format_args!("^"))?;
                }
                f.write_fmt(format_args!("\n"))?;
            }
            f.write_fmt(format_args!("{0}", self.inner))?;
            Ok(())
        }
    }
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
}
mod parser {
    //! Basic types to build the parsers
    use crate::combinator::impls;
    use crate::error::{AddContext, FromExternalError, ParseError, ParserError, Result};
    use crate::stream::{Compare, Location, ParseSlice, Stream, StreamIsPartial};
    /// Core trait for parsing
    ///
    /// The simplest way to implement a `Parser` is with a function
    /// ```rust
    /// use winnow::prelude::*;
    ///
    /// fn empty(input: &mut &str) -> ModalResult<()> {
    ///     let output = ();
    ///     Ok(output)
    /// }
    ///
    /// let (input, output) = empty.parse_peek("Hello").unwrap();
    /// assert_eq!(input, "Hello");  // We didn't consume any input
    /// ```
    ///
    /// which can be made stateful by returning a function
    /// ```rust
    /// use winnow::prelude::*;
    ///
    /// fn empty<O: Clone>(output: O) -> impl FnMut(&mut &str) -> ModalResult<O> {
    ///     move |input: &mut &str| {
    ///         let output = output.clone();
    ///         Ok(output)
    ///     }
    /// }
    ///
    /// let (input, output) = empty("World").parse_peek("Hello").unwrap();
    /// assert_eq!(input, "Hello");  // We didn't consume any input
    /// assert_eq!(output, "World");
    /// ```
    ///
    /// Additionally, some basic types implement `Parser` as well, including
    /// - `u8` and `char`, see [`winnow::token::one_of`][crate::token::one_of]
    /// - `&[u8]` and `&str`, see [`winnow::token::literal`][crate::token::literal]
    pub trait Parser<I, O, E> {
        /// Parse all of `input`, generating `O` from it
        ///
        /// This is intended for integrating your parser into the rest of your application.
        ///
        /// For one [`Parser`] to drive another [`Parser`] forward or for
        /// [incremental parsing][StreamIsPartial], see instead [`Parser::parse_next`].
        ///
        /// This assumes the [`Parser`] intends to read all of `input` and will return an
        /// [`eof`][crate::combinator::eof] error if it does not.
        /// To ignore trailing `input`, combine your parser with a [`rest`][crate::token::rest]
        /// (e.g. `(parser, rest).parse(input)`).
        ///
        /// See also the [tutorial][crate::_tutorial::chapter_6].
        #[inline]
        fn parse(
            &mut self,
            mut input: I,
        ) -> Result<O, ParseError<I, <E as ParserError<I>>::Inner>>
        where
            Self: core::marker::Sized,
            I: Stream,
            I: StreamIsPartial,
            E: ParserError<I>,
            <E as ParserError<I>>::Inner: ParserError<I>,
        {
            if true {
                if !!I::is_partial_supported() {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "partial streams need to handle `ErrMode::Incomplete`",
                            ),
                        );
                    }
                }
            }
            let start = input.checkpoint();
            let (o, _) = (self.by_ref(), crate::combinator::eof)
                .parse_next(&mut input)
                .map_err(|e| {
                    let e = e
                        .into_inner()
                        .unwrap_or_else(|_err| {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "complete parsers should not report `ErrMode::Incomplete(_)`",
                                    ),
                                );
                            }
                        });
                    ParseError::new(input, start, e)
                })?;
            Ok(o)
        }
        /// Repeat this parse until all of `input` is consumed, generating `O` from it
        ///
        /// This is intended for integrating your parser into the rest of your application.
        /// To instead iterate inside of a parser, see [iterator][crate::combinator::iterator].
        ///
        /// This assumes the [`Parser`] intends to read all of `input` and will return an
        /// [`eof`][crate::combinator::eof] error if it does not.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::ascii::dec_uint;
        /// # use winnow::ascii::newline;
        /// # use winnow::combinator::terminated;
        /// # use winnow::combinator::opt;
        /// # use winnow::prelude::*;
        /// fn number(input: &mut &str) -> Result<u32, ()> {
        ///   terminated(dec_uint, opt(newline)).parse_next(input)
        /// }
        ///
        /// let input = "10\n20\n30";
        /// let numbers = number.parse_iter(input)
        ///     .map(|r| r.unwrap()).collect::<Vec<_>>();
        /// assert_eq!(numbers, vec![10, 20, 30]);
        /// # }
        /// ```
        #[inline]
        fn parse_iter(&mut self, input: I) -> impls::ParseIter<'_, Self, I, O, E>
        where
            Self: core::marker::Sized,
            I: Stream,
            I: StreamIsPartial,
            E: ParserError<I>,
            <E as ParserError<I>>::Inner: ParserError<I>,
        {
            if true {
                if !!I::is_partial_supported() {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "partial streams need to handle `ErrMode::Incomplete`",
                            ),
                        );
                    }
                }
            }
            let start = input.checkpoint();
            impls::ParseIter {
                parser: self,
                input: Some(input),
                start: Some(start),
                marker: Default::default(),
            }
        }
        /// Take tokens from the [`Stream`], turning it into the output
        ///
        /// This includes advancing the input [`Stream`] to the next location.
        ///
        /// On error, `input` will be left pointing at the error location.
        ///
        /// This is intended for a [`Parser`] to drive another [`Parser`] forward or for
        /// [incremental parsing][StreamIsPartial]
        fn parse_next(&mut self, input: &mut I) -> Result<O, E>;
        /// Take tokens from the [`Stream`], turning it into the output
        ///
        /// This returns a copy of the [`Stream`] advanced to the next location.
        ///
        /// <div class="warning">
        ///
        /// Generally, prefer [`Parser::parse_next`].
        /// This is primarily intended for:
        /// - Migrating from older versions / `nom`
        /// - Testing [`Parser`]s
        ///
        /// For look-ahead parsing, see instead [`peek`][crate::combinator::peek].
        ///
        /// </div>
        #[inline(always)]
        fn parse_peek(&mut self, mut input: I) -> Result<(I, O), E> {
            match self.parse_next(&mut input) {
                Ok(o) => Ok((input, o)),
                Err(err) => Err(err),
            }
        }
        /// Treat `&mut Self` as a parser
        ///
        /// This helps when needing to move a `Parser` when all you have is a `&mut Parser`.
        ///
        /// # Example
        ///
        /// Because parsers are `FnMut`, they can be called multiple times. This prevents moving `f`
        /// into [`length_take`][crate::binary::length_take] and `g` into
        /// [`Parser::complete_err`]:
        /// ```rust,compile_fail
        /// # use winnow::prelude::*;
        /// # use winnow::Parser;
        /// # use winnow::error::ParserError;
        /// # use winnow::binary::length_take;
        /// pub fn length_value<'i, O, E: ParserError<&'i [u8]>>(
        ///     mut f: impl Parser<&'i [u8], usize, E>,
        ///     mut g: impl Parser<&'i [u8], O, E>
        /// ) -> impl Parser<&'i [u8], O, E> {
        ///   move |i: &mut &'i [u8]| {
        ///     let mut data = length_take(f).parse_next(i)?;
        ///     let o = g.complete_err().parse_next(&mut data)?;
        ///     Ok(o)
        ///   }
        /// }
        /// ```
        ///
        /// By adding `by_ref`, we can make this work:
        /// ```rust
        /// # #[cfg(feature = "binary")] {
        /// # use winnow::prelude::*;
        /// # use winnow::Parser;
        /// # use winnow::error::ParserError;
        /// # use winnow::binary::length_take;
        /// pub fn length_value<'i, O, E: ParserError<&'i [u8]>>(
        ///     mut f: impl Parser<&'i [u8], usize, E>,
        ///     mut g: impl Parser<&'i [u8], O, E>
        /// ) -> impl Parser<&'i [u8], O, E> {
        ///   move |i: &mut &'i [u8]| {
        ///     let mut data = length_take(f.by_ref()).parse_next(i)?;
        ///     let o = g.by_ref().complete_err().parse_next(&mut data)?;
        ///     Ok(o)
        ///   }
        /// }
        /// # }
        /// ```
        #[inline(always)]
        fn by_ref(&mut self) -> impls::ByRef<'_, Self, I, O, E>
        where
            Self: core::marker::Sized,
        {
            impls::ByRef {
                p: self,
                marker: Default::default(),
            }
        }
        /// Produce the provided value
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<i32> {
        ///     alpha1.value(1234).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd"), Ok(("", 1234)));
        /// assert!(parser.parse_peek("123abcd;").is_err());
        /// # }
        /// ```
        #[doc(alias = "to")]
        #[inline(always)]
        fn value<O2>(self, val: O2) -> impls::Value<Self, I, O, O2, E>
        where
            Self: core::marker::Sized,
            O2: Clone,
        {
            impls::Value {
                parser: self,
                val,
                marker: Default::default(),
            }
        }
        /// Produce a type's default value
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<u32> {
        ///     alpha1.default_value().parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd"), Ok(("", 0)));
        /// assert!(parser.parse_peek("123abcd;").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn default_value<O2>(self) -> impls::DefaultValue<Self, I, O, O2, E>
        where
            Self: core::marker::Sized,
            O2: core::default::Default,
        {
            impls::DefaultValue {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Discards the output of the `Parser`
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<()> {
        ///     alpha1.void().parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd"), Ok(("", ())));
        /// assert!(parser.parse_peek("123abcd;").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn void(self) -> impls::Void<Self, I, O, E>
        where
            Self: core::marker::Sized,
        {
            impls::Void {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Convert the parser's output to another type using [`std::convert::From`]
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::error::ContextError;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser1<'s>(i: &mut &'s str) -> ModalResult<&'s str> {
        ///   alpha1(i)
        /// }
        ///
        /// let mut parser2 = parser1.output_into();
        ///
        /// // the parser converts the &str output of the child parser into a Vec<u8>
        /// let bytes: ModalResult<(_, Vec<u8>), _> = parser2.parse_peek("abcd");
        /// assert_eq!(bytes, Ok(("", vec![97, 98, 99, 100])));
        /// # }
        /// ```
        #[inline(always)]
        fn output_into<O2>(self) -> impls::OutputInto<Self, I, O, O2, E>
        where
            Self: core::marker::Sized,
            O: Into<O2>,
        {
            impls::OutputInto {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Produce the consumed input as produced value.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::{alpha1};
        /// use winnow::combinator::separated_pair;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     separated_pair(alpha1, ',', alpha1).take().parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd,efgh"), Ok(("", "abcd,efgh")));
        /// assert!(parser.parse_peek("abcd;").is_err());
        /// # }
        /// ```
        #[doc(alias = "concat")]
        #[doc(alias = "recognize")]
        #[inline(always)]
        fn take(self) -> impls::Take<Self, I, O, E>
        where
            Self: core::marker::Sized,
            I: Stream,
        {
            impls::Take {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Produce the consumed input with the output
        ///
        /// Functions similarly to [take][Parser::take] except it
        /// returns the parser output as well.
        ///
        /// This can be useful especially in cases where the output is not the same type
        /// as the input, or the input is a user defined type.
        ///
        /// Returned tuple is of the format `(produced output, consumed input)`.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode};
        /// use winnow::ascii::{alpha1};
        /// use winnow::token::literal;
        /// use winnow::combinator::separated_pair;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<(bool, &'i str)> {
        ///     separated_pair(alpha1, ',', alpha1).value(true).with_taken().parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd,efgh1"), Ok(("1", (true, "abcd,efgh"))));
        /// assert!(parser.parse_peek("abcd;").is_err());
        /// # }
        /// ```
        #[doc(alias = "consumed")]
        #[doc(alias = "with_recognized")]
        #[inline(always)]
        fn with_taken(self) -> impls::WithTaken<Self, I, O, E>
        where
            Self: core::marker::Sized,
            I: Stream,
        {
            impls::WithTaken {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Produce the location of the consumed input as produced value.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, stream::Stream};
        /// # use std::ops::Range;
        /// use winnow::stream::LocatingSlice;
        /// use winnow::ascii::alpha1;
        /// use winnow::combinator::separated_pair;
        ///
        /// fn parser<'i>(input: &mut LocatingSlice<&'i str>) -> ModalResult<(Range<usize>, Range<usize>)> {
        ///     separated_pair(alpha1.span(), ',', alpha1.span()).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse(LocatingSlice::new("abcd,efgh")), Ok((0..4, 5..9)));
        /// assert!(parser.parse_peek(LocatingSlice::new("abcd;")).is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn span(self) -> impls::Span<Self, I, O, E>
        where
            Self: core::marker::Sized,
            I: Stream + Location,
        {
            impls::Span {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Produce the location of consumed input with the output
        ///
        /// Functions similarly to [`Parser::span`] except it
        /// returns the parser output as well.
        ///
        /// This can be useful especially in cases where the output is not the same type
        /// as the input, or the input is a user defined type.
        ///
        /// Returned tuple is of the format `(produced output, consumed input)`.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, stream::Stream};
        /// # use std::ops::Range;
        /// use winnow::stream::LocatingSlice;
        /// use winnow::ascii::alpha1;
        /// use winnow::token::literal;
        /// use winnow::combinator::separated_pair;
        ///
        /// fn parser<'i>(input: &mut LocatingSlice<&'i str>) -> ModalResult<((usize, Range<usize>), (usize, Range<usize>))> {
        ///     separated_pair(alpha1.value(1).with_span(), ',', alpha1.value(2).with_span()).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse(LocatingSlice::new("abcd,efgh")), Ok(((1, 0..4), (2, 5..9))));
        /// assert!(parser.parse_peek(LocatingSlice::new("abcd;")).is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn with_span(self) -> impls::WithSpan<Self, I, O, E>
        where
            Self: core::marker::Sized,
            I: Stream + Location,
        {
            impls::WithSpan {
                parser: self,
                marker: Default::default(),
            }
        }
        /// Maps a function over the output of a parser
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::ascii::digit1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<usize> {
        ///     digit1.map(|s: &str| s.len()).parse_next(input)
        /// }
        ///
        /// // the parser will count how many characters were returned by digit1
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", 6)));
        ///
        /// // this will fail if digit1 fails
        /// assert!(parser.parse_peek("abc").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn map<G, O2>(self, map: G) -> impls::Map<Self, G, I, O, O2, E>
        where
            G: FnMut(O) -> O2,
            Self: core::marker::Sized,
        {
            impls::Map {
                parser: self,
                map,
                marker: Default::default(),
            }
        }
        /// Applies a function returning a `Result` over the output of a parser.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::digit1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<u8> {
        ///     digit1.try_map(|s: &str| s.parse::<u8>()).parse_next(input)
        /// }
        ///
        /// // the parser will convert the result of digit1 to a number
        /// assert_eq!(parser.parse_peek("123"), Ok(("", 123)));
        ///
        /// // this will fail if digit1 fails
        /// assert!(parser.parse_peek("abc").is_err());
        ///
        /// // this will fail if the mapped function fails (a `u8` is too small to hold `123456`)
        /// assert!(parser.parse_peek("123456").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn try_map<G, O2, E2>(self, map: G) -> impls::TryMap<Self, G, I, O, O2, E, E2>
        where
            Self: core::marker::Sized,
            G: FnMut(O) -> Result<O2, E2>,
            I: Stream,
            E: FromExternalError<I, E2>,
            E: ParserError<I>,
        {
            impls::TryMap {
                parser: self,
                map,
                marker: Default::default(),
            }
        }
        /// Apply both [`Parser::verify`] and [`Parser::map`].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::digit1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<u8> {
        ///     digit1.verify_map(|s: &str| s.parse::<u8>().ok()).parse_next(input)
        /// }
        ///
        /// // the parser will convert the result of digit1 to a number
        /// assert_eq!(parser.parse_peek("123"), Ok(("", 123)));
        ///
        /// // this will fail if digit1 fails
        /// assert!(parser.parse_peek("abc").is_err());
        ///
        /// // this will fail if the mapped function fails (a `u8` is too small to hold `123456`)
        /// assert!(parser.parse_peek("123456").is_err());
        /// # }
        /// ```
        #[doc(alias = "satisfy_map")]
        #[doc(alias = "filter_map")]
        #[doc(alias = "map_opt")]
        #[inline(always)]
        fn verify_map<G, O2>(self, map: G) -> impls::VerifyMap<Self, G, I, O, O2, E>
        where
            Self: core::marker::Sized,
            G: FnMut(O) -> Option<O2>,
            I: Stream,
            E: ParserError<I>,
        {
            impls::VerifyMap {
                parser: self,
                map,
                marker: Default::default(),
            }
        }
        /// Creates a parser from the output of this one
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "binary")] {
        /// # use winnow::{error::ErrMode, ModalResult, Parser};
        /// use winnow::token::take;
        /// use winnow::binary::u8;
        ///
        /// fn length_take<'s>(input: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
        ///     u8.flat_map(take).parse_next(input)
        /// }
        ///
        /// assert_eq!(length_take.parse_peek(&[2, 0, 1, 2][..]), Ok((&[2][..], &[0, 1][..])));
        /// assert!(length_take.parse_peek(&[4, 0, 1, 2][..]).is_err());
        /// # }
        /// ```
        ///
        /// which is the same as
        /// ```rust
        /// # #[cfg(feature = "binary")] {
        /// # use winnow::{error::ErrMode, ModalResult, Parser};
        /// use winnow::token::take;
        /// use winnow::binary::u8;
        ///
        /// fn length_take<'s>(input: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
        ///     let length = u8.parse_next(input)?;
        ///     let data = take(length).parse_next(input)?;
        ///     Ok(data)
        /// }
        ///
        /// assert_eq!(length_take.parse_peek(&[2, 0, 1, 2][..]), Ok((&[2][..], &[0, 1][..])));
        /// assert!(length_take.parse_peek(&[4, 0, 1, 2][..]).is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn flat_map<G, H, O2>(self, map: G) -> impls::FlatMap<Self, G, H, I, O, O2, E>
        where
            Self: core::marker::Sized,
            G: FnMut(O) -> H,
            H: Parser<I, O2, E>,
        {
            impls::FlatMap {
                f: self,
                g: map,
                marker: Default::default(),
            }
        }
        /// Applies a second parser over the output of the first one
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::digit1;
        /// use winnow::token::take;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     take(5u8).and_then(digit1).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("12345"), Ok(("", "12345")));
        /// assert_eq!(parser.parse_peek("123ab"), Ok(("", "123")));
        /// assert!(parser.parse_peek("123").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn and_then<G, O2>(self, inner: G) -> impls::AndThen<Self, G, I, O, O2, E>
        where
            Self: core::marker::Sized,
            G: Parser<O, O2, E>,
            O: StreamIsPartial,
            I: Stream,
        {
            impls::AndThen {
                outer: self,
                inner,
                marker: Default::default(),
            }
        }
        /// Apply [`std::str::FromStr`] to the output of the parser
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// use winnow::{error::ErrMode, Parser};
        /// use winnow::ascii::digit1;
        ///
        /// fn parser<'s>(input: &mut &'s str) -> ModalResult<u64> {
        ///     digit1.parse_to().parse_next(input)
        /// }
        ///
        /// // the parser will count how many characters were returned by digit1
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", 123456)));
        ///
        /// // this will fail if digit1 fails
        /// assert!(parser.parse_peek("abc").is_err());
        /// # }
        /// ```
        #[doc(alias = "from_str")]
        #[inline(always)]
        fn parse_to<O2>(self) -> impls::ParseTo<Self, I, O, O2, E>
        where
            Self: core::marker::Sized,
            I: Stream,
            O: ParseSlice<O2>,
            E: ParserError<I>,
        {
            impls::ParseTo {
                p: self,
                marker: Default::default(),
            }
        }
        /// Returns the output of the child parser if it satisfies a verification function.
        ///
        /// The verification function takes as argument a reference to the output of the
        /// parser.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::ascii::alpha1;
        /// # use winnow::prelude::*;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     alpha1.verify(|s: &str| s.len() == 4).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd"), Ok(("", "abcd")));
        /// assert!(parser.parse_peek("abcde").is_err());
        /// assert!(parser.parse_peek("123abcd;").is_err());
        /// # }
        /// ```
        #[doc(alias = "satisfy")]
        #[doc(alias = "filter")]
        #[inline(always)]
        fn verify<G, O2>(self, filter: G) -> impls::Verify<Self, G, I, O, O2, E>
        where
            Self: core::marker::Sized,
            G: FnMut(&O2) -> bool,
            I: Stream,
            O: core::borrow::Borrow<O2>,
            O2: ?Sized,
            E: ParserError<I>,
        {
            impls::Verify {
                parser: self,
                filter,
                marker: Default::default(),
            }
        }
        /// If parsing fails, add context to the error
        ///
        /// This is used mainly to add user friendly information
        /// to errors when backtracking through a parse tree.
        ///
        /// See also [tutorial][crate::_tutorial::chapter_7].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::ascii::digit1;
        /// # use winnow::error::StrContext;
        /// # use winnow::error::StrContextValue;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     digit1
        ///       .context(StrContext::Expected(StrContextValue::Description("digit")))
        ///       .parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", "123456")));
        /// assert!(parser.parse_peek("abc").is_err());
        /// # }
        /// ```
        #[doc(alias = "labelled")]
        #[inline(always)]
        fn context<C>(self, context: C) -> impls::Context<Self, I, O, E, C>
        where
            Self: core::marker::Sized,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            C: Clone + core::fmt::Debug,
        {
            impls::Context {
                parser: self,
                context,
                marker: Default::default(),
            }
        }
        /// If parsing fails, dynamically add context to the error
        ///
        /// This is used mainly to add user friendly information
        /// to errors when backtracking through a parse tree.
        ///
        /// See also [tutorial][crate::_tutorial::chapter_7].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, Parser};
        /// # use winnow::ascii::digit1;
        /// # use winnow::error::StrContext;
        /// # use winnow::error::StrContextValue;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     digit1
        ///       .context_with(|| {
        ///         "0123456789".chars().map(|c| StrContext::Expected(c.into()))
        ///       })
        ///       .parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", "123456")));
        /// assert!(parser.parse_peek("abc").is_err());
        /// # }
        /// ```
        #[doc(alias = "labelled")]
        #[inline(always)]
        fn context_with<F, C, FI>(
            self,
            context: F,
        ) -> impls::ContextWith<Self, I, O, E, F, C, FI>
        where
            Self: core::marker::Sized,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            F: Fn() -> FI + Clone,
            C: core::fmt::Debug,
            FI: Iterator<Item = C>,
        {
            impls::ContextWith {
                parser: self,
                context,
                marker: Default::default(),
            }
        }
        /// Maps a function over the error of a parser
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::Parser;
        /// # use winnow::Result;
        /// # use winnow::ascii::digit1;
        /// # use winnow::error::StrContext;
        /// # use winnow::error::AddContext;
        /// # use winnow::error::ContextError;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> Result<&'i str> {
        ///     digit1.map_err(|mut e: ContextError| {
        ///         e.extend("0123456789".chars().map(|c| StrContext::Expected(c.into())));
        ///         e
        ///     }).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", "123456")));
        /// assert!(parser.parse_peek("abc").is_err());
        /// # }
        /// ```
        #[inline(always)]
        fn map_err<G, E2>(self, map: G) -> impls::MapErr<Self, G, I, O, E, E2>
        where
            G: FnMut(E) -> E2,
            Self: core::marker::Sized,
        {
            impls::MapErr {
                parser: self,
                map,
                marker: Default::default(),
            }
        }
        /// Transforms [`Incomplete`][crate::error::ErrMode::Incomplete] into [`Backtrack`][crate::error::ErrMode::Backtrack]
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::InputError, stream::Partial, Parser};
        /// # use winnow::token::take;
        /// # use winnow::prelude::*;
        /// # fn main() {
        ///
        /// fn parser<'i>(input: &mut Partial<&'i str>) -> ModalResult<&'i str, InputError<Partial<&'i str>>> {
        ///     take(5u8).complete_err().parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek(Partial::new("abcdefg")), Ok((Partial::new("fg"), "abcde")));
        /// assert_eq!(parser.parse_peek(Partial::new("abcd")), Err(ErrMode::Backtrack(InputError::at(Partial::new("abcd")))));
        /// # }
        /// ```
        #[inline(always)]
        fn complete_err(self) -> impls::CompleteErr<Self, I, O, E>
        where
            Self: core::marker::Sized,
        {
            impls::CompleteErr {
                p: self,
                marker: Default::default(),
            }
        }
        /// Convert the parser's error to another type using [`std::convert::From`]
        #[inline(always)]
        fn err_into<E2>(self) -> impls::ErrInto<Self, I, O, E, E2>
        where
            Self: core::marker::Sized,
            E: Into<E2>,
        {
            impls::ErrInto {
                parser: self,
                marker: Default::default(),
            }
        }
    }
    impl<I, O, E, F> Parser<I, O, E> for F
    where
        F: FnMut(&mut I) -> Result<O, E>,
        I: Stream,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<O, E> {
            self(i)
        }
    }
    /// This is a shortcut for [`one_of`][crate::token::one_of].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// fn parser<'s>(i: &mut &'s [u8]) -> ModalResult<u8>  {
    ///     b'a'.parse_next(i)
    /// }
    /// assert_eq!(parser.parse_peek(&b"abc"[..]), Ok((&b"bc"[..], b'a')));
    /// assert!(parser.parse_peek(&b" abc"[..]).is_err());
    /// assert!(parser.parse_peek(&b"bc"[..]).is_err());
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    impl<I, E> Parser<I, u8, E> for u8
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<u8>,
        E: ParserError<I>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<u8, E> {
            crate::token::literal(*self).value(*self).parse_next(i)
        }
    }
    /// This is a shortcut for [`one_of`][crate::token::one_of].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// fn parser<'s>(i: &mut &'s str) -> ModalResult<char> {
    ///     'a'.parse_next(i)
    /// }
    /// assert_eq!(parser.parse_peek("abc"), Ok(("bc", 'a')));
    /// assert!(parser.parse_peek(" abc").is_err());
    /// assert!(parser.parse_peek("bc").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    impl<I, E> Parser<I, char, E> for char
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<char>,
        E: ParserError<I>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<char, E> {
            crate::token::literal(*self).value(*self).parse_next(i)
        }
    }
    /// This is a shortcut for [`literal`][crate::token::literal].
    ///
    /// # Example
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::combinator::alt;
    /// # use winnow::token::take;
    ///
    /// fn parser<'s>(s: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
    ///   alt((&"Hello"[..], take(5usize))).parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"Hello, World!"[..]), Ok((&b", World!"[..], &b"Hello"[..])));
    /// assert_eq!(parser.parse_peek(&b"Something"[..]), Ok((&b"hing"[..], &b"Somet"[..])));
    /// assert!(parser.parse_peek(&b"Some"[..]).is_err());
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    impl<'s, I, E: ParserError<I>> Parser<I, <I as Stream>::Slice, E> for &'s [u8]
    where
        I: Compare<&'s [u8]> + StreamIsPartial,
        I: Stream,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
            crate::token::literal(*self).parse_next(i)
        }
    }
    /// This is a shortcut for [`literal`][crate::token::literal].
    ///
    /// # Example
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::combinator::alt;
    /// # use winnow::token::take;
    ///
    /// fn parser<'s>(s: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
    ///   alt((b"Hello", take(5usize))).parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"Hello, World!"[..]), Ok((&b", World!"[..], &b"Hello"[..])));
    /// assert_eq!(parser.parse_peek(&b"Something"[..]), Ok((&b"hing"[..], &b"Somet"[..])));
    /// assert!(parser.parse_peek(&b"Some"[..]).is_err());
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    impl<'s, I, E: ParserError<I>, const N: usize> Parser<I, <I as Stream>::Slice, E>
    for &'s [u8; N]
    where
        I: Compare<&'s [u8; N]> + StreamIsPartial,
        I: Stream,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
            crate::token::literal(*self).parse_next(i)
        }
    }
    /// This is a shortcut for [`literal`][crate::token::literal].
    ///
    /// # Example
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// # use winnow::combinator::alt;
    /// # use winnow::token::take;
    ///
    /// fn parser<'s>(s: &mut &'s str) -> ModalResult<&'s str> {
    ///   alt(("Hello", take(5usize))).parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("Hello, World!"), Ok((", World!", "Hello")));
    /// assert_eq!(parser.parse_peek("Something"), Ok(("hing", "Somet")));
    /// assert!(parser.parse_peek("Some").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    impl<'s, I, E: ParserError<I>> Parser<I, <I as Stream>::Slice, E> for &'s str
    where
        I: Compare<&'s str> + StreamIsPartial,
        I: Stream,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
            crate::token::literal(*self).parse_next(i)
        }
    }
    impl<I: Stream, E: ParserError<I>> Parser<I, (), E> for () {
        #[inline(always)]
        fn parse_next(&mut self, _i: &mut I) -> Result<(), E> {
            Ok(())
        }
    }
    #[allow(non_snake_case)]
    impl<I: Stream, O0, E: ParserError<I>, P0> Parser<I, (O0,), E> for (P0,)
    where
        P0: Parser<I, O0, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0,), E> {
            let O0 = self.0.parse_next(i)?;
            Ok((O0,))
        }
    }
    #[allow(non_snake_case)]
    impl<I: Stream, O0, O1, E: ParserError<I>, P0, P1> Parser<I, (O0, O1), E>
    for (P0, P1)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            Ok((O0, O1))
        }
    }
    #[allow(non_snake_case)]
    impl<I: Stream, O0, O1, O2, E: ParserError<I>, P0, P1, P2> Parser<I, (O0, O1, O2), E>
    for (P0, P1, P2)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1, O2), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            Ok((O0, O1, O2))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
    > Parser<I, (O0, O1, O2, O3), E> for (P0, P1, P2, P3)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1, O2, O3), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            Ok((O0, O1, O2, O3))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
    > Parser<I, (O0, O1, O2, O3, O4), E> for (P0, P1, P2, P3, P4)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1, O2, O3, O4), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
    > Parser<I, (O0, O1, O2, O3, O4, O5), E> for (P0, P1, P2, P3, P4, P5)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1, O2, O3, O4, O5), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        O6,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
        P6,
    > Parser<I, (O0, O1, O2, O3, O4, O5, O6), E> for (P0, P1, P2, P3, P4, P5, P6)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
        P6: Parser<I, O6, E>,
    {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<(O0, O1, O2, O3, O4, O5, O6), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            let O6 = self.6.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5, O6))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        O6,
        O7,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
        P6,
        P7,
    > Parser<I, (O0, O1, O2, O3, O4, O5, O6, O7), E> for (P0, P1, P2, P3, P4, P5, P6, P7)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
        P6: Parser<I, O6, E>,
        P7: Parser<I, O7, E>,
    {
        #[inline(always)]
        fn parse_next(
            &mut self,
            i: &mut I,
        ) -> Result<(O0, O1, O2, O3, O4, O5, O6, O7), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            let O6 = self.6.parse_next(i)?;
            let O7 = self.7.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5, O6, O7))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        O6,
        O7,
        O8,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
        P6,
        P7,
        P8,
    > Parser<I, (O0, O1, O2, O3, O4, O5, O6, O7, O8), E>
    for (P0, P1, P2, P3, P4, P5, P6, P7, P8)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
        P6: Parser<I, O6, E>,
        P7: Parser<I, O7, E>,
        P8: Parser<I, O8, E>,
    {
        #[inline(always)]
        fn parse_next(
            &mut self,
            i: &mut I,
        ) -> Result<(O0, O1, O2, O3, O4, O5, O6, O7, O8), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            let O6 = self.6.parse_next(i)?;
            let O7 = self.7.parse_next(i)?;
            let O8 = self.8.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5, O6, O7, O8))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        O6,
        O7,
        O8,
        O9,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
        P6,
        P7,
        P8,
        P9,
    > Parser<I, (O0, O1, O2, O3, O4, O5, O6, O7, O8, O9), E>
    for (P0, P1, P2, P3, P4, P5, P6, P7, P8, P9)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
        P6: Parser<I, O6, E>,
        P7: Parser<I, O7, E>,
        P8: Parser<I, O8, E>,
        P9: Parser<I, O9, E>,
    {
        #[inline(always)]
        fn parse_next(
            &mut self,
            i: &mut I,
        ) -> Result<(O0, O1, O2, O3, O4, O5, O6, O7, O8, O9), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            let O6 = self.6.parse_next(i)?;
            let O7 = self.7.parse_next(i)?;
            let O8 = self.8.parse_next(i)?;
            let O9 = self.9.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5, O6, O7, O8, O9))
        }
    }
    #[allow(non_snake_case)]
    impl<
        I: Stream,
        O0,
        O1,
        O2,
        O3,
        O4,
        O5,
        O6,
        O7,
        O8,
        O9,
        O10,
        E: ParserError<I>,
        P0,
        P1,
        P2,
        P3,
        P4,
        P5,
        P6,
        P7,
        P8,
        P9,
        P10,
    > Parser<I, (O0, O1, O2, O3, O4, O5, O6, O7, O8, O9, O10), E>
    for (P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10)
    where
        P0: Parser<I, O0, E>,
        P1: Parser<I, O1, E>,
        P2: Parser<I, O2, E>,
        P3: Parser<I, O3, E>,
        P4: Parser<I, O4, E>,
        P5: Parser<I, O5, E>,
        P6: Parser<I, O6, E>,
        P7: Parser<I, O7, E>,
        P8: Parser<I, O8, E>,
        P9: Parser<I, O9, E>,
        P10: Parser<I, O10, E>,
    {
        #[inline(always)]
        fn parse_next(
            &mut self,
            i: &mut I,
        ) -> Result<(O0, O1, O2, O3, O4, O5, O6, O7, O8, O9, O10), E> {
            let O0 = self.0.parse_next(i)?;
            let O1 = self.1.parse_next(i)?;
            let O2 = self.2.parse_next(i)?;
            let O3 = self.3.parse_next(i)?;
            let O4 = self.4.parse_next(i)?;
            let O5 = self.5.parse_next(i)?;
            let O6 = self.6.parse_next(i)?;
            let O7 = self.7.parse_next(i)?;
            let O8 = self.8.parse_next(i)?;
            let O9 = self.9.parse_next(i)?;
            let O10 = self.10.parse_next(i)?;
            Ok((O0, O1, O2, O3, O4, O5, O6, O7, O8, O9, O10))
        }
    }
    use alloc::boxed::Box;
    impl<I, O, E> Parser<I, O, E> for Box<dyn Parser<I, O, E> + '_> {
        #[inline(always)]
        fn parse_next(&mut self, i: &mut I) -> Result<O, E> {
            (**self).parse_next(i)
        }
    }
    /// Trait alias for [`Parser`] to be used with [`ModalResult`][crate::error::ModalResult]
    pub trait ModalParser<I, O, E>: Parser<I, O, crate::error::ErrMode<E>> {}
    impl<I, O, E, P> ModalParser<I, O, E> for P
    where
        P: Parser<I, O, crate::error::ErrMode<E>>,
    {}
}
pub mod stream {
    //! Stream capability for combinators to parse
    //!
    //! Stream types include:
    //! - `&[u8]` and [`Bytes`] for binary data
    //! - `&str` (aliased as [`Str`]) and [`BStr`] for UTF-8 data
    //! - [`LocatingSlice`] can track the location within the original buffer to report
    //!   [spans][crate::Parser::with_span]
    //! - [`Stateful`] to thread global state through your parsers
    //! - [`Partial`] can mark an input as partial buffer that is being streamed into
    //! - [Custom stream types][crate::_topic::stream]
    use core::hash::BuildHasher;
    use core::iter::{Cloned, Enumerate};
    use core::num::NonZeroUsize;
    use core::slice::Iter;
    use core::str::from_utf8;
    use core::str::CharIndices;
    use core::str::FromStr;
    use alloc::borrow::Cow;
    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;
    use alloc::collections::VecDeque;
    use alloc::string::String;
    use alloc::vec::Vec;
    use std::collections::HashMap;
    use std::collections::HashSet;
    mod bstr {
        use core::num::NonZeroUsize;
        use crate::stream::AsBStr;
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::FindSlice;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        use core::iter::{Cloned, Enumerate};
        use core::slice::Iter;
        use core::{cmp::Ordering, fmt, ops};
        /// Improved `Debug` experience for `&[u8]` UTF-8-ish streams
        #[allow(clippy::derived_hash_with_manual_eq)]
        #[repr(transparent)]
        pub struct BStr([u8]);
        #[automatically_derived]
        #[allow(clippy::derived_hash_with_manual_eq)]
        impl ::core::hash::Hash for BStr {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl BStr {
            /// Make a stream out of a byte slice-like.
            #[inline]
            pub fn new<B: ?Sized + AsRef<[u8]>>(bytes: &B) -> &Self {
                Self::from_bytes(bytes.as_ref())
            }
            #[inline]
            fn from_bytes(slice: &[u8]) -> &Self {
                unsafe { core::mem::transmute(slice) }
            }
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                &self.0
            }
        }
        impl SliceLen for &BStr {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.len()
            }
        }
        impl<'i> Stream for &'i BStr {
            type Token = u8;
            type Slice = &'i [u8];
            type IterOffsets = Enumerate<Cloned<Iter<'i, u8>>>;
            type Checkpoint = Checkpoint<Self, Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.iter().cloned().enumerate()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.len()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                if self.is_empty() {
                    None
                } else {
                    let token = self[0];
                    *self = &self[1..];
                    Some(token)
                }
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                if self.is_empty() { None } else { Some(self[0]) }
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.iter().position(|b| predicate(*b))
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                if let Some(needed) = tokens
                    .checked_sub(self.len())
                    .and_then(NonZeroUsize::new)
                {
                    Err(Needed::Size(needed))
                } else {
                    Ok(tokens)
                }
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                let (slice, next) = self.0.split_at(offset);
                *self = BStr::from_bytes(next);
                slice
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                self.peek_slice(offset);
                let slice = unsafe { self.0.get_unchecked(..offset) };
                let next = unsafe { self.0.get_unchecked(offset..) };
                *self = BStr::from_bytes(next);
                slice
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                &self[..offset]
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                self.peek_slice(offset);
                let slice = unsafe { self.0.get_unchecked(..offset) };
                slice
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(*self)
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                *self = checkpoint.inner;
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{0:#?}", self))
            }
        }
        impl StreamIsPartial for &BStr {
            type PartialState = ();
            #[inline]
            fn complete(&mut self) -> Self::PartialState {}
            #[inline]
            fn restore_partial(&mut self, _state: Self::PartialState) {}
            #[inline(always)]
            fn is_partial_supported() -> bool {
                false
            }
        }
        impl Offset for &BStr {
            #[inline(always)]
            fn offset_from(&self, start: &Self) -> usize {
                self.as_bytes().offset_from(&start.as_bytes())
            }
        }
        impl<'a> Offset<<&'a BStr as Stream>::Checkpoint> for &'a BStr {
            #[inline(always)]
            fn offset_from(&self, other: &<&'a BStr as Stream>::Checkpoint) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl AsBStr for &BStr {
            #[inline(always)]
            fn as_bstr(&self) -> &[u8] {
                (*self).as_bytes()
            }
        }
        impl<'a, T> Compare<T> for &'a BStr
        where
            &'a [u8]: Compare<T>,
        {
            #[inline(always)]
            fn compare(&self, t: T) -> CompareResult {
                let bytes = (*self).as_bytes();
                bytes.compare(t)
            }
        }
        impl<'i, S> FindSlice<S> for &'i BStr
        where
            &'i [u8]: FindSlice<S>,
        {
            #[inline(always)]
            fn find_slice(&self, substr: S) -> Option<core::ops::Range<usize>> {
                let bytes = (*self).as_bytes();
                let offset = bytes.find_slice(substr);
                offset
            }
        }
        impl UpdateSlice for &BStr {
            #[inline(always)]
            fn update_slice(self, inner: Self::Slice) -> Self {
                BStr::new(inner)
            }
        }
        impl fmt::Display for BStr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                alloc::string::String::from_utf8_lossy(self.as_bytes()).fmt(f)
            }
        }
        impl fmt::Debug for BStr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if !f.alternate() {
                    f.write_fmt(format_args!("\""))?;
                }
                for byte in self.as_bytes() {
                    let c = *byte as char;
                    f.write_fmt(format_args!("{0}", c.escape_debug()))?;
                }
                if !f.alternate() {
                    f.write_fmt(format_args!("\""))?;
                }
                Ok(())
            }
        }
        impl ops::Deref for BStr {
            type Target = [u8];
            #[inline]
            fn deref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl ops::Index<usize> for BStr {
            type Output = u8;
            #[inline]
            fn index(&self, idx: usize) -> &u8 {
                &self.as_bytes()[idx]
            }
        }
        impl ops::Index<ops::RangeFull> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, _: ops::RangeFull) -> &BStr {
                self
            }
        }
        impl ops::Index<ops::Range<usize>> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, r: ops::Range<usize>) -> &BStr {
                BStr::new(&self.as_bytes()[r.start..r.end])
            }
        }
        impl ops::Index<ops::RangeInclusive<usize>> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, r: ops::RangeInclusive<usize>) -> &BStr {
                BStr::new(&self.as_bytes()[*r.start()..=*r.end()])
            }
        }
        impl ops::Index<ops::RangeFrom<usize>> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, r: ops::RangeFrom<usize>) -> &BStr {
                BStr::new(&self.as_bytes()[r.start..])
            }
        }
        impl ops::Index<ops::RangeTo<usize>> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, r: ops::RangeTo<usize>) -> &BStr {
                BStr::new(&self.as_bytes()[..r.end])
            }
        }
        impl ops::Index<ops::RangeToInclusive<usize>> for BStr {
            type Output = BStr;
            #[inline]
            fn index(&self, r: ops::RangeToInclusive<usize>) -> &BStr {
                BStr::new(&self.as_bytes()[..=r.end])
            }
        }
        impl AsRef<[u8]> for BStr {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl AsRef<BStr> for [u8] {
            #[inline]
            fn as_ref(&self) -> &BStr {
                BStr::new(self)
            }
        }
        impl AsRef<BStr> for str {
            #[inline]
            fn as_ref(&self) -> &BStr {
                BStr::new(self)
            }
        }
        impl alloc::borrow::ToOwned for BStr {
            type Owned = alloc::vec::Vec<u8>;
            #[inline]
            fn to_owned(&self) -> Self::Owned {
                alloc::vec::Vec::from(self.as_bytes())
            }
        }
        impl core::borrow::Borrow<BStr> for alloc::vec::Vec<u8> {
            #[inline]
            fn borrow(&self) -> &BStr {
                BStr::from_bytes(self.as_slice())
            }
        }
        impl<'a> Default for &'a BStr {
            fn default() -> &'a BStr {
                BStr::new(b"")
            }
        }
        impl<'a> From<&'a [u8]> for &'a BStr {
            #[inline]
            fn from(s: &'a [u8]) -> &'a BStr {
                BStr::new(s)
            }
        }
        impl<'a> From<&'a BStr> for &'a [u8] {
            #[inline]
            fn from(s: &'a BStr) -> &'a [u8] {
                BStr::as_bytes(s)
            }
        }
        impl<'a> From<&'a str> for &'a BStr {
            #[inline]
            fn from(s: &'a str) -> &'a BStr {
                BStr::new(s.as_bytes())
            }
        }
        impl Eq for BStr {}
        impl PartialEq<BStr> for BStr {
            #[inline]
            fn eq(&self, other: &BStr) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<[u8]> for BStr {
            #[inline]
            fn eq(&self, other: &[u8]) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<BStr> for [u8] {
            #[inline]
            fn eq(&self, other: &BStr) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<&'a [u8]> for BStr {
            #[inline]
            fn eq(&self, other: &&'a [u8]) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<BStr> for &'a [u8] {
            #[inline]
            fn eq(&self, other: &BStr) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<str> for BStr {
            #[inline]
            fn eq(&self, other: &str) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<BStr> for str {
            #[inline]
            fn eq(&self, other: &BStr) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<&'a str> for BStr {
            #[inline]
            fn eq(&self, other: &&'a str) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<BStr> for &'a str {
            #[inline]
            fn eq(&self, other: &BStr) -> bool {
                PartialEq::eq(other, self)
            }
        }
        impl PartialOrd for BStr {
            #[inline]
            fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for BStr {
            #[inline]
            fn cmp(&self, other: &BStr) -> Ordering {
                Ord::cmp(self.as_bytes(), other.as_bytes())
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<[u8]> for BStr {
            #[inline]
            fn partial_cmp(&self, other: &[u8]) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<BStr> for [u8] {
            #[inline]
            fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<&'a [u8]> for BStr {
            #[inline]
            fn partial_cmp(&self, other: &&'a [u8]) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<BStr> for &'a [u8] {
            #[inline]
            fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<str> for BStr {
            #[inline]
            fn partial_cmp(&self, other: &str) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<BStr> for str {
            #[inline]
            fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<&'a str> for BStr {
            #[inline]
            fn partial_cmp(&self, other: &&'a str) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<BStr> for &'a str {
            #[inline]
            fn partial_cmp(&self, other: &BStr) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
    }
    mod bytes {
        use core::num::NonZeroUsize;
        use crate::stream::AsBytes;
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::FindSlice;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        use core::iter::{Cloned, Enumerate};
        use core::slice::Iter;
        use core::{cmp::Ordering, fmt, ops};
        /// Improved `Debug` experience for `&[u8]` byte streams
        #[allow(clippy::derived_hash_with_manual_eq)]
        #[repr(transparent)]
        pub struct Bytes([u8]);
        #[automatically_derived]
        #[allow(clippy::derived_hash_with_manual_eq)]
        impl ::core::hash::Hash for Bytes {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl Bytes {
            /// Make a stream out of a byte slice-like.
            #[inline]
            pub fn new<B: ?Sized + AsRef<[u8]>>(bytes: &B) -> &Self {
                Self::from_bytes(bytes.as_ref())
            }
            #[inline]
            fn from_bytes(slice: &[u8]) -> &Self {
                unsafe { core::mem::transmute(slice) }
            }
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                &self.0
            }
        }
        impl SliceLen for &Bytes {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.len()
            }
        }
        impl<'i> Stream for &'i Bytes {
            type Token = u8;
            type Slice = &'i [u8];
            type IterOffsets = Enumerate<Cloned<Iter<'i, u8>>>;
            type Checkpoint = Checkpoint<Self, Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.iter().cloned().enumerate()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.len()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                if self.is_empty() {
                    None
                } else {
                    let token = self[0];
                    *self = &self[1..];
                    Some(token)
                }
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                if self.is_empty() { None } else { Some(self[0]) }
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.iter().position(|b| predicate(*b))
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                if let Some(needed) = tokens
                    .checked_sub(self.len())
                    .and_then(NonZeroUsize::new)
                {
                    Err(Needed::Size(needed))
                } else {
                    Ok(tokens)
                }
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                let (slice, next) = self.0.split_at(offset);
                *self = Bytes::from_bytes(next);
                slice
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                self.peek_slice(offset);
                let slice = unsafe { self.0.get_unchecked(..offset) };
                let next = unsafe { self.0.get_unchecked(offset..) };
                *self = Bytes::from_bytes(next);
                slice
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                &self[..offset]
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                self.peek_slice(offset);
                let slice = unsafe { self.0.get_unchecked(..offset) };
                slice
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(*self)
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                *self = checkpoint.inner;
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{0:#?}", self))
            }
        }
        impl StreamIsPartial for &Bytes {
            type PartialState = ();
            #[inline]
            fn complete(&mut self) -> Self::PartialState {}
            #[inline]
            fn restore_partial(&mut self, _state: Self::PartialState) {}
            #[inline(always)]
            fn is_partial_supported() -> bool {
                false
            }
        }
        impl Offset for &Bytes {
            #[inline(always)]
            fn offset_from(&self, start: &Self) -> usize {
                self.as_bytes().offset_from(&start.as_bytes())
            }
        }
        impl<'a> Offset<<&'a Bytes as Stream>::Checkpoint> for &'a Bytes {
            #[inline(always)]
            fn offset_from(&self, other: &<&'a Bytes as Stream>::Checkpoint) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl AsBytes for &Bytes {
            #[inline(always)]
            fn as_bytes(&self) -> &[u8] {
                (*self).as_bytes()
            }
        }
        impl<'a, T> Compare<T> for &'a Bytes
        where
            &'a [u8]: Compare<T>,
        {
            #[inline(always)]
            fn compare(&self, t: T) -> CompareResult {
                let bytes = (*self).as_bytes();
                bytes.compare(t)
            }
        }
        impl<'i, S> FindSlice<S> for &'i Bytes
        where
            &'i [u8]: FindSlice<S>,
        {
            #[inline(always)]
            fn find_slice(&self, substr: S) -> Option<core::ops::Range<usize>> {
                let bytes = (*self).as_bytes();
                let offset = bytes.find_slice(substr);
                offset
            }
        }
        impl UpdateSlice for &Bytes {
            #[inline(always)]
            fn update_slice(self, inner: Self::Slice) -> Self {
                Bytes::new(inner)
            }
        }
        impl fmt::Display for Bytes {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                <Self as fmt::UpperHex>::fmt(self, f)
            }
        }
        impl fmt::Debug for Bytes {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                <Self as fmt::UpperHex>::fmt(self, f)
            }
        }
        impl fmt::LowerHex for Bytes {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.as_bytes() {
                    f.write_fmt(format_args!("{0:0>2x}", byte))?;
                }
                Ok(())
            }
        }
        impl fmt::UpperHex for Bytes {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for (i, byte) in self.as_bytes().iter().enumerate() {
                    if 0 < i {
                        let absolute = (self.as_bytes().as_ptr() as usize) + i;
                        if f.alternate() && absolute != 0 && absolute % 4 == 0 {
                            f.write_fmt(format_args!("_"))?;
                        }
                    }
                    f.write_fmt(format_args!("{0:0>2X}", byte))?;
                }
                Ok(())
            }
        }
        impl ops::Deref for Bytes {
            type Target = [u8];
            #[inline]
            fn deref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl ops::Index<usize> for Bytes {
            type Output = u8;
            #[inline]
            fn index(&self, idx: usize) -> &u8 {
                &self.as_bytes()[idx]
            }
        }
        impl ops::Index<ops::RangeFull> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, _: ops::RangeFull) -> &Bytes {
                self
            }
        }
        impl ops::Index<ops::Range<usize>> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, r: ops::Range<usize>) -> &Bytes {
                Bytes::new(&self.as_bytes()[r.start..r.end])
            }
        }
        impl ops::Index<ops::RangeInclusive<usize>> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, r: ops::RangeInclusive<usize>) -> &Bytes {
                Bytes::new(&self.as_bytes()[*r.start()..=*r.end()])
            }
        }
        impl ops::Index<ops::RangeFrom<usize>> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, r: ops::RangeFrom<usize>) -> &Bytes {
                Bytes::new(&self.as_bytes()[r.start..])
            }
        }
        impl ops::Index<ops::RangeTo<usize>> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, r: ops::RangeTo<usize>) -> &Bytes {
                Bytes::new(&self.as_bytes()[..r.end])
            }
        }
        impl ops::Index<ops::RangeToInclusive<usize>> for Bytes {
            type Output = Bytes;
            #[inline]
            fn index(&self, r: ops::RangeToInclusive<usize>) -> &Bytes {
                Bytes::new(&self.as_bytes()[..=r.end])
            }
        }
        impl AsRef<[u8]> for Bytes {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                self.as_bytes()
            }
        }
        impl AsRef<Bytes> for [u8] {
            #[inline]
            fn as_ref(&self) -> &Bytes {
                Bytes::new(self)
            }
        }
        impl AsRef<Bytes> for str {
            #[inline]
            fn as_ref(&self) -> &Bytes {
                Bytes::new(self)
            }
        }
        impl alloc::borrow::ToOwned for Bytes {
            type Owned = alloc::vec::Vec<u8>;
            #[inline]
            fn to_owned(&self) -> Self::Owned {
                alloc::vec::Vec::from(self.as_bytes())
            }
        }
        impl core::borrow::Borrow<Bytes> for alloc::vec::Vec<u8> {
            #[inline]
            fn borrow(&self) -> &Bytes {
                Bytes::from_bytes(self.as_slice())
            }
        }
        impl<'a> Default for &'a Bytes {
            fn default() -> &'a Bytes {
                Bytes::new(b"")
            }
        }
        impl<'a> From<&'a [u8]> for &'a Bytes {
            #[inline]
            fn from(s: &'a [u8]) -> &'a Bytes {
                Bytes::new(s)
            }
        }
        impl<'a> From<&'a Bytes> for &'a [u8] {
            #[inline]
            fn from(s: &'a Bytes) -> &'a [u8] {
                Bytes::as_bytes(s)
            }
        }
        impl<'a> From<&'a str> for &'a Bytes {
            #[inline]
            fn from(s: &'a str) -> &'a Bytes {
                Bytes::new(s.as_bytes())
            }
        }
        impl Eq for Bytes {}
        impl PartialEq<Bytes> for Bytes {
            #[inline]
            fn eq(&self, other: &Bytes) -> bool {
                self.as_bytes() == other.as_bytes()
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<[u8]> for Bytes {
            #[inline]
            fn eq(&self, other: &[u8]) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<Bytes> for [u8] {
            #[inline]
            fn eq(&self, other: &Bytes) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<&'a [u8]> for Bytes {
            #[inline]
            fn eq(&self, other: &&'a [u8]) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<Bytes> for &'a [u8] {
            #[inline]
            fn eq(&self, other: &Bytes) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<str> for Bytes {
            #[inline]
            fn eq(&self, other: &str) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<Bytes> for str {
            #[inline]
            fn eq(&self, other: &Bytes) -> bool {
                PartialEq::eq(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<&'a str> for Bytes {
            #[inline]
            fn eq(&self, other: &&'a str) -> bool {
                let l = self;
                let r: &Self = other.as_ref();
                PartialEq::eq(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialEq<Bytes> for &'a str {
            #[inline]
            fn eq(&self, other: &Bytes) -> bool {
                PartialEq::eq(other, self)
            }
        }
        impl PartialOrd for Bytes {
            #[inline]
            fn partial_cmp(&self, other: &Bytes) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Bytes {
            #[inline]
            fn cmp(&self, other: &Bytes) -> Ordering {
                Ord::cmp(self.as_bytes(), other.as_bytes())
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<[u8]> for Bytes {
            #[inline]
            fn partial_cmp(&self, other: &[u8]) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<Bytes> for [u8] {
            #[inline]
            fn partial_cmp(&self, other: &Bytes) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<&'a [u8]> for Bytes {
            #[inline]
            fn partial_cmp(&self, other: &&'a [u8]) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<Bytes> for &'a [u8] {
            #[inline]
            fn partial_cmp(&self, other: &Bytes) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<str> for Bytes {
            #[inline]
            fn partial_cmp(&self, other: &str) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<Bytes> for str {
            #[inline]
            fn partial_cmp(&self, other: &Bytes) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<&'a str> for Bytes {
            #[inline]
            fn partial_cmp(&self, other: &&'a str) -> Option<Ordering> {
                let l = self;
                let r: &Self = other.as_ref();
                PartialOrd::partial_cmp(l, r)
            }
        }
        #[allow(unused_lifetimes)]
        impl<'a> PartialOrd<Bytes> for &'a str {
            #[inline]
            fn partial_cmp(&self, other: &Bytes) -> Option<Ordering> {
                PartialOrd::partial_cmp(other, self)
            }
        }
    }
    mod locating {
        use crate::stream::AsBStr;
        use crate::stream::AsBytes;
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::FindSlice;
        use crate::stream::Location;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        /// Allow collecting the span of a parsed token within a slice
        ///
        /// Converting byte offsets to line or column numbers is left up to the user, as computing column
        /// numbers requires domain knowledge (are columns byte-based, codepoint-based, or grapheme-based?)
        /// and O(n) iteration over the input to determine codepoint and line boundaries.
        ///
        /// [The `line-span` crate](https://docs.rs/line-span/latest/line_span/) can help with converting
        /// byte offsets to line numbers.
        ///
        /// See [`Parser::span`][crate::Parser::span] and [`Parser::with_span`][crate::Parser::with_span] for more details
        #[doc(alias = "LocatingSliceSpan")]
        #[doc(alias = "Located")]
        pub struct LocatingSlice<I> {
            initial: I,
            input: I,
        }
        #[automatically_derived]
        impl<I: ::core::marker::Copy> ::core::marker::Copy for LocatingSlice<I> {}
        #[automatically_derived]
        impl<I: ::core::clone::Clone> ::core::clone::Clone for LocatingSlice<I> {
            #[inline]
            fn clone(&self) -> LocatingSlice<I> {
                LocatingSlice {
                    initial: ::core::clone::Clone::clone(&self.initial),
                    input: ::core::clone::Clone::clone(&self.input),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::default::Default> ::core::default::Default for LocatingSlice<I> {
            #[inline]
            fn default() -> LocatingSlice<I> {
                LocatingSlice {
                    initial: ::core::default::Default::default(),
                    input: ::core::default::Default::default(),
                }
            }
        }
        #[automatically_derived]
        impl<I> ::core::marker::StructuralPartialEq for LocatingSlice<I> {}
        #[automatically_derived]
        impl<I: ::core::cmp::PartialEq> ::core::cmp::PartialEq for LocatingSlice<I> {
            #[inline]
            fn eq(&self, other: &LocatingSlice<I>) -> bool {
                self.initial == other.initial && self.input == other.input
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::Eq> ::core::cmp::Eq for LocatingSlice<I> {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<I>;
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::PartialOrd> ::core::cmp::PartialOrd for LocatingSlice<I> {
            #[inline]
            fn partial_cmp(
                &self,
                other: &LocatingSlice<I>,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                match ::core::cmp::PartialOrd::partial_cmp(
                    &self.initial,
                    &other.initial,
                ) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        ::core::cmp::PartialOrd::partial_cmp(&self.input, &other.input)
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::Ord> ::core::cmp::Ord for LocatingSlice<I> {
            #[inline]
            fn cmp(&self, other: &LocatingSlice<I>) -> ::core::cmp::Ordering {
                match ::core::cmp::Ord::cmp(&self.initial, &other.initial) {
                    ::core::cmp::Ordering::Equal => {
                        ::core::cmp::Ord::cmp(&self.input, &other.input)
                    }
                    cmp => cmp,
                }
            }
        }
        impl<I> LocatingSlice<I>
        where
            I: Clone + Offset,
        {
            /// Wrap another Stream with span tracking
            pub fn new(input: I) -> Self {
                let initial = input.clone();
                Self { initial, input }
            }
            #[inline]
            fn previous_token_end(&self) -> usize {
                self.input.offset_from(&self.initial)
            }
            #[inline]
            fn current_token_start(&self) -> usize {
                self.input.offset_from(&self.initial)
            }
        }
        impl<I> LocatingSlice<I>
        where
            I: Clone + Stream + Offset,
        {
            /// Reset the stream to the start
            ///
            /// This is useful for formats that encode a graph with addresses relative to the start of the
            /// input.
            #[doc(alias = "fseek")]
            #[inline]
            pub fn reset_to_start(&mut self) {
                let start = self.initial.checkpoint();
                self.input.reset(&start);
            }
        }
        impl<I> AsRef<I> for LocatingSlice<I> {
            #[inline(always)]
            fn as_ref(&self) -> &I {
                &self.input
            }
        }
        impl<I: core::fmt::Debug> core::fmt::Debug for LocatingSlice<I> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.fmt(f)
            }
        }
        impl<I> core::ops::Deref for LocatingSlice<I> {
            type Target = I;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.input
            }
        }
        impl<I: core::fmt::Display> core::fmt::Display for LocatingSlice<I> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.fmt(f)
            }
        }
        impl<I> SliceLen for LocatingSlice<I>
        where
            I: SliceLen,
        {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.input.slice_len()
            }
        }
        impl<I: Stream> Stream for LocatingSlice<I> {
            type Token = <I as Stream>::Token;
            type Slice = <I as Stream>::Slice;
            type IterOffsets = <I as Stream>::IterOffsets;
            type Checkpoint = Checkpoint<I::Checkpoint, Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.input.iter_offsets()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.input.eof_offset()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                self.input.next_token()
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                self.input.peek_token()
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.input.offset_for(predicate)
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                self.input.offset_at(tokens)
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                self.input.next_slice(offset)
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                unsafe { self.input.next_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                self.input.peek_slice(offset)
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                unsafe { self.input.peek_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(self.input.checkpoint())
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                self.input.reset(&checkpoint.inner);
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.trace(f)
            }
        }
        impl<I> Location for LocatingSlice<I>
        where
            I: Clone + Offset,
        {
            #[inline(always)]
            fn previous_token_end(&self) -> usize {
                self.previous_token_end()
            }
            #[inline(always)]
            fn current_token_start(&self) -> usize {
                self.current_token_start()
            }
        }
        impl<I> StreamIsPartial for LocatingSlice<I>
        where
            I: StreamIsPartial,
        {
            type PartialState = I::PartialState;
            #[inline]
            fn complete(&mut self) -> Self::PartialState {
                self.input.complete()
            }
            #[inline]
            fn restore_partial(&mut self, state: Self::PartialState) {
                self.input.restore_partial(state);
            }
            #[inline(always)]
            fn is_partial_supported() -> bool {
                I::is_partial_supported()
            }
            #[inline(always)]
            fn is_partial(&self) -> bool {
                self.input.is_partial()
            }
        }
        impl<I> Offset for LocatingSlice<I>
        where
            I: Stream,
        {
            #[inline(always)]
            fn offset_from(&self, other: &Self) -> usize {
                self.offset_from(&other.checkpoint())
            }
        }
        impl<I> Offset<<LocatingSlice<I> as Stream>::Checkpoint> for LocatingSlice<I>
        where
            I: Stream,
        {
            #[inline(always)]
            fn offset_from(
                &self,
                other: &<LocatingSlice<I> as Stream>::Checkpoint,
            ) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl<I> AsBytes for LocatingSlice<I>
        where
            I: AsBytes,
        {
            #[inline(always)]
            fn as_bytes(&self) -> &[u8] {
                self.input.as_bytes()
            }
        }
        impl<I> AsBStr for LocatingSlice<I>
        where
            I: AsBStr,
        {
            #[inline(always)]
            fn as_bstr(&self) -> &[u8] {
                self.input.as_bstr()
            }
        }
        impl<I, U> Compare<U> for LocatingSlice<I>
        where
            I: Compare<U>,
        {
            #[inline(always)]
            fn compare(&self, other: U) -> CompareResult {
                self.input.compare(other)
            }
        }
        impl<I, T> FindSlice<T> for LocatingSlice<I>
        where
            I: FindSlice<T>,
        {
            #[inline(always)]
            fn find_slice(&self, substr: T) -> Option<core::ops::Range<usize>> {
                self.input.find_slice(substr)
            }
        }
        impl<I> UpdateSlice for LocatingSlice<I>
        where
            I: UpdateSlice,
        {
            #[inline(always)]
            fn update_slice(mut self, inner: Self::Slice) -> Self {
                self.input = I::update_slice(self.input, inner);
                self
            }
        }
    }
    mod partial {
        use crate::stream::AsBStr;
        use crate::stream::AsBytes;
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::FindSlice;
        use crate::stream::Location;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        /// Mark the input as a partial buffer for streaming input.
        ///
        /// Complete input means that we already have all of the data. This will be the common case with
        /// small files that can be read entirely to memory.
        ///
        /// In contrast, streaming input assumes that we might not have all of the data.
        /// This can happen with some network protocol or large file parsers, where the
        /// input buffer can be full and need to be resized or refilled.
        /// - [`ErrMode::Incomplete`][crate::error::ErrMode::Incomplete] will report how much more data is needed.
        /// - [`Parser::complete_err`][crate::Parser::complete_err] transform
        ///   [`ErrMode::Incomplete`][crate::error::ErrMode::Incomplete] to
        ///   [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]
        ///
        /// See also [`StreamIsPartial`] to tell whether the input supports complete or partial parsing.
        ///
        /// See also [Special Topics: Parsing Partial Input][crate::_topic::partial].
        ///
        /// # Example
        ///
        /// Here is how it works in practice:
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{Result, error::ErrMode, error::Needed, error::ContextError, token, ascii, stream::Partial};
        /// # use winnow::prelude::*;
        ///
        /// fn take_partial<'s>(i: &mut Partial<&'s [u8]>) -> ModalResult<&'s [u8], ContextError> {
        ///   token::take(4u8).parse_next(i)
        /// }
        ///
        /// fn take_complete<'s>(i: &mut &'s [u8]) -> ModalResult<&'s [u8], ContextError> {
        ///   token::take(4u8).parse_next(i)
        /// }
        ///
        /// // both parsers will take 4 bytes as expected
        /// assert_eq!(take_partial.parse_peek(Partial::new(&b"abcde"[..])), Ok((Partial::new(&b"e"[..]), &b"abcd"[..])));
        /// assert_eq!(take_complete.parse_peek(&b"abcde"[..]), Ok((&b"e"[..], &b"abcd"[..])));
        ///
        /// // if the input is smaller than 4 bytes, the partial parser
        /// // will return `Incomplete` to indicate that we need more data
        /// assert_eq!(take_partial.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
        ///
        /// // but the complete parser will return an error
        /// assert!(take_complete.parse_peek(&b"abc"[..]).is_err());
        ///
        /// // the alpha0 function takes 0 or more alphabetic characters
        /// fn alpha0_partial<'s>(i: &mut Partial<&'s str>) -> ModalResult<&'s str, ContextError> {
        ///   ascii::alpha0.parse_next(i)
        /// }
        ///
        /// fn alpha0_complete<'s>(i: &mut &'s str) -> ModalResult<&'s str, ContextError> {
        ///   ascii::alpha0.parse_next(i)
        /// }
        ///
        /// // if there's a clear limit to the taken characters, both parsers work the same way
        /// assert_eq!(alpha0_partial.parse_peek(Partial::new("abcd;")), Ok((Partial::new(";"), "abcd")));
        /// assert_eq!(alpha0_complete.parse_peek("abcd;"), Ok((";", "abcd")));
        ///
        /// // but when there's no limit, the partial version returns `Incomplete`, because it cannot
        /// // know if more input data should be taken. The whole input could be "abcd;", or
        /// // "abcde;"
        /// assert_eq!(alpha0_partial.parse_peek(Partial::new("abcd")), Err(ErrMode::Incomplete(Needed::new(1))));
        ///
        /// // while the complete version knows that all of the data is there
        /// assert_eq!(alpha0_complete.parse_peek("abcd"), Ok(("", "abcd")));
        /// # }
        /// ```
        pub struct Partial<I> {
            input: I,
            partial: bool,
        }
        #[automatically_derived]
        impl<I: ::core::marker::Copy> ::core::marker::Copy for Partial<I> {}
        #[automatically_derived]
        impl<I: ::core::clone::Clone> ::core::clone::Clone for Partial<I> {
            #[inline]
            fn clone(&self) -> Partial<I> {
                Partial {
                    input: ::core::clone::Clone::clone(&self.input),
                    partial: ::core::clone::Clone::clone(&self.partial),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::fmt::Debug> ::core::fmt::Debug for Partial<I> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Partial",
                    "input",
                    &self.input,
                    "partial",
                    &&self.partial,
                )
            }
        }
        #[automatically_derived]
        impl<I> ::core::marker::StructuralPartialEq for Partial<I> {}
        #[automatically_derived]
        impl<I: ::core::cmp::PartialEq> ::core::cmp::PartialEq for Partial<I> {
            #[inline]
            fn eq(&self, other: &Partial<I>) -> bool {
                self.partial == other.partial && self.input == other.input
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::Eq> ::core::cmp::Eq for Partial<I> {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<I>;
                let _: ::core::cmp::AssertParamIsEq<bool>;
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::PartialOrd> ::core::cmp::PartialOrd for Partial<I> {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Partial<I>,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                match ::core::cmp::PartialOrd::partial_cmp(&self.input, &other.input) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        ::core::cmp::PartialOrd::partial_cmp(
                            &self.partial,
                            &other.partial,
                        )
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::Ord> ::core::cmp::Ord for Partial<I> {
            #[inline]
            fn cmp(&self, other: &Partial<I>) -> ::core::cmp::Ordering {
                match ::core::cmp::Ord::cmp(&self.input, &other.input) {
                    ::core::cmp::Ordering::Equal => {
                        ::core::cmp::Ord::cmp(&self.partial, &other.partial)
                    }
                    cmp => cmp,
                }
            }
        }
        impl<I> Partial<I>
        where
            I: StreamIsPartial,
        {
            /// Create a partial input
            #[inline]
            pub fn new(input: I) -> Self {
                if true {
                    if !!I::is_partial_supported() {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!("`Partial` can only wrap complete sources"),
                            );
                        }
                    }
                }
                let partial = true;
                Self { input, partial }
            }
            /// Extract the original [`Stream`]
            #[inline(always)]
            pub fn into_inner(self) -> I {
                self.input
            }
        }
        impl<I> Default for Partial<I>
        where
            I: Default + StreamIsPartial,
        {
            #[inline]
            fn default() -> Self {
                Self::new(I::default())
            }
        }
        impl<I> core::ops::Deref for Partial<I> {
            type Target = I;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.input
            }
        }
        impl<I: core::fmt::Display> core::fmt::Display for Partial<I> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.fmt(f)
            }
        }
        impl<I> SliceLen for Partial<I>
        where
            I: SliceLen,
        {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.input.slice_len()
            }
        }
        impl<I: Stream> Stream for Partial<I> {
            type Token = <I as Stream>::Token;
            type Slice = <I as Stream>::Slice;
            type IterOffsets = <I as Stream>::IterOffsets;
            type Checkpoint = Checkpoint<I::Checkpoint, Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.input.iter_offsets()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.input.eof_offset()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                self.input.next_token()
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                self.input.peek_token()
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.input.offset_for(predicate)
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                self.input.offset_at(tokens)
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                self.input.next_slice(offset)
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                unsafe { self.input.next_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                self.input.peek_slice(offset)
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                unsafe { self.input.peek_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(self.input.checkpoint())
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                self.input.reset(&checkpoint.inner);
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.trace(f)
            }
        }
        impl<I> Location for Partial<I>
        where
            I: Location,
        {
            #[inline(always)]
            fn previous_token_end(&self) -> usize {
                self.input.previous_token_end()
            }
            #[inline(always)]
            fn current_token_start(&self) -> usize {
                self.input.current_token_start()
            }
        }
        impl<I> StreamIsPartial for Partial<I>
        where
            I: StreamIsPartial,
        {
            type PartialState = bool;
            #[inline]
            fn complete(&mut self) -> Self::PartialState {
                core::mem::replace(&mut self.partial, false)
            }
            #[inline]
            fn restore_partial(&mut self, state: Self::PartialState) {
                self.partial = state;
            }
            #[inline(always)]
            fn is_partial_supported() -> bool {
                true
            }
            #[inline(always)]
            fn is_partial(&self) -> bool {
                self.partial
            }
        }
        impl<I> Offset for Partial<I>
        where
            I: Stream,
        {
            #[inline(always)]
            fn offset_from(&self, start: &Self) -> usize {
                self.offset_from(&start.checkpoint())
            }
        }
        impl<I> Offset<<Partial<I> as Stream>::Checkpoint> for Partial<I>
        where
            I: Stream,
        {
            #[inline(always)]
            fn offset_from(&self, other: &<Partial<I> as Stream>::Checkpoint) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl<I> AsBytes for Partial<I>
        where
            I: AsBytes,
        {
            #[inline(always)]
            fn as_bytes(&self) -> &[u8] {
                self.input.as_bytes()
            }
        }
        impl<I> AsBStr for Partial<I>
        where
            I: AsBStr,
        {
            #[inline(always)]
            fn as_bstr(&self) -> &[u8] {
                self.input.as_bstr()
            }
        }
        impl<I, T> Compare<T> for Partial<I>
        where
            I: Compare<T>,
        {
            #[inline(always)]
            fn compare(&self, t: T) -> CompareResult {
                self.input.compare(t)
            }
        }
        impl<I, T> FindSlice<T> for Partial<I>
        where
            I: FindSlice<T>,
        {
            #[inline(always)]
            fn find_slice(&self, substr: T) -> Option<core::ops::Range<usize>> {
                self.input.find_slice(substr)
            }
        }
        impl<I> UpdateSlice for Partial<I>
        where
            I: UpdateSlice,
        {
            #[inline(always)]
            fn update_slice(self, inner: Self::Slice) -> Self {
                Partial {
                    input: I::update_slice(self.input, inner),
                    partial: self.partial,
                }
            }
        }
    }
    mod range {
        /// A range bounded inclusively for counting parses performed
        ///
        /// This is flexible in what can be converted to a [Range]:
        /// ```rust
        /// # #[cfg(all(feature = "std", feature = "parser"))] {
        /// # use winnow::prelude::*;
        /// # use winnow::token::any;
        /// # use winnow::combinator::repeat;
        /// # fn inner(input: &mut &str) -> ModalResult<char> {
        /// #     any.parse_next(input)
        /// # }
        /// # let mut input = "0123456789012345678901234567890123456789";
        /// # let input = &mut input;
        /// let parser: Vec<_> = repeat(5, inner).parse_next(input).unwrap();
        /// # let mut input = "0123456789012345678901234567890123456789";
        /// # let input = &mut input;
        /// let parser: Vec<_> = repeat(.., inner).parse_next(input).unwrap();
        /// # let mut input = "0123456789012345678901234567890123456789";
        /// # let input = &mut input;
        /// let parser: Vec<_> = repeat(1.., inner).parse_next(input).unwrap();
        /// # let mut input = "0123456789012345678901234567890123456789";
        /// # let input = &mut input;
        /// let parser: Vec<_> = repeat(5..8, inner).parse_next(input).unwrap();
        /// # let mut input = "0123456789012345678901234567890123456789";
        /// # let input = &mut input;
        /// let parser: Vec<_> = repeat(5..=8, inner).parse_next(input).unwrap();
        /// # }
        /// ```
        pub struct Range {
            pub(crate) start_inclusive: usize,
            pub(crate) end_inclusive: Option<usize>,
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Range {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Range {
            #[inline]
            fn eq(&self, other: &Range) -> bool {
                self.start_inclusive == other.start_inclusive
                    && self.end_inclusive == other.end_inclusive
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Range {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<usize>;
                let _: ::core::cmp::AssertParamIsEq<Option<usize>>;
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Range {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Range {}
        #[automatically_derived]
        impl ::core::clone::Clone for Range {
            #[inline]
            fn clone(&self) -> Range {
                let _: ::core::clone::AssertParamIsClone<usize>;
                let _: ::core::clone::AssertParamIsClone<Option<usize>>;
                *self
            }
        }
        impl Range {
            #[inline(always)]
            fn raw(start_inclusive: usize, end_inclusive: Option<usize>) -> Self {
                Self {
                    start_inclusive,
                    end_inclusive,
                }
            }
        }
        impl core::ops::RangeBounds<usize> for Range {
            #[inline(always)]
            fn start_bound(&self) -> core::ops::Bound<&usize> {
                core::ops::Bound::Included(&self.start_inclusive)
            }
            #[inline(always)]
            fn end_bound(&self) -> core::ops::Bound<&usize> {
                if let Some(end_inclusive) = &self.end_inclusive {
                    core::ops::Bound::Included(end_inclusive)
                } else {
                    core::ops::Bound::Unbounded
                }
            }
        }
        impl From<usize> for Range {
            #[inline(always)]
            fn from(fixed: usize) -> Self {
                (fixed..=fixed).into()
            }
        }
        impl From<core::ops::Range<usize>> for Range {
            #[inline(always)]
            fn from(range: core::ops::Range<usize>) -> Self {
                let start_inclusive = range.start;
                let end_inclusive = Some(range.end.saturating_sub(1));
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl From<core::ops::RangeFull> for Range {
            #[inline(always)]
            fn from(_: core::ops::RangeFull) -> Self {
                let start_inclusive = 0;
                let end_inclusive = None;
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl From<core::ops::RangeFrom<usize>> for Range {
            #[inline(always)]
            fn from(range: core::ops::RangeFrom<usize>) -> Self {
                let start_inclusive = range.start;
                let end_inclusive = None;
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl From<core::ops::RangeTo<usize>> for Range {
            #[inline(always)]
            fn from(range: core::ops::RangeTo<usize>) -> Self {
                let start_inclusive = 0;
                let end_inclusive = Some(range.end.saturating_sub(1));
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl From<core::ops::RangeInclusive<usize>> for Range {
            #[inline(always)]
            fn from(range: core::ops::RangeInclusive<usize>) -> Self {
                let start_inclusive = *range.start();
                let end_inclusive = Some(*range.end());
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl From<core::ops::RangeToInclusive<usize>> for Range {
            #[inline(always)]
            fn from(range: core::ops::RangeToInclusive<usize>) -> Self {
                let start_inclusive = 0;
                let end_inclusive = Some(range.end);
                Self::raw(start_inclusive, end_inclusive)
            }
        }
        impl core::fmt::Display for Range {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.start_inclusive.fmt(f)?;
                match self.end_inclusive {
                    Some(e) if e == self.start_inclusive => {}
                    Some(e) => {
                        "..=".fmt(f)?;
                        e.fmt(f)?;
                    }
                    None => {
                        "..".fmt(f)?;
                    }
                }
                Ok(())
            }
        }
        impl core::fmt::Debug for Range {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{0}", self))
            }
        }
    }
    mod stateful {
        use crate::stream::AsBStr;
        use crate::stream::AsBytes;
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::FindSlice;
        use crate::stream::Location;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        /// Thread global state through your parsers
        ///
        /// Use cases
        /// - Recursion checks
        /// - Error recovery
        /// - Debugging
        ///
        /// # Example
        ///
        /// ```
        /// # #[cfg(feature = "ascii")] {
        /// # use std::cell::Cell;
        /// # use winnow::prelude::*;
        /// # use winnow::stream::Stateful;
        /// # use winnow::ascii::alpha1;
        /// # type Error = ();
        ///
        /// #[derive(Debug)]
        /// struct State<'s>(&'s mut u32);
        ///
        /// impl<'s> State<'s> {
        ///     fn count(&mut self) {
        ///         *self.0 += 1;
        ///     }
        /// }
        ///
        /// type Stream<'is> = Stateful<&'is str, State<'is>>;
        ///
        /// fn word<'s>(i: &mut Stream<'s>) -> ModalResult<&'s str> {
        ///   i.state.count();
        ///   alpha1.parse_next(i)
        /// }
        ///
        /// let data = "Hello";
        /// let mut state = 0;
        /// let input = Stream { input: data, state: State(&mut state) };
        /// let output = word.parse(input).unwrap();
        /// assert_eq!(state, 1);
        /// # }
        /// ```
        #[doc(alias = "LocatingSliceSpan")]
        pub struct Stateful<I, S> {
            /// Inner input being wrapped in state
            pub input: I,
            /// User-provided state
            pub state: S,
        }
        #[automatically_derived]
        impl<I: ::core::clone::Clone, S: ::core::clone::Clone> ::core::clone::Clone
        for Stateful<I, S> {
            #[inline]
            fn clone(&self) -> Stateful<I, S> {
                Stateful {
                    input: ::core::clone::Clone::clone(&self.input),
                    state: ::core::clone::Clone::clone(&self.state),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::marker::Copy, S: ::core::marker::Copy> ::core::marker::Copy
        for Stateful<I, S> {}
        #[automatically_derived]
        impl<I: ::core::fmt::Debug, S: ::core::fmt::Debug> ::core::fmt::Debug
        for Stateful<I, S> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Stateful",
                    "input",
                    &self.input,
                    "state",
                    &&self.state,
                )
            }
        }
        #[automatically_derived]
        impl<
            I: ::core::default::Default,
            S: ::core::default::Default,
        > ::core::default::Default for Stateful<I, S> {
            #[inline]
            fn default() -> Stateful<I, S> {
                Stateful {
                    input: ::core::default::Default::default(),
                    state: ::core::default::Default::default(),
                }
            }
        }
        #[automatically_derived]
        impl<I: ::core::cmp::Eq, S: ::core::cmp::Eq> ::core::cmp::Eq for Stateful<I, S> {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<I>;
                let _: ::core::cmp::AssertParamIsEq<S>;
            }
        }
        #[automatically_derived]
        impl<I, S> ::core::marker::StructuralPartialEq for Stateful<I, S> {}
        #[automatically_derived]
        impl<I: ::core::cmp::PartialEq, S: ::core::cmp::PartialEq> ::core::cmp::PartialEq
        for Stateful<I, S> {
            #[inline]
            fn eq(&self, other: &Stateful<I, S>) -> bool {
                self.input == other.input && self.state == other.state
            }
        }
        impl<I, S> AsRef<I> for Stateful<I, S> {
            #[inline(always)]
            fn as_ref(&self) -> &I {
                &self.input
            }
        }
        impl<I, S> core::ops::Deref for Stateful<I, S> {
            type Target = I;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                self.as_ref()
            }
        }
        impl<I: core::fmt::Display, S> core::fmt::Display for Stateful<I, S> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.fmt(f)
            }
        }
        impl<I, S> SliceLen for Stateful<I, S>
        where
            I: SliceLen,
        {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.input.slice_len()
            }
        }
        impl<I: Stream, S: core::fmt::Debug> Stream for Stateful<I, S> {
            type Token = <I as Stream>::Token;
            type Slice = <I as Stream>::Slice;
            type IterOffsets = <I as Stream>::IterOffsets;
            type Checkpoint = Checkpoint<I::Checkpoint, Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.input.iter_offsets()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.input.eof_offset()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                self.input.next_token()
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                self.input.peek_token()
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.input.offset_for(predicate)
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                self.input.offset_at(tokens)
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                self.input.next_slice(offset)
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                unsafe { self.input.next_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                self.input.peek_slice(offset)
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                unsafe { self.input.peek_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(self.input.checkpoint())
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                self.input.reset(&checkpoint.inner);
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.trace(f)
            }
        }
        impl<I, S> Location for Stateful<I, S>
        where
            I: Location,
        {
            #[inline(always)]
            fn previous_token_end(&self) -> usize {
                self.input.previous_token_end()
            }
            #[inline(always)]
            fn current_token_start(&self) -> usize {
                self.input.current_token_start()
            }
        }
        impl<I, S> StreamIsPartial for Stateful<I, S>
        where
            I: StreamIsPartial,
        {
            type PartialState = I::PartialState;
            #[inline]
            fn complete(&mut self) -> Self::PartialState {
                self.input.complete()
            }
            #[inline]
            fn restore_partial(&mut self, state: Self::PartialState) {
                self.input.restore_partial(state);
            }
            #[inline(always)]
            fn is_partial_supported() -> bool {
                I::is_partial_supported()
            }
            #[inline(always)]
            fn is_partial(&self) -> bool {
                self.input.is_partial()
            }
        }
        impl<I, S> Offset for Stateful<I, S>
        where
            I: Stream,
            S: Clone + core::fmt::Debug,
        {
            #[inline(always)]
            fn offset_from(&self, start: &Self) -> usize {
                self.offset_from(&start.checkpoint())
            }
        }
        impl<I, S> Offset<<Stateful<I, S> as Stream>::Checkpoint> for Stateful<I, S>
        where
            I: Stream,
            S: core::fmt::Debug,
        {
            #[inline(always)]
            fn offset_from(
                &self,
                other: &<Stateful<I, S> as Stream>::Checkpoint,
            ) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl<I, S> AsBytes for Stateful<I, S>
        where
            I: AsBytes,
        {
            #[inline(always)]
            fn as_bytes(&self) -> &[u8] {
                self.input.as_bytes()
            }
        }
        impl<I, S> AsBStr for Stateful<I, S>
        where
            I: AsBStr,
        {
            #[inline(always)]
            fn as_bstr(&self) -> &[u8] {
                self.input.as_bstr()
            }
        }
        impl<I, S, U> Compare<U> for Stateful<I, S>
        where
            I: Compare<U>,
        {
            #[inline(always)]
            fn compare(&self, other: U) -> CompareResult {
                self.input.compare(other)
            }
        }
        impl<I, S, T> FindSlice<T> for Stateful<I, S>
        where
            I: FindSlice<T>,
        {
            #[inline(always)]
            fn find_slice(&self, substr: T) -> Option<core::ops::Range<usize>> {
                self.input.find_slice(substr)
            }
        }
        impl<I, S> UpdateSlice for Stateful<I, S>
        where
            I: UpdateSlice,
            S: Clone + core::fmt::Debug,
        {
            #[inline(always)]
            fn update_slice(mut self, inner: Self::Slice) -> Self {
                self.input = I::update_slice(self.input, inner);
                self
            }
        }
    }
    mod token {
        use crate::stream::Checkpoint;
        use crate::stream::Compare;
        use crate::stream::CompareResult;
        use crate::stream::Location;
        use crate::stream::Needed;
        use crate::stream::Offset;
        use crate::stream::SliceLen;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::stream::UpdateSlice;
        use core::iter::Enumerate;
        use core::slice::Iter;
        /// Specialized input for parsing lexed tokens
        ///
        /// Helpful impls
        /// - Any `PartialEq` type (e.g. a `TokenKind` or `&str`) can be used with
        ///   [`literal`][crate::token::literal]
        /// - A `PartialEq` for `&str` allows for using `&str` as a parser for tokens
        /// - [`ContainsToken`][crate::stream::ContainsToken] for `T` to for parsing with token sets
        /// - [`Location`] for `T` to extract spans from tokens
        ///
        /// See also [Lexing and Parsing][crate::_topic::lexing].
        pub struct TokenSlice<'t, T> {
            initial: &'t [T],
            input: &'t [T],
        }
        #[automatically_derived]
        impl<'t, T: ::core::marker::Copy> ::core::marker::Copy for TokenSlice<'t, T> {}
        #[automatically_derived]
        impl<'t, T: ::core::clone::Clone> ::core::clone::Clone for TokenSlice<'t, T> {
            #[inline]
            fn clone(&self) -> TokenSlice<'t, T> {
                TokenSlice {
                    initial: ::core::clone::Clone::clone(&self.initial),
                    input: ::core::clone::Clone::clone(&self.input),
                }
            }
        }
        #[automatically_derived]
        impl<'t, T> ::core::marker::StructuralPartialEq for TokenSlice<'t, T> {}
        #[automatically_derived]
        impl<'t, T: ::core::cmp::PartialEq> ::core::cmp::PartialEq
        for TokenSlice<'t, T> {
            #[inline]
            fn eq(&self, other: &TokenSlice<'t, T>) -> bool {
                self.initial == other.initial && self.input == other.input
            }
        }
        #[automatically_derived]
        impl<'t, T: ::core::cmp::Eq> ::core::cmp::Eq for TokenSlice<'t, T> {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<&'t [T]>;
                let _: ::core::cmp::AssertParamIsEq<&'t [T]>;
            }
        }
        impl<'t, T> TokenSlice<'t, T>
        where
            T: core::fmt::Debug + Clone,
        {
            /// Make a stream to parse tokens
            #[inline]
            pub fn new(input: &'t [T]) -> Self {
                Self { initial: input, input }
            }
            /// Reset the stream to the start
            ///
            /// This is useful for formats that encode a graph with addresses relative to the start of the
            /// input.
            #[doc(alias = "fseek")]
            #[inline]
            pub fn reset_to_start(&mut self) {
                let start = self.initial.checkpoint();
                self.input.reset(&start);
            }
            /// Iterate over consumed tokens starting with the last emitted
            ///
            /// This is intended to help build up appropriate context when reporting errors.
            #[inline]
            pub fn previous_tokens(&self) -> impl Iterator<Item = &'t T> {
                let offset = self.input.offset_from(&self.initial);
                self.initial[0..offset].iter().rev()
            }
        }
        /// Track locations by implementing [`Location`] on the Token.
        impl<T> TokenSlice<'_, T>
        where
            T: Location,
        {
            #[inline(always)]
            fn previous_token_end(&self) -> Option<usize> {
                let index = self.input.offset_from(&self.initial);
                index.checked_sub(1).map(|i| self.initial[i].previous_token_end())
            }
            #[inline(always)]
            fn current_token_start(&self) -> Option<usize> {
                self.input.first().map(|t| t.current_token_start())
            }
        }
        impl<T> Default for TokenSlice<'_, T>
        where
            T: core::fmt::Debug + Clone,
        {
            fn default() -> Self {
                Self::new(&[])
            }
        }
        impl<T> core::ops::Deref for TokenSlice<'_, T> {
            type Target = [T];
            fn deref(&self) -> &Self::Target {
                self.input
            }
        }
        impl<T: core::fmt::Debug> core::fmt::Debug for TokenSlice<'_, T> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.fmt(f)
            }
        }
        impl<T> SliceLen for TokenSlice<'_, T> {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.input.slice_len()
            }
        }
        impl<'t, T> Stream for TokenSlice<'t, T>
        where
            T: core::fmt::Debug + Clone,
        {
            type Token = &'t T;
            type Slice = &'t [T];
            type IterOffsets = Enumerate<Iter<'t, T>>;
            type Checkpoint = Checkpoint<&'t [T], Self>;
            #[inline(always)]
            fn iter_offsets(&self) -> Self::IterOffsets {
                self.input.iter().enumerate()
            }
            #[inline(always)]
            fn eof_offset(&self) -> usize {
                self.input.eof_offset()
            }
            #[inline(always)]
            fn next_token(&mut self) -> Option<Self::Token> {
                let (token, next) = self.input.split_first()?;
                self.input = next;
                Some(token)
            }
            #[inline(always)]
            fn peek_token(&self) -> Option<Self::Token> {
                self.input.first()
            }
            #[inline(always)]
            fn offset_for<P>(&self, predicate: P) -> Option<usize>
            where
                P: Fn(Self::Token) -> bool,
            {
                self.input.iter().position(predicate)
            }
            #[inline(always)]
            fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                self.input.offset_at(tokens)
            }
            #[inline(always)]
            fn next_slice(&mut self, offset: usize) -> Self::Slice {
                self.input.next_slice(offset)
            }
            #[inline(always)]
            unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
                unsafe { self.input.next_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn peek_slice(&self, offset: usize) -> Self::Slice {
                self.input.peek_slice(offset)
            }
            #[inline(always)]
            unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
                unsafe { self.input.peek_slice_unchecked(offset) }
            }
            #[inline(always)]
            fn checkpoint(&self) -> Self::Checkpoint {
                Checkpoint::<_, Self>::new(self.input)
            }
            #[inline(always)]
            fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                self.input = checkpoint.inner;
            }
            fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.input.trace(f)
            }
        }
        impl<T> Location for TokenSlice<'_, T>
        where
            T: Location,
        {
            #[inline(always)]
            fn previous_token_end(&self) -> usize {
                self.previous_token_end()
                    .or_else(|| self.current_token_start())
                    .unwrap_or(0)
            }
            #[inline(always)]
            fn current_token_start(&self) -> usize {
                self.current_token_start()
                    .or_else(|| self.previous_token_end())
                    .unwrap_or(0)
            }
        }
        impl<'t, T> StreamIsPartial for TokenSlice<'t, T>
        where
            T: core::fmt::Debug + Clone,
        {
            type PartialState = <&'t [T] as StreamIsPartial>::PartialState;
            #[inline]
            fn complete(&mut self) -> Self::PartialState {
                #![allow(clippy::semicolon_if_nothing_returned)]
                self.input.complete()
            }
            #[inline]
            fn restore_partial(&mut self, state: Self::PartialState) {
                self.input.restore_partial(state);
            }
            #[inline(always)]
            fn is_partial_supported() -> bool {
                <&[T] as StreamIsPartial>::is_partial_supported()
            }
            #[inline(always)]
            fn is_partial(&self) -> bool {
                self.input.is_partial()
            }
        }
        impl<T> Offset for TokenSlice<'_, T>
        where
            T: core::fmt::Debug + Clone,
        {
            #[inline(always)]
            fn offset_from(&self, other: &Self) -> usize {
                self.offset_from(&other.checkpoint())
            }
        }
        impl<T> Offset<<TokenSlice<'_, T> as Stream>::Checkpoint> for TokenSlice<'_, T>
        where
            T: core::fmt::Debug + Clone,
        {
            #[inline(always)]
            fn offset_from(
                &self,
                other: &<TokenSlice<'_, T> as Stream>::Checkpoint,
            ) -> usize {
                self.checkpoint().offset_from(other)
            }
        }
        impl<T, O> Compare<O> for TokenSlice<'_, T>
        where
            T: PartialEq<O> + Eq,
        {
            #[inline]
            fn compare(&self, t: O) -> CompareResult {
                if let Some(token) = self.first() {
                    if *token == t { CompareResult::Ok(1) } else { CompareResult::Error }
                } else {
                    CompareResult::Incomplete
                }
            }
        }
        impl<T> UpdateSlice for TokenSlice<'_, T>
        where
            T: core::fmt::Debug + Clone,
        {
            #[inline(always)]
            fn update_slice(mut self, inner: Self::Slice) -> Self {
                self.input = <&[T] as UpdateSlice>::update_slice(self.input, inner);
                self
            }
        }
    }
    pub use bstr::BStr;
    pub use bytes::Bytes;
    pub use locating::LocatingSlice;
    pub use partial::Partial;
    pub use range::Range;
    pub use stateful::Stateful;
    pub use token::TokenSlice;
    /// UTF-8 Stream
    pub type Str<'i> = &'i str;
    /// Abstract method to calculate the input length
    pub trait SliceLen {
        /// Calculates the input length, as indicated by its name,
        /// and the name of the trait itself
        fn slice_len(&self) -> usize;
    }
    impl<T> SliceLen for &[T] {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.len()
        }
    }
    impl<T, const LEN: usize> SliceLen for [T; LEN] {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.len()
        }
    }
    impl<T, const LEN: usize> SliceLen for &[T; LEN] {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.len()
        }
    }
    impl SliceLen for &str {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.len()
        }
    }
    impl SliceLen for u8 {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            1
        }
    }
    impl SliceLen for char {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.len_utf8()
        }
    }
    impl<I> SliceLen for (I, usize, usize)
    where
        I: SliceLen,
    {
        #[inline(always)]
        fn slice_len(&self) -> usize {
            self.0.slice_len() * 8 + self.2 - self.1
        }
    }
    /// Core definition for parser input state
    pub trait Stream: Offset<<Self as Stream>::Checkpoint> + core::fmt::Debug {
        /// The smallest unit being parsed
        ///
        /// Example: `u8` for `&[u8]` or `char` for `&str`
        type Token: core::fmt::Debug;
        /// Sequence of `Token`s
        ///
        /// Example: `&[u8]` for `LocatingSlice<&[u8]>` or `&str` for `LocatingSlice<&str>`
        type Slice: core::fmt::Debug;
        /// Iterate with the offset from the current location
        type IterOffsets: Iterator<Item = (usize, Self::Token)>;
        /// A parse location within the stream
        type Checkpoint: Offset + Clone + core::fmt::Debug;
        /// Iterate with the offset from the current location
        fn iter_offsets(&self) -> Self::IterOffsets;
        /// Returns the offset to the end of the input
        fn eof_offset(&self) -> usize;
        /// Split off the next token from the input
        fn next_token(&mut self) -> Option<Self::Token>;
        /// Split off the next token from the input
        fn peek_token(&self) -> Option<Self::Token>;
        /// Finds the offset of the next matching token
        fn offset_for<P>(&self, predicate: P) -> Option<usize>
        where
            P: Fn(Self::Token) -> bool;
        /// Get the offset for the number of `tokens` into the stream
        ///
        /// This means "0 tokens" will return `0` offset
        fn offset_at(&self, tokens: usize) -> Result<usize, Needed>;
        /// Split off a slice of tokens from the input
        ///
        /// <div class="warning">
        ///
        /// **Note:** For inputs with variable width tokens, like `&str`'s `char`, `offset` might not correspond
        /// with the number of tokens. To get a valid offset, use:
        /// - [`Stream::eof_offset`]
        /// - [`Stream::iter_offsets`]
        /// - [`Stream::offset_for`]
        /// - [`Stream::offset_at`]
        ///
        /// </div>
        ///
        /// # Panic
        ///
        /// This will panic if
        ///
        /// * Indexes must be within bounds of the original input;
        /// * Indexes must uphold invariants of the stream, like for `str` they must lie on UTF-8
        ///   sequence boundaries.
        ///
        fn next_slice(&mut self, offset: usize) -> Self::Slice;
        /// Split off a slice of tokens from the input
        ///
        /// <div class="warning">
        ///
        /// **Note:** For inputs with variable width tokens, like `&str`'s `char`, `offset` might not correspond
        /// with the number of tokens. To get a valid offset, use:
        /// - [`Stream::eof_offset`]
        /// - [`Stream::iter_offsets`]
        /// - [`Stream::offset_for`]
        /// - [`Stream::offset_at`]
        ///
        /// </div>
        ///
        /// # Safety
        ///
        /// Callers of this function are responsible that these preconditions are satisfied:
        ///
        /// * Indexes must be within bounds of the original input;
        /// * Indexes must uphold invariants of the stream, like for `str` they must lie on UTF-8
        ///   sequence boundaries.
        ///
        unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
            self.next_slice(offset)
        }
        /// Split off a slice of tokens from the input
        fn peek_slice(&self, offset: usize) -> Self::Slice;
        /// Split off a slice of tokens from the input
        ///
        /// # Safety
        ///
        /// Callers of this function are responsible that these preconditions are satisfied:
        ///
        /// * Indexes must be within bounds of the original input;
        /// * Indexes must uphold invariants of the stream, like for `str` they must lie on UTF-8
        ///   sequence boundaries.
        unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
            self.peek_slice(offset)
        }
        /// Advance to the end of the stream
        #[inline(always)]
        fn finish(&mut self) -> Self::Slice {
            self.next_slice(self.eof_offset())
        }
        /// Advance to the end of the stream
        #[inline(always)]
        fn peek_finish(&self) -> Self::Slice
        where
            Self: Clone,
        {
            self.peek_slice(self.eof_offset())
        }
        /// Save the current parse location within the stream
        fn checkpoint(&self) -> Self::Checkpoint;
        /// Revert the stream to a prior [`Self::Checkpoint`]
        ///
        /// # Panic
        ///
        /// May panic if an invalid [`Self::Checkpoint`] is provided
        fn reset(&mut self, checkpoint: &Self::Checkpoint);
        /// Write out a single-line summary of the current parse location
        fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result;
    }
    /// Contains information on needed data if a parser returned `Incomplete`
    ///
    /// <div class="warning">
    ///
    /// **Note:** This is only possible for `Stream` that are [partial][`crate::stream::StreamIsPartial`],
    /// like [`Partial`].
    ///
    /// </div>
    pub enum Needed {
        /// Needs more data, but we do not know how much
        Unknown,
        /// Contains a lower bound on the buffer offset needed to finish parsing
        ///
        /// For byte/`&str` streams, this translates to bytes
        Size(NonZeroUsize),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Needed {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Needed::Unknown => ::core::fmt::Formatter::write_str(f, "Unknown"),
                Needed::Size(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Size",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Needed {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Needed {
        #[inline]
        fn eq(&self, other: &Needed) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (Needed::Size(__self_0), Needed::Size(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => true,
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Needed {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<NonZeroUsize>;
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Needed {}
    #[automatically_derived]
    impl ::core::clone::Clone for Needed {
        #[inline]
        fn clone(&self) -> Needed {
            let _: ::core::clone::AssertParamIsClone<NonZeroUsize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Needed {}
    impl Needed {
        /// Creates `Needed` instance, returns `Needed::Unknown` if the argument is zero
        pub fn new(s: usize) -> Self {
            match NonZeroUsize::new(s) {
                Some(sz) => Needed::Size(sz),
                None => Needed::Unknown,
            }
        }
        /// Indicates if we know how many bytes we need
        pub fn is_known(&self) -> bool {
            *self != Needed::Unknown
        }
        /// Maps a `Needed` to `Needed` by applying a function to a contained `Size` value.
        #[inline]
        pub fn map<F: Fn(NonZeroUsize) -> usize>(self, f: F) -> Needed {
            match self {
                Needed::Unknown => Needed::Unknown,
                Needed::Size(n) => Needed::new(f(n)),
            }
        }
    }
    impl<'i, T> Stream for &'i [T]
    where
        T: Clone + core::fmt::Debug,
    {
        type Token = T;
        type Slice = &'i [T];
        type IterOffsets = Enumerate<Cloned<Iter<'i, T>>>;
        type Checkpoint = Checkpoint<Self, Self>;
        #[inline(always)]
        fn iter_offsets(&self) -> Self::IterOffsets {
            self.iter().cloned().enumerate()
        }
        #[inline(always)]
        fn eof_offset(&self) -> usize {
            self.len()
        }
        #[inline(always)]
        fn next_token(&mut self) -> Option<Self::Token> {
            let (token, next) = self.split_first()?;
            *self = next;
            Some(token.clone())
        }
        #[inline(always)]
        fn peek_token(&self) -> Option<Self::Token> {
            if self.is_empty() { None } else { Some(self[0].clone()) }
        }
        #[inline(always)]
        fn offset_for<P>(&self, predicate: P) -> Option<usize>
        where
            P: Fn(Self::Token) -> bool,
        {
            self.iter().position(|b| predicate(b.clone()))
        }
        #[inline(always)]
        fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
            if let Some(needed) = tokens
                .checked_sub(self.len())
                .and_then(NonZeroUsize::new)
            {
                Err(Needed::Size(needed))
            } else {
                Ok(tokens)
            }
        }
        #[inline(always)]
        fn next_slice(&mut self, offset: usize) -> Self::Slice {
            let (slice, next) = self.split_at(offset);
            *self = next;
            slice
        }
        #[inline(always)]
        unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
            self.peek_slice(offset);
            let slice = unsafe { self.get_unchecked(..offset) };
            let next = unsafe { self.get_unchecked(offset..) };
            *self = next;
            slice
        }
        #[inline(always)]
        fn peek_slice(&self, offset: usize) -> Self::Slice {
            &self[..offset]
        }
        #[inline(always)]
        unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
            self.peek_slice(offset);
            let slice = unsafe { self.get_unchecked(..offset) };
            slice
        }
        #[inline(always)]
        fn checkpoint(&self) -> Self::Checkpoint {
            Checkpoint::<_, Self>::new(*self)
        }
        #[inline(always)]
        fn reset(&mut self, checkpoint: &Self::Checkpoint) {
            *self = checkpoint.inner;
        }
        fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!("{0:?}", self))
        }
    }
    impl<'i> Stream for &'i str {
        type Token = char;
        type Slice = &'i str;
        type IterOffsets = CharIndices<'i>;
        type Checkpoint = Checkpoint<Self, Self>;
        #[inline(always)]
        fn iter_offsets(&self) -> Self::IterOffsets {
            self.char_indices()
        }
        #[inline(always)]
        fn eof_offset(&self) -> usize {
            self.len()
        }
        #[inline(always)]
        fn next_token(&mut self) -> Option<Self::Token> {
            let mut iter = self.chars();
            let c = iter.next()?;
            *self = iter.as_str();
            Some(c)
        }
        #[inline(always)]
        fn peek_token(&self) -> Option<Self::Token> {
            self.chars().next()
        }
        #[inline(always)]
        fn offset_for<P>(&self, predicate: P) -> Option<usize>
        where
            P: Fn(Self::Token) -> bool,
        {
            for (o, c) in self.iter_offsets() {
                if predicate(c) {
                    return Some(o);
                }
            }
            None
        }
        #[inline]
        fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
            let mut cnt = 0;
            for (offset, _) in self.iter_offsets() {
                if cnt == tokens {
                    return Ok(offset);
                }
                cnt += 1;
            }
            if cnt == tokens { Ok(self.eof_offset()) } else { Err(Needed::Unknown) }
        }
        #[inline(always)]
        fn next_slice(&mut self, offset: usize) -> Self::Slice {
            let (slice, next) = self.split_at(offset);
            *self = next;
            slice
        }
        #[inline(always)]
        unsafe fn next_slice_unchecked(&mut self, offset: usize) -> Self::Slice {
            self.peek_slice(offset);
            let slice = unsafe { self.get_unchecked(..offset) };
            let next = unsafe { self.get_unchecked(offset..) };
            *self = next;
            slice
        }
        #[inline(always)]
        fn peek_slice(&self, offset: usize) -> Self::Slice {
            &self[..offset]
        }
        #[inline(always)]
        unsafe fn peek_slice_unchecked(&self, offset: usize) -> Self::Slice {
            self.peek_slice(offset);
            let slice = unsafe { self.get_unchecked(..offset) };
            slice
        }
        #[inline(always)]
        fn checkpoint(&self) -> Self::Checkpoint {
            Checkpoint::<_, Self>::new(*self)
        }
        #[inline(always)]
        fn reset(&mut self, checkpoint: &Self::Checkpoint) {
            *self = checkpoint.inner;
        }
        fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!("{0:#?}", self))
        }
    }
    /// Current parse locations offset
    ///
    /// See [`LocatingSlice`] for adding location tracking to your [`Stream`]
    pub trait Location {
        /// Previous token's end offset
        fn previous_token_end(&self) -> usize;
        /// Current token's start offset
        fn current_token_start(&self) -> usize;
    }
    /// Marks the input as being the complete buffer or a partial buffer for streaming input
    ///
    /// See [`Partial`] for marking a presumed complete buffer type as a streaming buffer.
    pub trait StreamIsPartial: Sized {
        /// Whether the stream is currently partial or complete
        type PartialState;
        /// Mark the stream is complete
        #[must_use]
        fn complete(&mut self) -> Self::PartialState;
        /// Restore the stream back to its previous state
        fn restore_partial(&mut self, state: Self::PartialState);
        /// Report whether the [`Stream`] is can ever be incomplete
        fn is_partial_supported() -> bool;
        /// Report whether the [`Stream`] is currently incomplete
        #[inline(always)]
        fn is_partial(&self) -> bool {
            Self::is_partial_supported()
        }
    }
    impl<T> StreamIsPartial for &[T] {
        type PartialState = ();
        #[inline]
        fn complete(&mut self) -> Self::PartialState {}
        #[inline]
        fn restore_partial(&mut self, _state: Self::PartialState) {}
        #[inline(always)]
        fn is_partial_supported() -> bool {
            false
        }
    }
    impl StreamIsPartial for &str {
        type PartialState = ();
        #[inline]
        fn complete(&mut self) -> Self::PartialState {}
        #[inline]
        fn restore_partial(&mut self, _state: Self::PartialState) {}
        #[inline(always)]
        fn is_partial_supported() -> bool {
            false
        }
    }
    /// Useful functions to calculate the offset between slices and show a hexdump of a slice
    pub trait Offset<Start = Self> {
        /// Offset between the first byte of `start` and the first byte of `self`a
        ///
        /// <div class="warning">
        ///
        /// **Note:** This is an offset, not an index, and may point to the end of input
        /// (`start.len()`) when `self` is exhausted.
        ///
        /// </div>
        fn offset_from(&self, start: &Start) -> usize;
    }
    impl<T> Offset for &[T] {
        #[inline]
        fn offset_from(&self, start: &Self) -> usize {
            let fst = (*start).as_ptr();
            let snd = (*self).as_ptr();
            if true {
                if !(fst <= snd) {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "`Offset::offset_from({0:?}, {1:?})` only accepts slices of `self`",
                                snd,
                                fst,
                            ),
                        );
                    }
                }
            }
            (snd as usize - fst as usize) / core::mem::size_of::<T>()
        }
    }
    impl<'a, T> Offset<<&'a [T] as Stream>::Checkpoint> for &'a [T]
    where
        T: Clone + core::fmt::Debug,
    {
        #[inline(always)]
        fn offset_from(&self, other: &<&'a [T] as Stream>::Checkpoint) -> usize {
            self.checkpoint().offset_from(other)
        }
    }
    impl Offset for &str {
        #[inline(always)]
        fn offset_from(&self, start: &Self) -> usize {
            self.as_bytes().offset_from(&start.as_bytes())
        }
    }
    impl<'a> Offset<<&'a str as Stream>::Checkpoint> for &'a str {
        #[inline(always)]
        fn offset_from(&self, other: &<&'a str as Stream>::Checkpoint) -> usize {
            self.checkpoint().offset_from(other)
        }
    }
    impl<I, S> Offset for Checkpoint<I, S>
    where
        I: Offset,
    {
        #[inline(always)]
        fn offset_from(&self, start: &Self) -> usize {
            self.inner.offset_from(&start.inner)
        }
    }
    /// Helper trait for types that can be viewed as a byte slice
    pub trait AsBytes {
        /// Casts the input type to a byte slice
        fn as_bytes(&self) -> &[u8];
    }
    impl AsBytes for &[u8] {
        #[inline(always)]
        fn as_bytes(&self) -> &[u8] {
            self
        }
    }
    /// Helper trait for types that can be viewed as a byte slice
    pub trait AsBStr {
        /// Casts the input type to a byte slice
        fn as_bstr(&self) -> &[u8];
    }
    impl AsBStr for &[u8] {
        #[inline(always)]
        fn as_bstr(&self) -> &[u8] {
            self
        }
    }
    impl AsBStr for &str {
        #[inline(always)]
        fn as_bstr(&self) -> &[u8] {
            (*self).as_bytes()
        }
    }
    /// Result of [`Compare::compare`]
    pub enum CompareResult {
        /// Comparison was successful
        ///
        /// `usize` is the end of the successful match within the buffer.
        /// This is most relevant for caseless UTF-8 where `Compare::compare`'s parameter might be a different
        /// length than the match within the buffer.
        Ok(usize),
        /// We need more data to be sure
        Incomplete,
        /// Comparison failed
        Error,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CompareResult {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                CompareResult::Ok(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Ok", &__self_0)
                }
                CompareResult::Incomplete => {
                    ::core::fmt::Formatter::write_str(f, "Incomplete")
                }
                CompareResult::Error => ::core::fmt::Formatter::write_str(f, "Error"),
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for CompareResult {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<usize>;
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for CompareResult {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for CompareResult {
        #[inline]
        fn eq(&self, other: &CompareResult) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (CompareResult::Ok(__self_0), CompareResult::Ok(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    _ => true,
                }
        }
    }
    /// Abstracts comparison operations
    pub trait Compare<T> {
        /// Compares self to another value for equality
        fn compare(&self, t: T) -> CompareResult;
    }
    impl<'b> Compare<&'b [u8]> for &[u8] {
        #[inline]
        fn compare(&self, t: &'b [u8]) -> CompareResult {
            if t.iter().zip(*self).any(|(a, b)| a != b) {
                CompareResult::Error
            } else if self.len() < t.slice_len() {
                CompareResult::Incomplete
            } else {
                CompareResult::Ok(t.slice_len())
            }
        }
    }
    impl<const LEN: usize> Compare<[u8; LEN]> for &[u8] {
        #[inline(always)]
        fn compare(&self, t: [u8; LEN]) -> CompareResult {
            self.compare(&t[..])
        }
    }
    impl<'b, const LEN: usize> Compare<&'b [u8; LEN]> for &[u8] {
        #[inline(always)]
        fn compare(&self, t: &'b [u8; LEN]) -> CompareResult {
            self.compare(&t[..])
        }
    }
    impl<'b> Compare<&'b str> for &[u8] {
        #[inline(always)]
        fn compare(&self, t: &'b str) -> CompareResult {
            self.compare(t.as_bytes())
        }
    }
    impl Compare<u8> for &[u8] {
        #[inline]
        fn compare(&self, t: u8) -> CompareResult {
            match self.first().copied() {
                Some(c) if t == c => CompareResult::Ok(t.slice_len()),
                Some(_) => CompareResult::Error,
                None => CompareResult::Incomplete,
            }
        }
    }
    impl Compare<char> for &[u8] {
        #[inline(always)]
        fn compare(&self, t: char) -> CompareResult {
            self.compare(t.encode_utf8(&mut [0; 4]).as_bytes())
        }
    }
    impl<'b> Compare<&'b str> for &str {
        #[inline(always)]
        fn compare(&self, t: &'b str) -> CompareResult {
            self.as_bytes().compare(t.as_bytes())
        }
    }
    impl Compare<char> for &str {
        #[inline(always)]
        fn compare(&self, t: char) -> CompareResult {
            self.as_bytes().compare(t)
        }
    }
    /// Look for a slice in self
    pub trait FindSlice<T> {
        /// Returns the offset of the slice if it is found
        fn find_slice(&self, substr: T) -> Option<core::ops::Range<usize>>;
    }
    impl<'s> FindSlice<&'s [u8]> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: &'s [u8]) -> Option<core::ops::Range<usize>> {
            memmem(self, substr)
        }
    }
    impl<'s> FindSlice<(&'s [u8],)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (&'s [u8],)) -> Option<core::ops::Range<usize>> {
            memmem(self, substr.0)
        }
    }
    impl<'s> FindSlice<(&'s [u8], &'s [u8])> for &[u8] {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s [u8], &'s [u8]),
        ) -> Option<core::ops::Range<usize>> {
            memmem2(self, substr)
        }
    }
    impl<'s> FindSlice<(&'s [u8], &'s [u8], &'s [u8])> for &[u8] {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s [u8], &'s [u8], &'s [u8]),
        ) -> Option<core::ops::Range<usize>> {
            memmem3(self, substr)
        }
    }
    impl FindSlice<char> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: char) -> Option<core::ops::Range<usize>> {
            let mut b = [0; 4];
            let substr = substr.encode_utf8(&mut b);
            self.find_slice(&*substr)
        }
    }
    impl FindSlice<(char,)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (char,)) -> Option<core::ops::Range<usize>> {
            let mut b = [0; 4];
            let substr0 = substr.0.encode_utf8(&mut b);
            self.find_slice((&*substr0,))
        }
    }
    impl FindSlice<(char, char)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (char, char)) -> Option<core::ops::Range<usize>> {
            let mut b = [0; 4];
            let substr0 = substr.0.encode_utf8(&mut b);
            let mut b = [0; 4];
            let substr1 = substr.1.encode_utf8(&mut b);
            self.find_slice((&*substr0, &*substr1))
        }
    }
    impl FindSlice<(char, char, char)> for &[u8] {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (char, char, char),
        ) -> Option<core::ops::Range<usize>> {
            let mut b = [0; 4];
            let substr0 = substr.0.encode_utf8(&mut b);
            let mut b = [0; 4];
            let substr1 = substr.1.encode_utf8(&mut b);
            let mut b = [0; 4];
            let substr2 = substr.2.encode_utf8(&mut b);
            self.find_slice((&*substr0, &*substr1, &*substr2))
        }
    }
    impl FindSlice<u8> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: u8) -> Option<core::ops::Range<usize>> {
            memchr(substr, self).map(|i| i..i + 1)
        }
    }
    impl FindSlice<(u8,)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (u8,)) -> Option<core::ops::Range<usize>> {
            memchr(substr.0, self).map(|i| i..i + 1)
        }
    }
    impl FindSlice<(u8, u8)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (u8, u8)) -> Option<core::ops::Range<usize>> {
            memchr2(substr, self).map(|i| i..i + 1)
        }
    }
    impl FindSlice<(u8, u8, u8)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (u8, u8, u8)) -> Option<core::ops::Range<usize>> {
            memchr3(substr, self).map(|i| i..i + 1)
        }
    }
    impl<'s> FindSlice<&'s str> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: &'s str) -> Option<core::ops::Range<usize>> {
            self.find_slice(substr.as_bytes())
        }
    }
    impl<'s> FindSlice<(&'s str,)> for &[u8] {
        #[inline(always)]
        fn find_slice(&self, substr: (&'s str,)) -> Option<core::ops::Range<usize>> {
            memmem(self, substr.0.as_bytes())
        }
    }
    impl<'s> FindSlice<(&'s str, &'s str)> for &[u8] {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s str, &'s str),
        ) -> Option<core::ops::Range<usize>> {
            memmem2(self, (substr.0.as_bytes(), substr.1.as_bytes()))
        }
    }
    impl<'s> FindSlice<(&'s str, &'s str, &'s str)> for &[u8] {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s str, &'s str, &'s str),
        ) -> Option<core::ops::Range<usize>> {
            memmem3(
                self,
                (substr.0.as_bytes(), substr.1.as_bytes(), substr.2.as_bytes()),
            )
        }
    }
    impl<'s> FindSlice<&'s str> for &str {
        #[inline(always)]
        fn find_slice(&self, substr: &'s str) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl<'s> FindSlice<(&'s str,)> for &str {
        #[inline(always)]
        fn find_slice(&self, substr: (&'s str,)) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl<'s> FindSlice<(&'s str, &'s str)> for &str {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s str, &'s str),
        ) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl<'s> FindSlice<(&'s str, &'s str, &'s str)> for &str {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (&'s str, &'s str, &'s str),
        ) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl FindSlice<char> for &str {
        #[inline(always)]
        fn find_slice(&self, substr: char) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl FindSlice<(char,)> for &str {
        #[inline(always)]
        fn find_slice(&self, substr: (char,)) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl FindSlice<(char, char)> for &str {
        #[inline(always)]
        fn find_slice(&self, substr: (char, char)) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    impl FindSlice<(char, char, char)> for &str {
        #[inline(always)]
        fn find_slice(
            &self,
            substr: (char, char, char),
        ) -> Option<core::ops::Range<usize>> {
            self.as_bytes().find_slice(substr)
        }
    }
    /// Used to integrate `str`'s `parse()` method
    pub trait ParseSlice<R> {
        /// Succeeds if `parse()` succeeded
        ///
        /// The byte slice implementation will first convert it to a `&str`, then apply the `parse()`
        /// function
        fn parse_slice(&self) -> Option<R>;
    }
    impl<R: FromStr> ParseSlice<R> for &[u8] {
        #[inline(always)]
        fn parse_slice(&self) -> Option<R> {
            from_utf8(self).ok().and_then(|s| s.parse().ok())
        }
    }
    impl<R: FromStr> ParseSlice<R> for &str {
        #[inline(always)]
        fn parse_slice(&self) -> Option<R> {
            self.parse().ok()
        }
    }
    /// Convert a `Stream` into an appropriate `Output` type
    pub trait UpdateSlice: Stream {
        /// Convert an `Output` type to be used as `Stream`
        fn update_slice(self, inner: Self::Slice) -> Self;
    }
    impl<T> UpdateSlice for &[T]
    where
        T: Clone + core::fmt::Debug,
    {
        #[inline(always)]
        fn update_slice(self, inner: Self::Slice) -> Self {
            inner
        }
    }
    impl UpdateSlice for &str {
        #[inline(always)]
        fn update_slice(self, inner: Self::Slice) -> Self {
            inner
        }
    }
    /// Ensure checkpoint details are kept private
    pub struct Checkpoint<T, S> {
        pub(crate) inner: T,
        stream: core::marker::PhantomData<S>,
    }
    impl<T, S> Checkpoint<T, S> {
        pub(crate) fn new(inner: T) -> Self {
            Self {
                inner,
                stream: Default::default(),
            }
        }
    }
    impl<T: Copy, S> Copy for Checkpoint<T, S> {}
    impl<T: Clone, S> Clone for Checkpoint<T, S> {
        #[inline(always)]
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                stream: Default::default(),
            }
        }
    }
    impl<T: PartialOrd, S> PartialOrd for Checkpoint<T, S> {
        #[inline(always)]
        fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
            self.inner.partial_cmp(&other.inner)
        }
    }
    impl<T: Ord, S> Ord for Checkpoint<T, S> {
        #[inline(always)]
        fn cmp(&self, other: &Self) -> core::cmp::Ordering {
            self.inner.cmp(&other.inner)
        }
    }
    impl<T: PartialEq, S> PartialEq for Checkpoint<T, S> {
        #[inline(always)]
        fn eq(&self, other: &Self) -> bool {
            self.inner.eq(&other.inner)
        }
    }
    impl<T: Eq, S> Eq for Checkpoint<T, S> {}
    impl<T: core::fmt::Debug, S> core::fmt::Debug for Checkpoint<T, S> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.inner.fmt(f)
        }
    }
    /// Abstracts something which can extend an `Extend`.
    ///
    /// Used to build modified input slices in [`escaped`][crate::ascii::escaped].
    pub trait Accumulate<T>: Sized {
        /// Create a new `Extend` of the correct type
        fn initial(capacity: Option<usize>) -> Self;
        /// Accumulate the input into an accumulator
        fn accumulate(&mut self, acc: T);
    }
    impl<T> Accumulate<T> for () {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {}
        #[inline(always)]
        fn accumulate(&mut self, _acc: T) {}
    }
    impl<T> Accumulate<T> for usize {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            0
        }
        #[inline(always)]
        fn accumulate(&mut self, _acc: T) {
            *self += 1;
        }
    }
    impl<T> Accumulate<T> for Vec<T> {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => Vec::with_capacity(clamp_capacity::<T>(capacity)),
                None => Vec::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: T) {
            self.push(acc);
        }
    }
    impl<'i, T: Clone> Accumulate<&'i [T]> for Vec<T> {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => Vec::with_capacity(clamp_capacity::<T>(capacity)),
                None => Vec::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: &'i [T]) {
            self.extend(acc.iter().cloned());
        }
    }
    impl Accumulate<char> for String {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => String::with_capacity(clamp_capacity::<char>(capacity)),
                None => String::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: char) {
            self.push(acc);
        }
    }
    impl<'i> Accumulate<&'i str> for String {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => String::with_capacity(clamp_capacity::<char>(capacity)),
                None => String::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: &'i str) {
            self.push_str(acc);
        }
    }
    impl<'i> Accumulate<Cow<'i, str>> for String {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => String::with_capacity(clamp_capacity::<char>(capacity)),
                None => String::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: Cow<'i, str>) {
            self.push_str(&acc);
        }
    }
    impl Accumulate<String> for String {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => String::with_capacity(clamp_capacity::<char>(capacity)),
                None => String::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: String) {
            self.push_str(&acc);
        }
    }
    impl Accumulate<char> for Cow<'_, str> {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            Cow::Borrowed("")
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: char) {
            self.to_mut().accumulate(acc);
        }
    }
    impl<'i> Accumulate<&'i str> for Cow<'i, str> {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            Cow::Borrowed("")
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: &'i str) {
            if self.as_ref().is_empty() {
                *self = Cow::Borrowed(acc);
            } else {
                self.to_mut().accumulate(acc);
            }
        }
    }
    impl<'i> Accumulate<Cow<'i, str>> for Cow<'i, str> {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            Cow::Borrowed("")
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: Cow<'i, str>) {
            if self.as_ref().is_empty() {
                *self = acc;
            } else {
                self.to_mut().accumulate(acc);
            }
        }
    }
    impl Accumulate<String> for Cow<'_, str> {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            Cow::Borrowed("")
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: String) {
            self.to_mut().accumulate(acc);
        }
    }
    impl<K, V> Accumulate<(K, V)> for BTreeMap<K, V>
    where
        K: core::cmp::Ord,
    {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            BTreeMap::new()
        }
        #[inline(always)]
        fn accumulate(&mut self, (key, value): (K, V)) {
            self.insert(key, value);
        }
    }
    impl<K, V, S> Accumulate<(K, V)> for HashMap<K, V, S>
    where
        K: core::cmp::Eq + core::hash::Hash,
        S: BuildHasher + Default,
    {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            let h = S::default();
            match capacity {
                Some(capacity) => {
                    HashMap::with_capacity_and_hasher(
                        clamp_capacity::<(K, V)>(capacity),
                        h,
                    )
                }
                None => HashMap::with_hasher(h),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, (key, value): (K, V)) {
            self.insert(key, value);
        }
    }
    impl<K> Accumulate<K> for BTreeSet<K>
    where
        K: core::cmp::Ord,
    {
        #[inline(always)]
        fn initial(_capacity: Option<usize>) -> Self {
            BTreeSet::new()
        }
        #[inline(always)]
        fn accumulate(&mut self, key: K) {
            self.insert(key);
        }
    }
    impl<K, S> Accumulate<K> for HashSet<K, S>
    where
        K: core::cmp::Eq + core::hash::Hash,
        S: BuildHasher + Default,
    {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            let h = S::default();
            match capacity {
                Some(capacity) => {
                    HashSet::with_capacity_and_hasher(clamp_capacity::<K>(capacity), h)
                }
                None => HashSet::with_hasher(h),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, key: K) {
            self.insert(key);
        }
    }
    impl<'i, T: Clone> Accumulate<&'i [T]> for VecDeque<T> {
        #[inline(always)]
        fn initial(capacity: Option<usize>) -> Self {
            match capacity {
                Some(capacity) => VecDeque::with_capacity(clamp_capacity::<T>(capacity)),
                None => VecDeque::new(),
            }
        }
        #[inline(always)]
        fn accumulate(&mut self, acc: &'i [T]) {
            self.extend(acc.iter().cloned());
        }
    }
    #[inline]
    pub(crate) fn clamp_capacity<T>(capacity: usize) -> usize {
        /// Don't pre-allocate more than 64KiB when calling `Vec::with_capacity`.
        ///
        /// Pre-allocating memory is a nice optimization but count fields can't
        /// always be trusted. We should clamp initial capacities to some reasonable
        /// amount. This reduces the risk of a bogus count value triggering a panic
        /// due to an OOM error.
        ///
        /// This does not affect correctness. `winnow` will always read the full number
        /// of elements regardless of the capacity cap.
        const MAX_INITIAL_CAPACITY_BYTES: usize = 65536;
        let max_initial_capacity = MAX_INITIAL_CAPACITY_BYTES
            / core::mem::size_of::<T>().max(1);
        capacity.min(max_initial_capacity)
    }
    /// Helper trait to convert numbers to usize.
    ///
    /// By default, usize implements `From<u8>` and `From<u16>` but not
    /// `From<u32>` and `From<u64>` because that would be invalid on some
    /// platforms. This trait implements the conversion for platforms
    /// with 32 and 64 bits pointer platforms
    pub trait ToUsize {
        /// converts self to usize
        fn to_usize(&self) -> usize;
    }
    impl ToUsize for u8 {
        #[inline(always)]
        fn to_usize(&self) -> usize {
            *self as usize
        }
    }
    impl ToUsize for u16 {
        #[inline(always)]
        fn to_usize(&self) -> usize {
            *self as usize
        }
    }
    impl ToUsize for usize {
        #[inline(always)]
        fn to_usize(&self) -> usize {
            *self
        }
    }
    impl ToUsize for u32 {
        #[inline(always)]
        fn to_usize(&self) -> usize {
            *self as usize
        }
    }
    impl ToUsize for u64 {
        #[inline(always)]
        fn to_usize(&self) -> usize {
            *self as usize
        }
    }
    /// Transforms a token into a char for basic string parsing
    #[allow(clippy::len_without_is_empty)]
    #[allow(clippy::wrong_self_convention)]
    pub trait AsChar {
        /// Makes a char from self
        ///
        /// # Example
        ///
        /// ```
        /// use winnow::prelude::*;
        ///
        /// assert_eq!('a'.as_char(), 'a');
        /// assert_eq!(u8::MAX.as_char(), std::char::from_u32(u8::MAX as u32).unwrap());
        /// ```
        fn as_char(self) -> char;
        /// Tests that self is an ASCII alphabetic character
        fn is_alpha(self) -> bool;
        /// Tests that self is an alphabetic character
        /// or a decimal digit
        fn is_alphanum(self) -> bool;
        /// Tests that self is a decimal digit
        fn is_dec_digit(self) -> bool;
        /// Tests that self is an hex digit
        fn is_hex_digit(self) -> bool;
        /// Tests that self is an octal digit
        fn is_oct_digit(self) -> bool;
        /// Gets the len in bytes for self
        fn len(self) -> usize;
        /// Tests that self is ASCII space or tab
        fn is_space(self) -> bool;
        /// Tests if byte is ASCII newline: \n
        fn is_newline(self) -> bool;
    }
    impl AsChar for u8 {
        #[inline(always)]
        fn as_char(self) -> char {
            self as char
        }
        #[inline]
        fn is_alpha(self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                0x41..=0x5A | 0x61..=0x7A => true,
                _ => false,
            }
        }
        #[inline]
        fn is_alphanum(self) -> bool {
            self.is_alpha() || self.is_dec_digit()
        }
        #[inline]
        fn is_dec_digit(self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                0x30..=0x39 => true,
                _ => false,
            }
        }
        #[inline]
        fn is_hex_digit(self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66 => true,
                _ => false,
            }
        }
        #[inline]
        fn is_oct_digit(self) -> bool {
            #[allow(non_exhaustive_omitted_patterns)]
            match self {
                0x30..=0x37 => true,
                _ => false,
            }
        }
        #[inline]
        fn len(self) -> usize {
            1
        }
        #[inline]
        fn is_space(self) -> bool {
            self == b' ' || self == b'\t'
        }
        #[inline]
        fn is_newline(self) -> bool {
            self == b'\n'
        }
    }
    impl AsChar for &u8 {
        #[inline(always)]
        fn as_char(self) -> char {
            (*self).as_char()
        }
        #[inline(always)]
        fn is_alpha(self) -> bool {
            (*self).is_alpha()
        }
        #[inline(always)]
        fn is_alphanum(self) -> bool {
            (*self).is_alphanum()
        }
        #[inline(always)]
        fn is_dec_digit(self) -> bool {
            (*self).is_dec_digit()
        }
        #[inline(always)]
        fn is_hex_digit(self) -> bool {
            (*self).is_hex_digit()
        }
        #[inline(always)]
        fn is_oct_digit(self) -> bool {
            (*self).is_oct_digit()
        }
        #[inline(always)]
        fn len(self) -> usize {
            (*self).len()
        }
        #[inline(always)]
        fn is_space(self) -> bool {
            (*self).is_space()
        }
        #[inline(always)]
        fn is_newline(self) -> bool {
            (*self).is_newline()
        }
    }
    impl AsChar for char {
        #[inline(always)]
        fn as_char(self) -> char {
            self
        }
        #[inline]
        fn is_alpha(self) -> bool {
            self.is_ascii_alphabetic()
        }
        #[inline]
        fn is_alphanum(self) -> bool {
            self.is_alpha() || self.is_dec_digit()
        }
        #[inline]
        fn is_dec_digit(self) -> bool {
            self.is_ascii_digit()
        }
        #[inline]
        fn is_hex_digit(self) -> bool {
            self.is_ascii_hexdigit()
        }
        #[inline]
        fn is_oct_digit(self) -> bool {
            self.is_digit(8)
        }
        #[inline]
        fn len(self) -> usize {
            self.len_utf8()
        }
        #[inline]
        fn is_space(self) -> bool {
            self == ' ' || self == '\t'
        }
        #[inline]
        fn is_newline(self) -> bool {
            self == '\n'
        }
    }
    impl AsChar for &char {
        #[inline(always)]
        fn as_char(self) -> char {
            (*self).as_char()
        }
        #[inline(always)]
        fn is_alpha(self) -> bool {
            (*self).is_alpha()
        }
        #[inline(always)]
        fn is_alphanum(self) -> bool {
            (*self).is_alphanum()
        }
        #[inline(always)]
        fn is_dec_digit(self) -> bool {
            (*self).is_dec_digit()
        }
        #[inline(always)]
        fn is_hex_digit(self) -> bool {
            (*self).is_hex_digit()
        }
        #[inline(always)]
        fn is_oct_digit(self) -> bool {
            (*self).is_oct_digit()
        }
        #[inline(always)]
        fn len(self) -> usize {
            (*self).len()
        }
        #[inline(always)]
        fn is_space(self) -> bool {
            (*self).is_space()
        }
        #[inline(always)]
        fn is_newline(self) -> bool {
            (*self).is_newline()
        }
    }
    /// Check if a token is in a set of possible tokens
    ///
    /// While this can be implemented manually, you can also build up sets using:
    /// - `b'c'` and `'c'`
    /// - `b""`
    /// - `|c| true`
    /// - `b'a'..=b'z'`, `'a'..='z'` (etc for each [range type][std::ops])
    /// - `(set1, set2, ...)`
    ///
    /// # Example
    ///
    /// For example, you could implement `hex_digit0` as:
    /// ```
    /// # #[cfg(feature = "parser")] {
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// # use winnow::token::take_while;
    /// fn hex_digit1<'s>(input: &mut &'s str) -> ModalResult<&'s str, ContextError> {
    ///     take_while(1.., ('a'..='f', 'A'..='F', '0'..='9')).parse_next(input)
    /// }
    ///
    /// assert_eq!(hex_digit1.parse_peek("21cZ"), Ok(("Z", "21c")));
    /// assert!(hex_digit1.parse_peek("H2").is_err());
    /// assert!(hex_digit1.parse_peek("").is_err());
    /// # }
    /// ```
    pub trait ContainsToken<T> {
        /// Returns true if self contains the token
        fn contains_token(&self, token: T) -> bool;
    }
    impl ContainsToken<u8> for u8 {
        #[inline(always)]
        fn contains_token(&self, token: u8) -> bool {
            *self == token
        }
    }
    impl ContainsToken<&u8> for u8 {
        #[inline(always)]
        fn contains_token(&self, token: &u8) -> bool {
            self.contains_token(*token)
        }
    }
    impl ContainsToken<char> for u8 {
        #[inline(always)]
        fn contains_token(&self, token: char) -> bool {
            self.as_char() == token
        }
    }
    impl ContainsToken<&char> for u8 {
        #[inline(always)]
        fn contains_token(&self, token: &char) -> bool {
            self.contains_token(*token)
        }
    }
    impl<C: AsChar> ContainsToken<C> for char {
        #[inline(always)]
        fn contains_token(&self, token: C) -> bool {
            *self == token.as_char()
        }
    }
    impl<C, F: Fn(C) -> bool> ContainsToken<C> for F {
        #[inline(always)]
        fn contains_token(&self, token: C) -> bool {
            self(token)
        }
    }
    impl<C1: AsChar, C2: AsChar + Clone> ContainsToken<C1> for core::ops::Range<C2> {
        #[inline(always)]
        fn contains_token(&self, token: C1) -> bool {
            let start = self.start.clone().as_char();
            let end = self.end.clone().as_char();
            (start..end).contains(&token.as_char())
        }
    }
    impl<C1: AsChar, C2: AsChar + Clone> ContainsToken<C1>
    for core::ops::RangeInclusive<C2> {
        #[inline(always)]
        fn contains_token(&self, token: C1) -> bool {
            let start = self.start().clone().as_char();
            let end = self.end().clone().as_char();
            (start..=end).contains(&token.as_char())
        }
    }
    impl<C1: AsChar, C2: AsChar + Clone> ContainsToken<C1> for core::ops::RangeFrom<C2> {
        #[inline(always)]
        fn contains_token(&self, token: C1) -> bool {
            let start = self.start.clone().as_char();
            (start..).contains(&token.as_char())
        }
    }
    impl<C1: AsChar, C2: AsChar + Clone> ContainsToken<C1> for core::ops::RangeTo<C2> {
        #[inline(always)]
        fn contains_token(&self, token: C1) -> bool {
            let end = self.end.clone().as_char();
            (..end).contains(&token.as_char())
        }
    }
    impl<C1: AsChar, C2: AsChar + Clone> ContainsToken<C1>
    for core::ops::RangeToInclusive<C2> {
        #[inline(always)]
        fn contains_token(&self, token: C1) -> bool {
            let end = self.end.clone().as_char();
            (..=end).contains(&token.as_char())
        }
    }
    impl<C1: AsChar> ContainsToken<C1> for core::ops::RangeFull {
        #[inline(always)]
        fn contains_token(&self, _token: C1) -> bool {
            true
        }
    }
    impl<C: AsChar> ContainsToken<C> for &'_ [u8] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.iter().any(|t| t.as_char() == token)
        }
    }
    impl<C: AsChar> ContainsToken<C> for &'_ [char] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.contains(&token)
        }
    }
    impl<const LEN: usize, C: AsChar> ContainsToken<C> for &'_ [u8; LEN] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.iter().any(|t| t.as_char() == token)
        }
    }
    impl<const LEN: usize, C: AsChar> ContainsToken<C> for &'_ [char; LEN] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.contains(&token)
        }
    }
    impl<const LEN: usize, C: AsChar> ContainsToken<C> for [u8; LEN] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.iter().any(|t| t.as_char() == token)
        }
    }
    impl<const LEN: usize, C: AsChar> ContainsToken<C> for [char; LEN] {
        #[inline]
        fn contains_token(&self, token: C) -> bool {
            let token = token.as_char();
            self.contains(&token)
        }
    }
    impl<T> ContainsToken<T> for () {
        #[inline(always)]
        fn contains_token(&self, _token: T) -> bool {
            false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1> ContainsToken<T> for (F1,)
    where
        T: Clone,
        F1: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1,) = *self;
            F1.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2> ContainsToken<T> for (F1, F2)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3> ContainsToken<T> for (F1, F2, F3)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4> ContainsToken<T> for (F1, F2, F3, F4)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3, ref F4) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5> ContainsToken<T> for (F1, F2, F3, F4, F5)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3, ref F4, ref F5) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5, F6> ContainsToken<T> for (F1, F2, F3, F4, F5, F6)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
        F6: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3, ref F4, ref F5, ref F6) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || F6.contains_token(token.clone())
                || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5, F6, F7> ContainsToken<T> for (F1, F2, F3, F4, F5, F6, F7)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
        F6: ContainsToken<T>,
        F7: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3, ref F4, ref F5, ref F6, ref F7) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || F6.contains_token(token.clone())
                || F7.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5, F6, F7, F8> ContainsToken<T>
    for (F1, F2, F3, F4, F5, F6, F7, F8)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
        F6: ContainsToken<T>,
        F7: ContainsToken<T>,
        F8: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (ref F1, ref F2, ref F3, ref F4, ref F5, ref F6, ref F7, ref F8) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || F6.contains_token(token.clone())
                || F7.contains_token(token.clone()) || F8.contains_token(token.clone())
                || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5, F6, F7, F8, F9> ContainsToken<T>
    for (F1, F2, F3, F4, F5, F6, F7, F8, F9)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
        F6: ContainsToken<T>,
        F7: ContainsToken<T>,
        F8: ContainsToken<T>,
        F9: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (
                ref F1,
                ref F2,
                ref F3,
                ref F4,
                ref F5,
                ref F6,
                ref F7,
                ref F8,
                ref F9,
            ) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || F6.contains_token(token.clone())
                || F7.contains_token(token.clone()) || F8.contains_token(token.clone())
                || F9.contains_token(token.clone()) || false
        }
    }
    #[allow(non_snake_case)]
    impl<T, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10> ContainsToken<T>
    for (F1, F2, F3, F4, F5, F6, F7, F8, F9, F10)
    where
        T: Clone,
        F1: ContainsToken<T>,
        F2: ContainsToken<T>,
        F3: ContainsToken<T>,
        F4: ContainsToken<T>,
        F5: ContainsToken<T>,
        F6: ContainsToken<T>,
        F7: ContainsToken<T>,
        F8: ContainsToken<T>,
        F9: ContainsToken<T>,
        F10: ContainsToken<T>,
    {
        #[inline]
        fn contains_token(&self, token: T) -> bool {
            let (
                ref F1,
                ref F2,
                ref F3,
                ref F4,
                ref F5,
                ref F6,
                ref F7,
                ref F8,
                ref F9,
                ref F10,
            ) = *self;
            F1.contains_token(token.clone()) || F2.contains_token(token.clone())
                || F3.contains_token(token.clone()) || F4.contains_token(token.clone())
                || F5.contains_token(token.clone()) || F6.contains_token(token.clone())
                || F7.contains_token(token.clone()) || F8.contains_token(token.clone())
                || F9.contains_token(token.clone()) || F10.contains_token(token.clone())
                || false
        }
    }
    #[inline(always)]
    fn memchr(token: u8, slice: &[u8]) -> Option<usize> {
        slice.iter().position(|t| *t == token)
    }
    #[inline(always)]
    fn memchr2(token: (u8, u8), slice: &[u8]) -> Option<usize> {
        slice.iter().position(|t| *t == token.0 || *t == token.1)
    }
    #[inline(always)]
    fn memchr3(token: (u8, u8, u8), slice: &[u8]) -> Option<usize> {
        slice.iter().position(|t| *t == token.0 || *t == token.1 || *t == token.2)
    }
    #[inline(always)]
    fn memmem(slice: &[u8], literal: &[u8]) -> Option<core::ops::Range<usize>> {
        match literal.len() {
            0 => Some(0..0),
            1 => memchr(literal[0], slice).map(|i| i..i + 1),
            _ => memmem_(slice, literal),
        }
    }
    #[inline(always)]
    fn memmem2(
        slice: &[u8],
        literal: (&[u8], &[u8]),
    ) -> Option<core::ops::Range<usize>> {
        match (literal.0.len(), literal.1.len()) {
            (0, _) | (_, 0) => Some(0..0),
            (1, 1) => memchr2((literal.0[0], literal.1[0]), slice).map(|i| i..i + 1),
            _ => memmem2_(slice, literal),
        }
    }
    #[inline(always)]
    fn memmem3(
        slice: &[u8],
        literal: (&[u8], &[u8], &[u8]),
    ) -> Option<core::ops::Range<usize>> {
        match (literal.0.len(), literal.1.len(), literal.2.len()) {
            (0, _, _) | (_, 0, _) | (_, _, 0) => Some(0..0),
            (1, 1, 1) => {
                memchr3((literal.0[0], literal.1[0], literal.2[0]), slice)
                    .map(|i| i..i + 1)
            }
            _ => memmem3_(slice, literal),
        }
    }
    fn memmem_(slice: &[u8], literal: &[u8]) -> Option<core::ops::Range<usize>> {
        for i in 0..slice.len() {
            let subslice = &slice[i..];
            if subslice.starts_with(literal) {
                let i_end = i + literal.len();
                return Some(i..i_end);
            }
        }
        None
    }
    fn memmem2_(
        slice: &[u8],
        literal: (&[u8], &[u8]),
    ) -> Option<core::ops::Range<usize>> {
        for i in 0..slice.len() {
            let subslice = &slice[i..];
            if subslice.starts_with(literal.0) {
                let i_end = i + literal.0.len();
                return Some(i..i_end);
            }
            if subslice.starts_with(literal.1) {
                let i_end = i + literal.1.len();
                return Some(i..i_end);
            }
        }
        None
    }
    fn memmem3_(
        slice: &[u8],
        literal: (&[u8], &[u8], &[u8]),
    ) -> Option<core::ops::Range<usize>> {
        for i in 0..slice.len() {
            let subslice = &slice[i..];
            if subslice.starts_with(literal.0) {
                let i_end = i + literal.0.len();
                return Some(i..i_end);
            }
            if subslice.starts_with(literal.1) {
                let i_end = i + literal.1.len();
                return Some(i..i_end);
            }
            if subslice.starts_with(literal.2) {
                let i_end = i + literal.2.len();
                return Some(i..i_end);
            }
        }
        None
    }
}
pub mod ascii {
    //! Character specific parsers and combinators
    //!
    //! Functions recognizing specific characters
    mod caseless {
        use crate::error::ParserError;
        use crate::stream::{Compare, CompareResult};
        use crate::stream::{SliceLen, Stream, StreamIsPartial};
        use crate::Parser;
        use crate::Result;
        /// Mark a value as case-insensitive for ASCII characters
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::ascii::Caseless;
        ///
        /// fn parser<'s>(s: &mut &'s str) -> ModalResult<&'s str> {
        ///   Caseless("hello").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("Hello, World!"), Ok((", World!", "Hello")));
        /// assert_eq!(parser.parse_peek("hello, World!"), Ok((", World!", "hello")));
        /// assert_eq!(parser.parse_peek("HeLlo, World!"), Ok((", World!", "HeLlo")));
        /// assert!(parser.parse_peek("Some").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// ```
        pub struct Caseless<T>(pub T);
        #[automatically_derived]
        impl<T: ::core::marker::Copy> ::core::marker::Copy for Caseless<T> {}
        #[automatically_derived]
        impl<T: ::core::clone::Clone> ::core::clone::Clone for Caseless<T> {
            #[inline]
            fn clone(&self) -> Caseless<T> {
                Caseless(::core::clone::Clone::clone(&self.0))
            }
        }
        #[automatically_derived]
        impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Caseless<T> {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "Caseless",
                    &&self.0,
                )
            }
        }
        impl Caseless<&str> {
            /// Get the byte-representation of this case-insensitive value
            #[inline(always)]
            pub fn as_bytes(&self) -> Caseless<&[u8]> {
                Caseless(self.0.as_bytes())
            }
        }
        impl<S: SliceLen> SliceLen for Caseless<S> {
            #[inline(always)]
            fn slice_len(&self) -> usize {
                self.0.slice_len()
            }
        }
        impl<'b> Compare<Caseless<&'b [u8]>> for &[u8] {
            #[inline]
            fn compare(&self, t: Caseless<&'b [u8]>) -> CompareResult {
                if t.0.iter().zip(*self).any(|(a, b)| !a.eq_ignore_ascii_case(b)) {
                    CompareResult::Error
                } else if self.len() < t.slice_len() {
                    CompareResult::Incomplete
                } else {
                    CompareResult::Ok(t.slice_len())
                }
            }
        }
        impl<const LEN: usize> Compare<Caseless<[u8; LEN]>> for &[u8] {
            #[inline(always)]
            fn compare(&self, t: Caseless<[u8; LEN]>) -> CompareResult {
                self.compare(Caseless(&t.0[..]))
            }
        }
        impl<'b, const LEN: usize> Compare<Caseless<&'b [u8; LEN]>> for &[u8] {
            #[inline(always)]
            fn compare(&self, t: Caseless<&'b [u8; LEN]>) -> CompareResult {
                self.compare(Caseless(&t.0[..]))
            }
        }
        impl<'b> Compare<Caseless<&'b str>> for &[u8] {
            #[inline(always)]
            fn compare(&self, t: Caseless<&'b str>) -> CompareResult {
                self.compare(Caseless(t.0.as_bytes()))
            }
        }
        impl Compare<Caseless<u8>> for &[u8] {
            #[inline]
            fn compare(&self, t: Caseless<u8>) -> CompareResult {
                match self.first() {
                    Some(c) if t.0.eq_ignore_ascii_case(c) => {
                        CompareResult::Ok(t.slice_len())
                    }
                    Some(_) => CompareResult::Error,
                    None => CompareResult::Incomplete,
                }
            }
        }
        impl Compare<Caseless<char>> for &[u8] {
            #[inline(always)]
            fn compare(&self, t: Caseless<char>) -> CompareResult {
                self.compare(Caseless(t.0.encode_utf8(&mut [0; 4]).as_bytes()))
            }
        }
        impl<'b> Compare<Caseless<&'b str>> for &str {
            #[inline(always)]
            fn compare(&self, t: Caseless<&'b str>) -> CompareResult {
                self.as_bytes().compare(t.as_bytes())
            }
        }
        impl Compare<Caseless<char>> for &str {
            #[inline(always)]
            fn compare(&self, t: Caseless<char>) -> CompareResult {
                self.as_bytes().compare(t)
            }
        }
        /// This is a shortcut for [`literal`][crate::token::literal].
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
        /// # use winnow::combinator::alt;
        /// # use winnow::token::take;
        /// use winnow::ascii::Caseless;
        ///
        /// fn parser<'s>(s: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
        ///   alt((Caseless(&"hello"[..]), take(5usize))).parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek(&b"Hello, World!"[..]), Ok((&b", World!"[..], &b"Hello"[..])));
        /// assert_eq!(parser.parse_peek(&b"hello, World!"[..]), Ok((&b", World!"[..], &b"hello"[..])));
        /// assert_eq!(parser.parse_peek(&b"HeLlo, World!"[..]), Ok((&b", World!"[..], &b"HeLlo"[..])));
        /// assert_eq!(parser.parse_peek(&b"Something"[..]), Ok((&b"hing"[..], &b"Somet"[..])));
        /// assert!(parser.parse_peek(&b"Some"[..]).is_err());
        /// assert!(parser.parse_peek(&b""[..]).is_err());
        /// ```
        impl<'s, I, E: ParserError<I>> Parser<I, <I as Stream>::Slice, E>
        for Caseless<&'s [u8]>
        where
            I: Compare<Caseless<&'s [u8]>> + StreamIsPartial,
            I: Stream,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
                crate::token::literal(*self).parse_next(i)
            }
        }
        /// This is a shortcut for [`literal`][crate::token::literal].
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
        /// # use winnow::combinator::alt;
        /// # use winnow::token::take;
        /// use winnow::ascii::Caseless;
        ///
        /// fn parser<'s>(s: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
        ///   alt((Caseless(b"hello"), take(5usize))).parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek(&b"Hello, World!"[..]), Ok((&b", World!"[..], &b"Hello"[..])));
        /// assert_eq!(parser.parse_peek(&b"hello, World!"[..]), Ok((&b", World!"[..], &b"hello"[..])));
        /// assert_eq!(parser.parse_peek(&b"HeLlo, World!"[..]), Ok((&b", World!"[..], &b"HeLlo"[..])));
        /// assert_eq!(parser.parse_peek(&b"Something"[..]), Ok((&b"hing"[..], &b"Somet"[..])));
        /// assert!(parser.parse_peek(&b"Some"[..]).is_err());
        /// assert!(parser.parse_peek(&b""[..]).is_err());
        /// ```
        impl<'s, I, E: ParserError<I>, const N: usize> Parser<I, <I as Stream>::Slice, E>
        for Caseless<&'s [u8; N]>
        where
            I: Compare<Caseless<&'s [u8; N]>> + StreamIsPartial,
            I: Stream,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
                crate::token::literal(*self).parse_next(i)
            }
        }
        /// This is a shortcut for [`literal`][crate::token::literal].
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::{error::ErrMode, error::ContextError};
        /// # use winnow::combinator::alt;
        /// # use winnow::token::take;
        /// # use winnow::ascii::Caseless;
        ///
        /// fn parser<'s>(s: &mut &'s str) -> ModalResult<&'s str> {
        ///   alt((Caseless("hello"), take(5usize))).parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("Hello, World!"), Ok((", World!", "Hello")));
        /// assert_eq!(parser.parse_peek("hello, World!"), Ok((", World!", "hello")));
        /// assert_eq!(parser.parse_peek("HeLlo, World!"), Ok((", World!", "HeLlo")));
        /// assert_eq!(parser.parse_peek("Something"), Ok(("hing", "Somet")));
        /// assert!(parser.parse_peek("Some").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// ```
        impl<'s, I, E: ParserError<I>> Parser<I, <I as Stream>::Slice, E>
        for Caseless<&'s str>
        where
            I: Compare<Caseless<&'s str>> + StreamIsPartial,
            I: Stream,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<<I as Stream>::Slice, E> {
                crate::token::literal(*self).parse_next(i)
            }
        }
    }
    pub use self::caseless::Caseless;
    use core::ops::{Add, Shl};
    use crate::combinator::alt;
    use crate::combinator::dispatch;
    use crate::combinator::empty;
    use crate::combinator::fail;
    use crate::combinator::opt;
    use crate::combinator::peek;
    use crate::combinator::trace;
    use crate::error::Needed;
    use crate::error::ParserError;
    use crate::stream::FindSlice;
    use crate::stream::{AsBStr, AsChar, ParseSlice, Stream, StreamIsPartial};
    use crate::stream::{Compare, CompareResult};
    use crate::token::any;
    use crate::token::one_of;
    use crate::token::take_until;
    use crate::token::take_while;
    use crate::Parser;
    use crate::Result;
    /// Recognizes the string `"\r\n"`.
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn crlf<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::crlf.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::crlf;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     crlf.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("\r\nc"), Ok(("c", "\r\n")));
    /// assert!(parser.parse_peek("ab\r\nc").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::crlf;
    /// assert_eq!(crlf::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\r\nc")), Ok((Partial::new("c"), "\r\n")));
    /// assert!(crlf::<_, ErrMode<ContextError>>.parse_peek(Partial::new("ab\r\nc")).is_err());
    /// assert_eq!(crlf::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    #[inline(always)]
    pub fn crlf<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream + Compare<&'static str>,
        Error: ParserError<Input>,
    {
        trace("crlf", "\r\n").parse_next(input)
    }
    /// Recognizes a string of 0+ characters until `"\r\n"`, `"\n"`, or eof.
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn till_line_ending<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::till_line_ending.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::till_line_ending;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     till_line_ending.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("ab\r\nc"), Ok(("\r\nc", "ab")));
    /// assert_eq!(parser.parse_peek("ab\nc"), Ok(("\nc", "ab")));
    /// assert_eq!(parser.parse_peek("abc"), Ok(("", "abc")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// assert!(parser.parse_peek("a\rb\nc").is_err());
    /// assert!(parser.parse_peek("a\rbc").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::till_line_ending;
    /// assert_eq!(till_line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("ab\r\nc")), Ok((Partial::new("\r\nc"), "ab")));
    /// assert_eq!(till_line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("abc")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert_eq!(till_line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert!(till_line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("a\rb\nc")).is_err());
    /// assert!(till_line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("a\rbc")).is_err());
    /// ```
    #[inline(always)]
    pub fn till_line_ending<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream + Compare<&'static str>
            + FindSlice<(char, char)>,
        <Input as Stream>::Token: AsChar + Clone,
        Error: ParserError<Input>,
    {
        trace(
                "till_line_ending",
                move |input: &mut Input| {
                    if <Input as StreamIsPartial>::is_partial_supported() {
                        till_line_ending_::<_, _, true>(input)
                    } else {
                        till_line_ending_::<_, _, false>(input)
                    }
                },
            )
            .parse_next(input)
    }
    fn till_line_ending_<I, E: ParserError<I>, const PARTIAL: bool>(
        input: &mut I,
    ) -> Result<<I as Stream>::Slice, E>
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<&'static str>,
        I: FindSlice<(char, char)>,
        <I as Stream>::Token: AsChar + Clone,
    {
        let res = match take_until(0.., ('\r', '\n')).parse_next(input).map_err(|e: E| e)
        {
            Ok(slice) => slice,
            Err(err) if err.is_backtrack() => input.finish(),
            Err(err) => {
                return Err(err);
            }
        };
        if #[allow(non_exhaustive_omitted_patterns)]
        match input.compare("\r") {
            CompareResult::Ok(_) => true,
            _ => false,
        } {
            let comp = input.compare("\r\n");
            match comp {
                CompareResult::Ok(_) => {}
                CompareResult::Incomplete if PARTIAL && input.is_partial() => {
                    return Err(ParserError::incomplete(input, Needed::Unknown));
                }
                CompareResult::Incomplete | CompareResult::Error => {
                    return Err(ParserError::from_input(input));
                }
            }
        }
        Ok(res)
    }
    /// Recognizes an end of line (both `"\n"` and `"\r\n"`).
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn line_ending<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::line_ending.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::line_ending;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     line_ending.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("\r\nc"), Ok(("c", "\r\n")));
    /// assert!(parser.parse_peek("ab\r\nc").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::line_ending;
    /// assert_eq!(line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\r\nc")), Ok((Partial::new("c"), "\r\n")));
    /// assert!(line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("ab\r\nc")).is_err());
    /// assert_eq!(line_ending::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    #[inline(always)]
    pub fn line_ending<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream + Compare<&'static str>,
        Error: ParserError<Input>,
    {
        trace("line_ending", alt(("\n", "\r\n"))).parse_next(input)
    }
    /// Matches a newline character `'\n'`.
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn newline(input: &mut &str) -> ModalResult<char>
    /// # {
    /// #     winnow::ascii::newline.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::newline;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<char> {
    ///     newline.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("\nc"), Ok(("c", '\n')));
    /// assert!(parser.parse_peek("\r\nc").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::newline;
    /// assert_eq!(newline::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\nc")), Ok((Partial::new("c"), '\n')));
    /// assert!(newline::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\r\nc")).is_err());
    /// assert_eq!(newline::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    #[inline(always)]
    pub fn newline<I, Error: ParserError<I>>(input: &mut I) -> Result<char, Error>
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<char>,
    {
        trace("newline", '\n').parse_next(input)
    }
    /// Matches a tab character `'\t'`.
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn tab(input: &mut &str) -> ModalResult<char>
    /// # {
    /// #     winnow::ascii::tab.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::tab;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<char> {
    ///     tab.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("\tc"), Ok(("c", '\t')));
    /// assert!(parser.parse_peek("\r\nc").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::tab;
    /// assert_eq!(tab::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\tc")), Ok((Partial::new("c"), '\t')));
    /// assert!(tab::<_, ErrMode<ContextError>>.parse_peek(Partial::new("\r\nc")).is_err());
    /// assert_eq!(tab::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    #[inline(always)]
    pub fn tab<Input, Error>(input: &mut Input) -> Result<char, Error>
    where
        Input: StreamIsPartial + Stream + Compare<char>,
        Error: ParserError<Input>,
    {
        trace("tab", '\t').parse_next(input)
    }
    /// Recognizes zero or more lowercase and uppercase ASCII alphabetic characters: `'a'..='z'`, `'A'..='Z'`
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non
    /// alphabetic character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non alphabetic character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn alpha0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::alpha0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::alpha0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     alpha0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("ab1c"), Ok(("1c", "ab")));
    /// assert_eq!(parser.parse_peek("1c"), Ok(("1c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::alpha0;
    /// assert_eq!(alpha0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("ab1c")), Ok((Partial::new("1c"), "ab")));
    /// assert_eq!(alpha0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("1c")), Ok((Partial::new("1c"), "")));
    /// assert_eq!(alpha0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn alpha0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("alpha0", take_while(0.., AsChar::is_alpha)).parse_next(input)
    }
    /// Recognizes one or more lowercase and uppercase ASCII alphabetic characters: `'a'..='z'`, `'A'..='Z'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found  (a non alphabetic character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non alphabetic character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn alpha1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::alpha1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::alpha1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     alpha1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("aB1c"), Ok(("1c", "aB")));
    /// assert!(parser.parse_peek("1c").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::alpha1;
    /// assert_eq!(alpha1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("aB1c")), Ok((Partial::new("1c"), "aB")));
    /// assert!(alpha1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("1c")).is_err());
    /// assert_eq!(alpha1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn alpha1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("alpha1", take_while(1.., AsChar::is_alpha)).parse_next(input)
    }
    /// Recognizes zero or more ASCII numerical characters: `'0'..='9'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn digit0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::digit0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::digit0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     digit0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21c"), Ok(("c", "21")));
    /// assert_eq!(parser.parse_peek("21"), Ok(("", "21")));
    /// assert_eq!(parser.parse_peek("a21c"), Ok(("a21c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::digit0;
    /// assert_eq!(digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21c")), Ok((Partial::new("c"), "21")));
    /// assert_eq!(digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("a21c")), Ok((Partial::new("a21c"), "")));
    /// assert_eq!(digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn digit0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("digit0", take_while(0.., AsChar::is_dec_digit)).parse_next(input)
    }
    /// Recognizes one or more ASCII numerical characters: `'0'..='9'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn digit1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::digit1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::digit1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     digit1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21c"), Ok(("c", "21")));
    /// assert!(parser.parse_peek("c1").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::digit1;
    /// assert_eq!(digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21c")), Ok((Partial::new("c"), "21")));
    /// assert!(digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("c1")).is_err());
    /// assert_eq!(digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    ///
    /// ## Parsing an integer
    ///
    /// You can use `digit1` in combination with [`Parser::try_map`] to parse an integer:
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::digit1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<u32> {
    ///   digit1.try_map(str::parse).parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("416"), Ok(("", 416)));
    /// assert_eq!(parser.parse_peek("12b"), Ok(("b", 12)));
    /// assert!(parser.parse_peek("b").is_err());
    /// ```
    #[inline(always)]
    pub fn digit1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("digit1", take_while(1.., AsChar::is_dec_digit)).parse_next(input)
    }
    /// Recognizes zero or more ASCII hexadecimal numerical characters: `'0'..='9'`, `'A'..='F'`,
    /// `'a'..='f'`
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non hexadecimal digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non hexadecimal digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn hex_digit0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::hex_digit0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::hex_digit0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     hex_digit0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ"), Ok(("Z", "21c")));
    /// assert_eq!(parser.parse_peek("Z21c"), Ok(("Z21c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::hex_digit0;
    /// assert_eq!(hex_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ")), Ok((Partial::new("Z"), "21c")));
    /// assert_eq!(hex_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("Z21c")), Ok((Partial::new("Z21c"), "")));
    /// assert_eq!(hex_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn hex_digit0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("hex_digit0", take_while(0.., AsChar::is_hex_digit)).parse_next(input)
    }
    /// Recognizes one or more ASCII hexadecimal numerical characters: `'0'..='9'`, `'A'..='F'`,
    /// `'a'..='f'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non hexadecimal digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non hexadecimal digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn hex_digit1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::hex_digit1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::hex_digit1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     hex_digit1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ"), Ok(("Z", "21c")));
    /// assert!(parser.parse_peek("H2").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::hex_digit1;
    /// assert_eq!(hex_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ")), Ok((Partial::new("Z"), "21c")));
    /// assert!(hex_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("H2")).is_err());
    /// assert_eq!(hex_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn hex_digit1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("hex_digit1", take_while(1.., AsChar::is_hex_digit)).parse_next(input)
    }
    /// Recognizes zero or more octal characters: `'0'..='7'`
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non octal
    /// digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non octal digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn oct_digit0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::oct_digit0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::oct_digit0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     oct_digit0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ"), Ok(("cZ", "21")));
    /// assert_eq!(parser.parse_peek("Z21c"), Ok(("Z21c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::oct_digit0;
    /// assert_eq!(oct_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ")), Ok((Partial::new("cZ"), "21")));
    /// assert_eq!(oct_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("Z21c")), Ok((Partial::new("Z21c"), "")));
    /// assert_eq!(oct_digit0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn oct_digit0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial,
        Input: Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("oct_digit0", take_while(0.., AsChar::is_oct_digit)).parse_next(input)
    }
    /// Recognizes one or more octal characters: `'0'..='7'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non octal digit character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non octal digit character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn oct_digit1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::oct_digit1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::oct_digit1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     oct_digit1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ"), Ok(("cZ", "21")));
    /// assert!(parser.parse_peek("H2").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::oct_digit1;
    /// assert_eq!(oct_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ")), Ok((Partial::new("cZ"), "21")));
    /// assert!(oct_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("H2")).is_err());
    /// assert_eq!(oct_digit1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn oct_digit1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("oct_digit1", take_while(1.., AsChar::is_oct_digit)).parse_next(input)
    }
    /// Recognizes zero or more ASCII numerical and alphabetic characters: `'a'..='z'`, `'A'..='Z'`, `'0'..='9'`
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non
    /// alphanumerical character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non alphanumerical character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn alphanumeric0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::alphanumeric0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::alphanumeric0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     alphanumeric0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ%1"), Ok(("%1", "21cZ")));
    /// assert_eq!(parser.parse_peek("&Z21c"), Ok(("&Z21c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::alphanumeric0;
    /// assert_eq!(alphanumeric0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ%1")), Ok((Partial::new("%1"), "21cZ")));
    /// assert_eq!(alphanumeric0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("&Z21c")), Ok((Partial::new("&Z21c"), "")));
    /// assert_eq!(alphanumeric0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn alphanumeric0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("alphanumeric0", take_while(0.., AsChar::is_alphanum)).parse_next(input)
    }
    /// Recognizes one or more ASCII numerical and alphabetic characters: `'a'..='z'`, `'A'..='Z'`, `'0'..='9'`
    ///
    /// *Complete version*: Will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non alphanumerical character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non alphanumerical character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn alphanumeric1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::alphanumeric1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::alphanumeric1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     alphanumeric1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("21cZ%1"), Ok(("%1", "21cZ")));
    /// assert!(parser.parse_peek("&H2").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::alphanumeric1;
    /// assert_eq!(alphanumeric1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("21cZ%1")), Ok((Partial::new("%1"), "21cZ")));
    /// assert!(alphanumeric1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("&H2")).is_err());
    /// assert_eq!(alphanumeric1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn alphanumeric1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("alphanumeric1", take_while(1.., AsChar::is_alphanum)).parse_next(input)
    }
    /// Recognizes zero or more spaces and tabs.
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non space
    /// character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non space character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn space0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::space0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::space0;
    /// assert_eq!(space0::<_, ErrMode<ContextError>>.parse_peek(Partial::new(" \t21c")), Ok((Partial::new("21c"), " \t")));
    /// assert_eq!(space0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("Z21c")), Ok((Partial::new("Z21c"), "")));
    /// assert_eq!(space0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn space0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("space0", take_while(0.., AsChar::is_space)).parse_next(input)
    }
    /// Recognizes one or more spaces and tabs.
    ///
    /// *Complete version*: Will return the whole input if no terminating token is found (a non space
    /// character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non space character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn space1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::space1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::space1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     space1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(" \t21c"), Ok(("21c", " \t")));
    /// assert!(parser.parse_peek("H2").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::space1;
    /// assert_eq!(space1::<_, ErrMode<ContextError>>.parse_peek(Partial::new(" \t21c")), Ok((Partial::new("21c"), " \t")));
    /// assert!(space1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("H2")).is_err());
    /// assert_eq!(space1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn space1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        Error: ParserError<Input>,
    {
        trace("space1", take_while(1.., AsChar::is_space)).parse_next(input)
    }
    /// Recognizes zero or more spaces, tabs, carriage returns and line feeds.
    ///
    /// *Complete version*: will return the whole input if no terminating token is found (a non space
    /// character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non space character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn multispace0<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::multispace0.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::multispace0;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     multispace0.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
    /// assert_eq!(parser.parse_peek("Z21c"), Ok(("Z21c", "")));
    /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::multispace0;
    /// assert_eq!(multispace0::<_, ErrMode<ContextError>>.parse_peek(Partial::new(" \t\n\r21c")), Ok((Partial::new("21c"), " \t\n\r")));
    /// assert_eq!(multispace0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("Z21c")), Ok((Partial::new("Z21c"), "")));
    /// assert_eq!(multispace0::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn multispace0<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar + Clone,
        Error: ParserError<Input>,
    {
        trace("multispace0", take_while(0.., (' ', '\t', '\r', '\n'))).parse_next(input)
    }
    /// Recognizes one or more spaces, tabs, carriage returns and line feeds.
    ///
    /// *Complete version*: will return an error if there's not enough input data,
    /// or the whole input if no terminating token is found (a non space character).
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data,
    /// or if no terminating token is found (a non space character).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn multispace1<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::ascii::multispace1.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::multispace1;
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    ///     multispace1.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(" \t\n\r21c"), Ok(("21c", " \t\n\r")));
    /// assert!(parser.parse_peek("H2").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::ascii::multispace1;
    /// assert_eq!(multispace1::<_, ErrMode<ContextError>>.parse_peek(Partial::new(" \t\n\r21c")), Ok((Partial::new("21c"), " \t\n\r")));
    /// assert!(multispace1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("H2")).is_err());
    /// assert_eq!(multispace1::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn multispace1<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar + Clone,
        Error: ParserError<Input>,
    {
        trace("multispace1", take_while(1.., (' ', '\t', '\r', '\n'))).parse_next(input)
    }
    /// Decode a decimal unsigned integer (e.g. [`u32`])
    ///
    /// *Complete version*: can parse until the end of input.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] into a `u32`:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn dec_uint(input: &mut &str) -> ModalResult<u32>
    /// # {
    /// #     winnow::ascii::dec_uint.parse_next(input)
    /// # }
    /// ```
    #[doc(alias = "u8")]
    #[doc(alias = "u16")]
    #[doc(alias = "u32")]
    #[doc(alias = "u64")]
    #[doc(alias = "u128")]
    pub fn dec_uint<Input, Output, Error>(input: &mut Input) -> Result<Output, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Slice: AsBStr,
        <Input as Stream>::Token: AsChar + Clone,
        Output: Uint,
        Error: ParserError<Input>,
    {
        trace(
                "dec_uint",
                move |input: &mut Input| {
                    alt(((one_of('1'..='9'), digit0).void(), one_of('0').void()))
                        .take()
                        .verify_map(|s: <Input as Stream>::Slice| {
                            let s = s.as_bstr();
                            let s = unsafe { core::str::from_utf8_unchecked(s) };
                            Output::try_from_dec_uint(s)
                        })
                        .parse_next(input)
                },
            )
            .parse_next(input)
    }
    /// Metadata for parsing unsigned integers, see [`dec_uint`]
    pub trait Uint: Sized {
        #[doc(hidden)]
        fn try_from_dec_uint(slice: &str) -> Option<Self>;
    }
    impl Uint for u8 {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Uint for u16 {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Uint for u32 {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Uint for u64 {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Uint for u128 {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Uint for usize {
        fn try_from_dec_uint(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    /// Decode a decimal signed integer (e.g. [`i32`])
    ///
    /// *Complete version*: can parse until the end of input.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] into an `i32`:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn dec_int(input: &mut &str) -> ModalResult<i32>
    /// # {
    /// #     winnow::ascii::dec_int.parse_next(input)
    /// # }
    /// ```
    #[doc(alias = "i8")]
    #[doc(alias = "i16")]
    #[doc(alias = "i32")]
    #[doc(alias = "i64")]
    #[doc(alias = "i128")]
    pub fn dec_int<Input, Output, Error>(input: &mut Input) -> Result<Output, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Slice: AsBStr,
        <Input as Stream>::Token: AsChar + Clone,
        Output: Int,
        Error: ParserError<Input>,
    {
        trace(
                "dec_int",
                move |input: &mut Input| {
                    let sign = opt(
                        crate::combinator::trace(
                            "dispatch",
                            move |i: &mut _| {
                                use crate::Parser;
                                let initial = any.map(AsChar::as_char).parse_next(i)?;
                                match initial {
                                    '+' => empty.value(true).parse_next(i),
                                    '-' => empty.value(false).parse_next(i),
                                    _ => fail.parse_next(i),
                                }
                            },
                        ),
                    );
                    alt(((sign, one_of('1'..='9'), digit0).void(), one_of('0').void()))
                        .take()
                        .verify_map(|s: <Input as Stream>::Slice| {
                            let s = s.as_bstr();
                            let s = unsafe { core::str::from_utf8_unchecked(s) };
                            Output::try_from_dec_int(s)
                        })
                        .parse_next(input)
                },
            )
            .parse_next(input)
    }
    /// Metadata for parsing signed integers, see [`dec_int`]
    pub trait Int: Sized {
        #[doc(hidden)]
        fn try_from_dec_int(slice: &str) -> Option<Self>;
    }
    impl Int for i8 {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Int for i16 {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Int for i32 {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Int for i64 {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Int for i128 {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    impl Int for isize {
        fn try_from_dec_int(slice: &str) -> Option<Self> {
            slice.parse().ok()
        }
    }
    /// Decode a variable-width hexadecimal integer (e.g. [`u32`])
    ///
    /// *Complete version*: Will parse until the end of input if it has fewer characters than the type
    /// supports.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if end-of-input
    /// is hit before a hard boundary (non-hex character, more characters than supported).
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] into a `u32`:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn hex_uint(input: &mut &str) -> ModalResult<u32>
    /// # {
    /// #     winnow::ascii::hex_uint.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// use winnow::ascii::hex_uint;
    ///
    /// fn parser<'s>(s: &mut &'s [u8]) -> ModalResult<u32> {
    ///   hex_uint(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"01AE"[..]), Ok((&b""[..], 0x01AE)));
    /// assert_eq!(parser.parse_peek(&b"abc"[..]), Ok((&b""[..], 0x0ABC)));
    /// assert!(parser.parse_peek(&b"ggg"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::Needed};
    /// # use winnow::Partial;
    /// use winnow::ascii::hex_uint;
    ///
    /// fn parser<'s>(s: &mut Partial<&'s [u8]>) -> ModalResult<u32> {
    ///   hex_uint(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"01AE;"[..])), Ok((Partial::new(&b";"[..]), 0x01AE)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert!(parser.parse_peek(Partial::new(&b"ggg"[..])).is_err());
    /// ```
    #[inline]
    pub fn hex_uint<Input, Output, Error>(input: &mut Input) -> Result<Output, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: AsChar,
        <Input as Stream>::Slice: AsBStr,
        Output: HexUint,
        Error: ParserError<Input>,
    {
        trace(
                "hex_uint",
                move |input: &mut Input| {
                    let invalid_offset = input
                        .offset_for(|c| !c.is_hex_digit())
                        .unwrap_or_else(|| input.eof_offset());
                    let max_nibbles = Output::max_nibbles(sealed::SealedMarker);
                    let max_offset = input.offset_at(max_nibbles);
                    let offset = match max_offset {
                        Ok(max_offset) => {
                            if max_offset < invalid_offset {
                                return Err(ParserError::from_input(input));
                            } else {
                                invalid_offset
                            }
                        }
                        Err(_) => {
                            if <Input as StreamIsPartial>::is_partial_supported()
                                && input.is_partial()
                                && invalid_offset == input.eof_offset()
                            {
                                return Err(ParserError::incomplete(input, Needed::new(1)));
                            } else {
                                invalid_offset
                            }
                        }
                    };
                    if offset == 0 {
                        return Err(ParserError::from_input(input));
                    }
                    let parsed = input.next_slice(offset);
                    let mut res = Output::default();
                    for &c in parsed.as_bstr() {
                        let nibble = match c {
                            b'0'..=b'9' => c - b'0',
                            b'a'..=b'f' => c - b'a' + 10,
                            b'A'..=b'F' => c - b'A' + 10,
                            _ => {
                                ::core::panicking::panic(
                                    "internal error: entered unreachable code",
                                )
                            }
                        };
                        let nibble = Output::from(nibble);
                        res = (res << Output::from(4)) + nibble;
                    }
                    Ok(res)
                },
            )
            .parse_next(input)
    }
    /// Metadata for parsing hex numbers, see [`hex_uint`]
    pub trait HexUint: Default + Shl<
            Self,
            Output = Self,
        > + Add<Self, Output = Self> + From<u8> {
        #[doc(hidden)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize;
    }
    impl HexUint for u8 {
        #[inline(always)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize {
            2
        }
    }
    impl HexUint for u16 {
        #[inline(always)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize {
            4
        }
    }
    impl HexUint for u32 {
        #[inline(always)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize {
            8
        }
    }
    impl HexUint for u64 {
        #[inline(always)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize {
            16
        }
    }
    impl HexUint for u128 {
        #[inline(always)]
        fn max_nibbles(_: sealed::SealedMarker) -> usize {
            32
        }
    }
    /// Recognizes floating point number in text format and returns a [`f32`] or [`f64`].
    ///
    /// *Complete version*: Can parse until the end of input.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] into an `f64`:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn float(input: &mut &str) -> ModalResult<f64>
    /// # {
    /// #     winnow::ascii::float.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::ascii::float;
    ///
    /// fn parser<'s>(s: &mut &'s str) -> ModalResult<f64> {
    ///   float(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("11e-1"), Ok(("", 1.1)));
    /// assert_eq!(parser.parse_peek("123E-02"), Ok(("", 1.23)));
    /// assert_eq!(parser.parse_peek("123K-01"), Ok(("K-01", 123.0)));
    /// assert!(parser.parse_peek("abc").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::Needed};
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::ascii::float;
    ///
    /// fn parser<'s>(s: &mut Partial<&'s str>) -> ModalResult<f64> {
    ///   float(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new("11e-1 ")), Ok((Partial::new(" "), 1.1)));
    /// assert_eq!(parser.parse_peek(Partial::new("11e-1")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(parser.parse_peek(Partial::new("123E-02")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(parser.parse_peek(Partial::new("123K-01")), Ok((Partial::new("K-01"), 123.0)));
    /// assert!(parser.parse_peek(Partial::new("abc")).is_err());
    /// ```
    #[inline(always)]
    #[doc(alias = "f32")]
    #[doc(alias = "double")]
    #[allow(clippy::trait_duplication_in_bounds)]
    pub fn float<Input, Output, Error>(input: &mut Input) -> Result<Output, Error>
    where
        Input: StreamIsPartial + Stream + Compare<Caseless<&'static str>> + Compare<char>
            + AsBStr,
        <Input as Stream>::Slice: ParseSlice<Output>,
        <Input as Stream>::Token: AsChar + Clone,
        <Input as Stream>::IterOffsets: Clone,
        Error: ParserError<Input>,
    {
        trace(
                "float",
                move |input: &mut Input| {
                    let s = take_float_or_exceptions(input)?;
                    s.parse_slice().ok_or_else(|| ParserError::from_input(input))
                },
            )
            .parse_next(input)
    }
    #[allow(clippy::trait_duplication_in_bounds)]
    fn take_float_or_exceptions<I, E: ParserError<I>>(
        input: &mut I,
    ) -> Result<<I as Stream>::Slice, E>
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<Caseless<&'static str>>,
        I: Compare<char>,
        <I as Stream>::Token: AsChar + Clone,
        <I as Stream>::IterOffsets: Clone,
        I: AsBStr,
    {
        crate::combinator::trace(
                "dispatch",
                move |i: &mut _| {
                    use crate::Parser;
                    let initial = opt(peek(any).map(AsChar::as_char)).parse_next(i)?;
                    match initial {
                        Some('N') | Some('n') => Caseless("nan").void().parse_next(i),
                        Some('+') | Some('-') => {
                            (any, take_unsigned_float_or_exceptions).void().parse_next(i)
                        }
                        _ => take_unsigned_float_or_exceptions.parse_next(i),
                    }
                },
            )
            .take()
            .parse_next(input)
    }
    #[allow(clippy::trait_duplication_in_bounds)]
    fn take_unsigned_float_or_exceptions<I, E: ParserError<I>>(
        input: &mut I,
    ) -> Result<(), E>
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<Caseless<&'static str>>,
        I: Compare<char>,
        <I as Stream>::Token: AsChar + Clone,
        <I as Stream>::IterOffsets: Clone,
        I: AsBStr,
    {
        crate::combinator::trace(
                "dispatch",
                move |i: &mut _| {
                    use crate::Parser;
                    let initial = opt(peek(any).map(AsChar::as_char)).parse_next(i)?;
                    match initial {
                        Some('I') | Some('i') => {
                            (Caseless("inf"), opt(Caseless("inity")))
                                .void()
                                .parse_next(i)
                        }
                        Some('.') => ('.', digit1, take_exp).void().parse_next(i),
                        _ => {
                            (digit1, opt(('.', opt(digit1))), take_exp)
                                .void()
                                .parse_next(i)
                        }
                    }
                },
            )
            .parse_next(input)
    }
    #[allow(clippy::trait_duplication_in_bounds)]
    fn take_exp<I, E: ParserError<I>>(input: &mut I) -> Result<(), E>
    where
        I: StreamIsPartial,
        I: Stream,
        I: Compare<char>,
        <I as Stream>::Token: AsChar + Clone,
        <I as Stream>::IterOffsets: Clone,
        I: AsBStr,
    {
        crate::combinator::trace(
                "dispatch",
                move |i: &mut _| {
                    use crate::Parser;
                    let initial = opt(peek(any).map(AsChar::as_char)).parse_next(i)?;
                    match initial {
                        Some('E') | Some('e') => {
                            (one_of(['e', 'E']), opt(one_of(['+', '-'])), digit1)
                                .void()
                                .parse_next(i)
                        }
                        _ => empty.parse_next(i),
                    }
                },
            )
            .parse_next(input)
    }
    /// Recognize the input slice with escaped characters.
    ///
    /// Arguments:
    /// - `normal`: unescapeable characters
    ///   - Must not include `control`
    /// - `control_char`: e.g. `\` for strings in most languages
    /// - `escape`: parse and transform the escaped character
    ///
    /// Parsing ends when:
    /// - `alt(normal, control_char)` [`Backtrack`s][crate::error::ErrMode::Backtrack]
    /// - `normal` doesn't advance the input stream
    /// - *(complete)* input stream is exhausted
    ///
    /// See also [`escaped`]
    ///
    /// <div class="warning">
    ///
    /// **Warning:** If the `normal` parser passed to `take_escaped` accepts empty inputs
    /// (like `alpha0` or `digit0`), `take_escaped` will return an error,
    /// to prevent going into an infinite loop.
    ///
    /// </div>
    ///
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::ascii::digit1;
    /// # use winnow::prelude::*;
    /// use winnow::ascii::take_escaped;
    /// use winnow::token::one_of;
    ///
    /// fn esc<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    ///   take_escaped(digit1, '\\', one_of(['"', 'n', '\\'])).parse_next(input)
    /// }
    ///
    /// assert_eq!(esc.parse_peek("123;"), Ok((";", "123")));
    /// assert_eq!(esc.parse_peek(r#"12\"34;"#), Ok((";", r#"12\"34"#)));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::Needed};
    /// # use winnow::ascii::digit1;
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::ascii::take_escaped;
    /// use winnow::token::one_of;
    ///
    /// fn esc<'i>(input: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take_escaped(digit1, '\\', one_of(['"', 'n', '\\'])).parse_next(input)
    /// }
    ///
    /// assert_eq!(esc.parse_peek(Partial::new("123;")), Ok((Partial::new(";"), "123")));
    /// assert_eq!(esc.parse_peek(Partial::new("12\\\"34;")), Ok((Partial::new(";"), "12\\\"34")));
    /// ```
    #[inline(always)]
    pub fn take_escaped<
        Input,
        Error,
        Normal,
        ControlChar,
        Escapable,
        NormalOutput,
        ControlCharOutput,
        EscapableOutput,
    >(
        mut normal: Normal,
        mut control_char: ControlChar,
        mut escapable: Escapable,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        Normal: Parser<Input, NormalOutput, Error>,
        ControlChar: Parser<Input, ControlCharOutput, ()>,
        Escapable: Parser<Input, EscapableOutput, Error>,
        Error: ParserError<Input>,
    {
        trace(
            "take_escaped",
            move |input: &mut Input| {
                if <Input as StreamIsPartial>::is_partial_supported()
                    && input.is_partial()
                {
                    escaped_internal::<
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        true,
                    >(input, &mut normal, &mut control_char, &mut escapable)
                } else {
                    escaped_internal::<
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        false,
                    >(input, &mut normal, &mut control_char, &mut escapable)
                }
            },
        )
    }
    fn escaped_internal<I, Error, F, ControlChar, G, O1, O2, O3, const PARTIAL: bool>(
        input: &mut I,
        normal: &mut F,
        control_char: &mut ControlChar,
        escapable: &mut G,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream,
        F: Parser<I, O1, Error>,
        ControlChar: Parser<I, O3, ()>,
        G: Parser<I, O2, Error>,
        Error: ParserError<I>,
    {
        let start = input.checkpoint();
        while input.eof_offset() > 0 {
            let current_len = input.eof_offset();
            match opt(normal.by_ref()).parse_next(input)? {
                Some(_) => {
                    if input.eof_offset() == current_len {
                        return Err(
                            ParserError::assert(
                                input,
                                "`take_escaped` parsers must always consume",
                            ),
                        );
                    }
                }
                None => {
                    if control_char.by_ref().parse_next(input).is_ok() {
                        let _ = escapable.parse_next(input)?;
                    } else {
                        let offset = input.offset_from(&start);
                        input.reset(&start);
                        return Ok(input.next_slice(offset));
                    }
                }
            }
        }
        if PARTIAL && input.is_partial() {
            Err(ParserError::incomplete(input, Needed::Unknown))
        } else {
            input.reset(&start);
            Ok(input.finish())
        }
    }
    /// Parse escaped characters, unescaping them
    ///
    /// Arguments:
    /// - `normal`: unescapeable characters
    ///   - Must not include `control`
    /// - `control_char`: e.g. `\` for strings in most languages
    /// - `escape`: parse and transform the escaped character
    ///
    /// Parsing ends when:
    /// - `alt(normal, control_char)` [`Backtrack`s][crate::error::ErrMode::Backtrack]
    /// - `normal` doesn't advance the input stream
    /// - *(complete)* input stream is exhausted
    ///
    /// <div class="warning">
    ///
    /// **Warning:** If the `normal` parser passed to `escaped` accepts empty inputs
    /// (like `alpha0` or `digit0`), `escaped` will return an error,
    /// to prevent going into an infinite loop.
    ///
    /// </div>
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "std")] {
    /// # use winnow::prelude::*;
    /// # use std::str::from_utf8;
    /// use winnow::token::literal;
    /// use winnow::ascii::escaped;
    /// use winnow::ascii::alpha1;
    /// use winnow::combinator::alt;
    ///
    /// fn parser<'s>(input: &mut &'s str) -> ModalResult<String> {
    ///   escaped(
    ///     alpha1,
    ///     '\\',
    ///     alt((
    ///       "\\".value("\\"),
    ///       "\"".value("\""),
    ///       "n".value("\n"),
    ///     ))
    ///   ).parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("ab\\\"cd"), Ok(("", String::from("ab\"cd"))));
    /// assert_eq!(parser.parse_peek("ab\\ncd"), Ok(("", String::from("ab\ncd"))));
    /// # }
    /// ```
    ///
    /// ```rust
    /// # #[cfg(feature = "std")] {
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::Needed};
    /// # use std::str::from_utf8;
    /// # use winnow::Partial;
    /// use winnow::token::literal;
    /// use winnow::ascii::escaped;
    /// use winnow::ascii::alpha1;
    /// use winnow::combinator::alt;
    ///
    /// fn parser<'s>(input: &mut Partial<&'s str>) -> ModalResult<String> {
    ///   escaped(
    ///     alpha1,
    ///     '\\',
    ///     alt((
    ///       "\\".value("\\"),
    ///       "\"".value("\""),
    ///       "n".value("\n"),
    ///     ))
    ///   ).parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new("ab\\\"cd\"")), Ok((Partial::new("\""), String::from("ab\"cd"))));
    /// # }
    /// ```
    #[inline(always)]
    pub fn escaped<
        Input,
        Error,
        Normal,
        ControlChar,
        Escape,
        NormalOutput,
        ControlCharOutput,
        EscapeOutput,
        Output,
    >(
        mut normal: Normal,
        mut control_char: ControlChar,
        mut escape: Escape,
    ) -> impl Parser<Input, Output, Error>
    where
        Input: StreamIsPartial + Stream,
        Normal: Parser<Input, NormalOutput, Error>,
        ControlChar: Parser<Input, ControlCharOutput, ()>,
        Escape: Parser<Input, EscapeOutput, Error>,
        Output: crate::stream::Accumulate<NormalOutput>,
        Output: crate::stream::Accumulate<EscapeOutput>,
        Error: ParserError<Input>,
    {
        trace(
            "escaped",
            move |input: &mut Input| {
                if <Input as StreamIsPartial>::is_partial_supported()
                    && input.is_partial()
                {
                    escaped_transform_internal::<
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        true,
                    >(input, &mut normal, &mut control_char, &mut escape)
                } else {
                    escaped_transform_internal::<
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        _,
                        false,
                    >(input, &mut normal, &mut control_char, &mut escape)
                }
            },
        )
    }
    fn escaped_transform_internal<
        I,
        Error,
        F,
        NormalOutput,
        ControlChar,
        ControlCharOutput,
        G,
        EscapeOutput,
        Output,
        const PARTIAL: bool,
    >(
        input: &mut I,
        normal: &mut F,
        control_char: &mut ControlChar,
        transform: &mut G,
    ) -> Result<Output, Error>
    where
        I: StreamIsPartial,
        I: Stream,
        Output: crate::stream::Accumulate<NormalOutput>,
        Output: crate::stream::Accumulate<EscapeOutput>,
        F: Parser<I, NormalOutput, Error>,
        ControlChar: Parser<I, ControlCharOutput, ()>,
        G: Parser<I, EscapeOutput, Error>,
        Error: ParserError<I>,
    {
        let mut res = <Output as crate::stream::Accumulate<
            NormalOutput,
        >>::initial(Some(input.eof_offset()));
        while input.eof_offset() > 0 {
            let current_len = input.eof_offset();
            match opt(normal.by_ref()).parse_next(input)? {
                Some(o) => {
                    res.accumulate(o);
                    if input.eof_offset() == current_len {
                        return Err(
                            ParserError::assert(
                                input,
                                "`escaped` parsers must always consume",
                            ),
                        );
                    }
                }
                None => {
                    if control_char.by_ref().parse_next(input).is_ok() {
                        let o = transform.parse_next(input)?;
                        res.accumulate(o);
                    } else {
                        return Ok(res);
                    }
                }
            }
        }
        if PARTIAL && input.is_partial() {
            Err(ParserError::incomplete(input, Needed::Unknown))
        } else {
            Ok(res)
        }
    }
    mod sealed {
        #[allow(unnameable_types)]
        pub struct SealedMarker;
    }
}
pub mod binary {
    //! Parsers recognizing numbers
    #![allow(clippy::match_same_arms)]
    pub mod bits {
        //! Bit level parsers
        //!
        mod stream {
            use core::num::NonZeroUsize;
            use crate::error::Needed;
            use crate::stream::{Checkpoint, Offset, Stream, StreamIsPartial};
            /// Bit-level stream state over a byte stream.
            pub struct Bits<I>(pub I, pub usize);
            #[automatically_derived]
            impl<I: ::core::marker::Copy> ::core::marker::Copy for Bits<I> {}
            #[automatically_derived]
            impl<I: ::core::clone::Clone> ::core::clone::Clone for Bits<I> {
                #[inline]
                fn clone(&self) -> Bits<I> {
                    Bits(
                        ::core::clone::Clone::clone(&self.0),
                        ::core::clone::Clone::clone(&self.1),
                    )
                }
            }
            #[automatically_derived]
            impl<I: ::core::fmt::Debug> ::core::fmt::Debug for Bits<I> {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::debug_tuple_field2_finish(
                        f,
                        "Bits",
                        &self.0,
                        &&self.1,
                    )
                }
            }
            #[automatically_derived]
            impl<I> ::core::marker::StructuralPartialEq for Bits<I> {}
            #[automatically_derived]
            impl<I: ::core::cmp::PartialEq> ::core::cmp::PartialEq for Bits<I> {
                #[inline]
                fn eq(&self, other: &Bits<I>) -> bool {
                    self.0 == other.0 && self.1 == other.1
                }
            }
            #[automatically_derived]
            impl<I: ::core::cmp::Eq> ::core::cmp::Eq for Bits<I> {
                #[inline]
                #[doc(hidden)]
                #[coverage(off)]
                fn assert_receiver_is_total_eq(&self) {
                    let _: ::core::cmp::AssertParamIsEq<I>;
                    let _: ::core::cmp::AssertParamIsEq<usize>;
                }
            }
            impl<I> Stream for Bits<I>
            where
                I: Stream<Token = u8> + Clone,
            {
                type Token = bool;
                type Slice = (I::Slice, usize, usize);
                type IterOffsets = BitOffsets<I>;
                type Checkpoint = Checkpoint<Bits<I::Checkpoint>, Self>;
                #[inline(always)]
                fn iter_offsets(&self) -> Self::IterOffsets {
                    BitOffsets {
                        i: self.clone(),
                        o: 0,
                    }
                }
                #[inline(always)]
                fn eof_offset(&self) -> usize {
                    let offset = self.0.eof_offset() * 8;
                    if offset == 0 { 0 } else { offset - self.1 }
                }
                #[inline(always)]
                fn next_token(&mut self) -> Option<Self::Token> {
                    next_bit(self)
                }
                #[inline(always)]
                fn peek_token(&self) -> Option<Self::Token> {
                    peek_bit(self)
                }
                #[inline(always)]
                fn offset_for<P>(&self, predicate: P) -> Option<usize>
                where
                    P: Fn(Self::Token) -> bool,
                {
                    self.iter_offsets().find_map(|(o, b)| predicate(b).then_some(o))
                }
                #[inline(always)]
                fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
                    if let Some(needed) = tokens
                        .checked_sub(self.eof_offset())
                        .and_then(NonZeroUsize::new)
                    {
                        Err(Needed::Size(needed))
                    } else {
                        Ok(tokens)
                    }
                }
                #[inline(always)]
                fn next_slice(&mut self, offset: usize) -> Self::Slice {
                    let byte_offset = (offset + self.1) / 8;
                    let end_offset = (offset + self.1) % 8;
                    let s = self.0.next_slice(byte_offset);
                    let start_offset = self.1;
                    self.1 = end_offset;
                    (s, start_offset, end_offset)
                }
                #[inline(always)]
                fn peek_slice(&self, offset: usize) -> Self::Slice {
                    let byte_offset = (offset + self.1) / 8;
                    let end_offset = (offset + self.1) % 8;
                    let s = self.0.peek_slice(byte_offset);
                    let start_offset = self.1;
                    (s, start_offset, end_offset)
                }
                #[inline(always)]
                fn checkpoint(&self) -> Self::Checkpoint {
                    Checkpoint::<_, Self>::new(Bits(self.0.checkpoint(), self.1))
                }
                #[inline(always)]
                fn reset(&mut self, checkpoint: &Self::Checkpoint) {
                    self.0.reset(&checkpoint.inner.0);
                    self.1 = checkpoint.inner.1;
                }
                fn trace(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_fmt(format_args!("{0:#?}", self))
                }
            }
            /// Iterator for [bit][crate::binary::bits] stream ([`Bits`])
            pub struct BitOffsets<I> {
                i: Bits<I>,
                o: usize,
            }
            impl<I> Iterator for BitOffsets<I>
            where
                I: Stream<Token = u8> + Clone,
            {
                type Item = (usize, bool);
                fn next(&mut self) -> Option<Self::Item> {
                    let b = next_bit(&mut self.i)?;
                    let o = self.o;
                    self.o += 1;
                    Some((o, b))
                }
            }
            fn next_bit<I>(i: &mut Bits<I>) -> Option<bool>
            where
                I: Stream<Token = u8> + Clone,
            {
                if i.eof_offset() == 0 {
                    return None;
                }
                let offset = i.1;
                let mut next_i = i.0.clone();
                let byte = next_i.next_token()?;
                let bit = (byte >> offset) & 0x1 == 0x1;
                let next_offset = offset + 1;
                if next_offset == 8 {
                    i.0 = next_i;
                    i.1 = 0;
                } else {
                    i.1 = next_offset;
                }
                Some(bit)
            }
            fn peek_bit<I>(i: &Bits<I>) -> Option<bool>
            where
                I: Stream<Token = u8> + Clone,
            {
                if i.eof_offset() == 0 {
                    return None;
                }
                let offset = i.1;
                let mut next_i = i.0.clone();
                let byte = next_i.next_token()?;
                Some((byte >> offset) & 0x1 == 0x1)
            }
            impl<I> StreamIsPartial for Bits<I>
            where
                I: StreamIsPartial,
            {
                type PartialState = I::PartialState;
                #[inline]
                fn complete(&mut self) -> Self::PartialState {
                    self.0.complete()
                }
                #[inline]
                fn restore_partial(&mut self, state: Self::PartialState) {
                    self.0.restore_partial(state);
                }
                #[inline(always)]
                fn is_partial_supported() -> bool {
                    I::is_partial_supported()
                }
                #[inline(always)]
                fn is_partial(&self) -> bool {
                    self.0.is_partial()
                }
            }
            impl<I> Offset for Bits<I>
            where
                I: Offset,
            {
                #[inline(always)]
                fn offset_from(&self, start: &Self) -> usize {
                    self.0.offset_from(&start.0) * 8 + self.1 - start.1
                }
            }
            impl<I> Offset<<Bits<I> as Stream>::Checkpoint> for Bits<I>
            where
                I: Stream<Token = u8> + Clone,
            {
                #[inline(always)]
                fn offset_from(&self, other: &<Bits<I> as Stream>::Checkpoint) -> usize {
                    self.checkpoint().offset_from(other)
                }
            }
            impl<I: Clone> crate::error::ErrorConvert<crate::error::InputError<Bits<I>>>
            for crate::error::InputError<I> {
                #[inline]
                fn convert(self) -> crate::error::InputError<Bits<I>> {
                    self.map_input(|i| Bits(i, 0))
                }
            }
            impl<I: Clone> crate::error::ErrorConvert<crate::error::InputError<I>>
            for crate::error::InputError<Bits<I>> {
                #[inline]
                fn convert(self) -> crate::error::InputError<I> {
                    self.map_input(|Bits(i, _o)| i)
                }
            }
        }
        pub use self::stream::BitOffsets;
        pub use self::stream::Bits;
        use crate::combinator::trace;
        use crate::error::{ErrorConvert, Needed, ParserError};
        use crate::stream::{Stream, StreamIsPartial, ToUsize};
        use crate::{Parser, Result};
        use core::ops::{AddAssign, Div, Shl, Shr};
        /// Number of bits in a byte
        const BYTE: usize = u8::BITS as usize;
        /// Converts a byte-level input to a bit-level input
        ///
        /// See [`bytes`] to convert it back.
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::Bytes;
        /// # use winnow::binary::bits::{bits, take};
        /// # use winnow::error::ContextError;
        /// # use winnow::error::ErrMode;
        /// type Stream<'i> = &'i Bytes;
        ///
        /// fn stream(b: &[u8]) -> Stream<'_> {
        ///     Bytes::new(b)
        /// }
        ///
        /// fn parse(input: &mut Stream<'_>) -> ModalResult<(u8, u8)> {
        ///     bits::<_, _, ErrMode<ContextError>, _, _>((take(4usize), take(8usize))).parse_next(input)
        /// }
        ///
        /// let input = stream(&[0x12, 0x34, 0xff, 0xff]);
        ///
        /// let output = parse.parse_peek(input).expect("We take 1.5 bytes and the input is longer than 2 bytes");
        ///
        /// // The first byte is consumed, the second byte is partially consumed and dropped.
        /// let remaining = output.0;
        /// assert_eq!(remaining, stream(&[0xff, 0xff]));
        ///
        /// let parsed = output.1;
        /// assert_eq!(parsed.0, 0x01);
        /// assert_eq!(parsed.1, 0x23);
        /// ```
        pub fn bits<Input, Output, BitError, ByteError, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, Output, ByteError>
        where
            BitError: ParserError<Bits<Input>> + ErrorConvert<ByteError>,
            ByteError: ParserError<Input>,
            Bits<Input>: Stream,
            Input: Stream + Clone,
            ParseNext: Parser<Bits<Input>, Output, BitError>,
        {
            trace(
                "bits",
                move |input: &mut Input| {
                    let mut bit_input = Bits(input.clone(), 0);
                    match parser.parse_next(&mut bit_input) {
                        Ok(result) => {
                            let Bits(mut rest, offset) = bit_input;
                            let remaining_bytes_index = offset / BYTE
                                + if offset % BYTE == 0 { 0 } else { 1 };
                            let _ = rest.next_slice(remaining_bytes_index);
                            *input = rest;
                            Ok(result)
                        }
                        Err(e) => {
                            match e.needed() {
                                Some(n) => {
                                    Err(
                                        ParserError::incomplete(
                                            input,
                                            n.map(|u| u.get() / BYTE + 1),
                                        ),
                                    )
                                }
                                None => Err(ErrorConvert::convert(e)),
                            }
                        }
                    }
                },
            )
        }
        /// Convert a [`bits`] stream back into a byte stream
        ///
        /// <div class="warning">
        ///
        /// **Warning:** A partial byte remaining in the input will be ignored and the given parser will
        /// start parsing at the next full byte.
        ///
        /// </div>
        ///
        /// # Examples
        ///
        /// ```
        /// # use winnow::prelude::*;
        /// # use winnow::Bytes;
        /// # use winnow::token::rest;
        /// # use winnow::error::ContextError;
        /// # use winnow::error::ErrMode;
        /// use winnow::binary::bits::{bits, bytes, take};
        ///
        /// type Stream<'i> = &'i Bytes;
        ///
        /// fn stream(b: &[u8]) -> Stream<'_> {
        ///     Bytes::new(b)
        /// }
        ///
        /// fn parse<'i>(input: &mut Stream<'i>) -> ModalResult<(u8, u8, &'i [u8])> {
        ///   bits::<_, _, ErrMode<ContextError>, _, _>((
        ///     take(4usize),
        ///     take(8usize),
        ///     bytes::<_, _, ErrMode<ContextError>, _, _>(rest)
        ///   )).parse_next(input)
        /// }
        ///
        /// let input = stream(&[0x12, 0x34, 0xff, 0xff]);
        ///
        /// assert_eq!(parse.parse_peek(input), Ok(( stream(&[]), (0x01, 0x23, &[0xff, 0xff][..]) )));
        /// ```
        pub fn bytes<Input, Output, ByteError, BitError, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Bits<Input>, Output, BitError>
        where
            ByteError: ParserError<Input> + ErrorConvert<BitError>,
            BitError: ParserError<Bits<Input>>,
            Input: Stream<Token = u8> + Clone,
            ParseNext: Parser<Input, Output, ByteError>,
        {
            trace(
                "bytes",
                move |bit_input: &mut Bits<Input>| {
                    let Bits(mut input, offset) = bit_input.clone();
                    let _ = if offset % BYTE != 0 {
                        input.next_slice(1 + offset / BYTE)
                    } else {
                        input.next_slice(offset / BYTE)
                    };
                    match parser.parse_next(&mut input) {
                        Ok(res) => {
                            *bit_input = Bits(input, 0);
                            Ok(res)
                        }
                        Err(e) => {
                            match e.needed() {
                                Some(Needed::Unknown) => {
                                    Err(ParserError::incomplete(bit_input, Needed::Unknown))
                                }
                                Some(Needed::Size(sz)) => {
                                    Err(
                                        match sz.get().checked_mul(BYTE) {
                                            Some(v) => {
                                                ParserError::incomplete(bit_input, Needed::new(v))
                                            }
                                            None => {
                                                ParserError::assert(
                                                    bit_input,
                                                    "overflow in turning needed bytes into needed bits",
                                                )
                                            }
                                        },
                                    )
                                }
                                None => Err(ErrorConvert::convert(e)),
                            }
                        }
                    }
                },
            )
        }
        /// Parse taking `count` bits
        ///
        /// # Effective Signature
        ///
        /// Assuming you are parsing a [`Bits<&[u8]>`][Bits] bit [Stream]:
        /// ```rust
        /// # use winnow::prelude::*;;
        /// # use winnow::error::ContextError;
        /// # use winnow::binary::bits::Bits;
        /// pub fn take<'i>(count: usize) -> impl Parser<Bits<&'i [u8]>, u8, ContextError>
        /// # {
        /// #     winnow::binary::bits::take(count)
        /// # }
        /// ```
        ///
        /// # Example
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::Bytes;
        /// # use winnow::error::ContextError;
        /// use winnow::binary::bits::{Bits, take};
        ///
        /// type Stream<'i> = &'i Bytes;
        ///
        /// fn stream(b: &[u8]) -> Stream<'_> {
        ///     Bytes::new(b)
        /// }
        ///
        /// // Consumes 0 bits, returns 0
        /// assert_eq!(take::<_, usize, _, ContextError>(0usize).parse_peek(Bits(stream(&[0b00010010]), 0)), Ok((Bits(stream(&[0b00010010]), 0), 0)));
        ///
        /// // Consumes 4 bits, returns their values and increase offset to 4
        /// assert_eq!(take::<_, usize, _, ContextError>(4usize).parse_peek(Bits(stream(&[0b00010010]), 0)), Ok((Bits(stream(&[0b00010010]), 4), 0b00000001)));
        ///
        /// // Consumes 4 bits, offset is 4, returns their values and increase offset to 0 of next byte
        /// assert_eq!(take::<_, usize, _, ContextError>(4usize).parse_peek(Bits(stream(&[0b00010010]), 4)), Ok((Bits(stream(&[]), 0), 0b00000010)));
        ///
        /// // Tries to consume 12 bits but only 8 are available
        /// assert!(take::<_, usize, _, ContextError>(12usize).parse_peek(Bits(stream(&[0b00010010]), 0)).is_err());
        /// ```
        #[inline(always)]
        pub fn take<Input, Output, Count, Error>(
            count: Count,
        ) -> impl Parser<Bits<Input>, Output, Error>
        where
            Input: Stream<Token = u8> + StreamIsPartial + Clone,
            Output: From<u8> + AddAssign + Shl<usize, Output = Output>
                + Shr<usize, Output = Output>,
            Count: ToUsize,
            Error: ParserError<Bits<Input>>,
        {
            let count = count.to_usize();
            trace(
                "take",
                move |input: &mut Bits<Input>| {
                    if <Input as StreamIsPartial>::is_partial_supported() {
                        take_::<_, _, _, true>(input, count)
                    } else {
                        take_::<_, _, _, false>(input, count)
                    }
                },
            )
        }
        fn take_<I, O, E: ParserError<Bits<I>>, const PARTIAL: bool>(
            bit_input: &mut Bits<I>,
            count: usize,
        ) -> Result<O, E>
        where
            I: StreamIsPartial,
            I: Stream<Token = u8> + Clone,
            O: From<u8> + AddAssign + Shl<usize, Output = O> + Shr<usize, Output = O>,
        {
            if count == 0 {
                Ok(0u8.into())
            } else {
                let Bits(mut input, bit_offset) = bit_input.clone();
                if input.eof_offset() * BYTE < count + bit_offset {
                    if PARTIAL && input.is_partial() {
                        Err(ParserError::incomplete(bit_input, Needed::new(count)))
                    } else {
                        Err(ParserError::from_input(&Bits(input, bit_offset)))
                    }
                } else {
                    let cnt = (count + bit_offset).div(BYTE);
                    let mut acc: O = 0_u8.into();
                    let mut offset: usize = bit_offset;
                    let mut remaining: usize = count;
                    let mut end_offset: usize = 0;
                    for (_, byte) in input.iter_offsets().take(cnt + 1) {
                        if remaining == 0 {
                            break;
                        }
                        let val: O = if offset == 0 {
                            byte.into()
                        } else {
                            (byte << offset >> offset).into()
                        };
                        if remaining < BYTE - offset {
                            acc += val >> (BYTE - offset - remaining);
                            end_offset = remaining + offset;
                            break;
                        } else {
                            acc += val << (remaining - (BYTE - offset));
                            remaining -= BYTE - offset;
                            offset = 0;
                        }
                    }
                    let _ = input.next_slice(cnt);
                    *bit_input = Bits(input, end_offset);
                    Ok(acc)
                }
            }
        }
        /// Parse taking `count` bits and comparing them to `pattern`
        ///
        /// # Effective Signature
        ///
        /// Assuming you are parsing a [`Bits<&[u8]>`][Bits] bit [Stream]:
        /// ```rust
        /// # use winnow::prelude::*;;
        /// # use winnow::error::ContextError;
        /// # use winnow::binary::bits::Bits;
        /// pub fn pattern<'i>(pattern: u8, count: usize) -> impl Parser<Bits<&'i [u8]>, u8, ContextError>
        /// # {
        /// #     winnow::binary::bits::pattern(pattern, count)
        /// # }
        /// ```
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::Bytes;
        /// # use winnow::error::ContextError;
        /// use winnow::binary::bits::pattern;
        ///
        /// type Stream<'i> = &'i Bytes;
        ///
        /// fn stream(b: &[u8]) -> Stream<'_> {
        ///     Bytes::new(b)
        /// }
        ///
        /// /// Compare the lowest `count` bits of `input` against the lowest `count` bits of `pattern`.
        /// /// Return Ok and the matching section of `input` if there's a match.
        /// /// Return Err if there's no match.
        /// # use winnow::binary::bits::Bits;
        /// fn parser(bits: u8, count: u8, input: &mut Bits<Stream<'_>>) -> ModalResult<u8> {
        ///     pattern(bits, count).parse_next(input)
        /// }
        ///
        /// // The lowest 4 bits of 0b00001111 match the lowest 4 bits of 0b11111111.
        /// assert_eq!(
        ///     pattern::<_, usize, _, ContextError>(0b0000_1111, 4usize).parse_peek(Bits(stream(&[0b1111_1111]), 0)),
        ///     Ok((Bits(stream(&[0b1111_1111]), 4), 0b0000_1111))
        /// );
        ///
        /// // The lowest bit of 0b00001111 matches the lowest bit of 0b11111111 (both are 1).
        /// assert_eq!(
        ///     pattern::<_, usize, _, ContextError>(0b00000001, 1usize).parse_peek(Bits(stream(&[0b11111111]), 0)),
        ///     Ok((Bits(stream(&[0b11111111]), 1), 0b00000001))
        /// );
        ///
        /// // The lowest 2 bits of 0b11111111 and 0b00000001 are different.
        /// assert!(pattern::<_, usize, _, ContextError>(0b000000_01, 2usize).parse_peek(Bits(stream(&[0b111111_11]), 0)).is_err());
        ///
        /// // The lowest 8 bits of 0b11111111 and 0b11111110 are different.
        /// assert!(pattern::<_, usize, _, ContextError>(0b11111110, 8usize).parse_peek(Bits(stream(&[0b11111111]), 0)).is_err());
        /// ```
        #[inline(always)]
        #[doc(alias = "literal")]
        #[doc(alias = "just")]
        #[doc(alias = "tag")]
        pub fn pattern<Input, Output, Count, Error: ParserError<Bits<Input>>>(
            pattern: Output,
            count: Count,
        ) -> impl Parser<Bits<Input>, Output, Error>
        where
            Input: Stream<Token = u8> + StreamIsPartial + Clone,
            Count: ToUsize,
            Output: From<u8> + AddAssign + Shl<usize, Output = Output>
                + Shr<usize, Output = Output> + PartialEq,
        {
            let count = count.to_usize();
            trace(
                "pattern",
                move |input: &mut Bits<Input>| {
                    let start = input.checkpoint();
                    take(count)
                        .parse_next(input)
                        .and_then(|o| {
                            if pattern == o {
                                Ok(o)
                            } else {
                                input.reset(&start);
                                Err(ParserError::from_input(input))
                            }
                        })
                },
            )
        }
        /// Parses one specific bit as a bool.
        ///
        /// # Effective Signature
        ///
        /// Assuming you are parsing a [`Bits<&[u8]>`][Bits] bit [Stream]:
        /// ```rust
        /// # use winnow::prelude::*;;
        /// # use winnow::error::ContextError;
        /// # use winnow::binary::bits::Bits;
        /// pub fn bool(input: &mut Bits<&[u8]>) -> ModalResult<bool>
        /// # {
        /// #     winnow::binary::bits::bool.parse_next(input)
        /// # }
        /// ```
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::Bytes;
        /// # use winnow::error::InputError;
        /// use winnow::binary::bits::bool;
        ///
        /// type Stream<'i> = &'i Bytes;
        ///
        /// fn stream(b: &[u8]) -> Stream<'_> {
        ///     Bytes::new(b)
        /// }
        ///
        /// # use winnow::binary::bits::Bits;
        /// fn parse(input: &mut Bits<Stream<'_>>) -> ModalResult<bool> {
        ///     bool.parse_next(input)
        /// }
        ///
        /// assert_eq!(parse.parse_peek(Bits(stream(&[0b10000000]), 0)), Ok((Bits(stream(&[0b10000000]), 1), true)));
        /// assert_eq!(parse.parse_peek(Bits(stream(&[0b10000000]), 1)), Ok((Bits(stream(&[0b10000000]), 2), false)));
        /// ```
        #[doc(alias = "any")]
        pub fn bool<Input, Error: ParserError<Bits<Input>>>(
            input: &mut Bits<Input>,
        ) -> Result<bool, Error>
        where
            Input: Stream<Token = u8> + StreamIsPartial + Clone,
        {
            trace(
                    "bool",
                    |input: &mut Bits<Input>| {
                        let bit: u32 = take(1usize).parse_next(input)?;
                        Ok(bit != 0)
                    },
                )
                .parse_next(input)
        }
    }
    use crate::combinator::repeat;
    use crate::combinator::trace;
    use crate::error::Needed;
    use crate::error::ParserError;
    use crate::stream::Accumulate;
    use crate::stream::{Stream, StreamIsPartial};
    use crate::stream::{ToUsize, UpdateSlice};
    use crate::Parser;
    use crate::Result;
    use core::ops::{Add, Shl};
    /// Configurable endianness
    pub enum Endianness {
        /// Big endian
        Big,
        /// Little endian
        Little,
        /// Will match the host's endianness
        Native,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Endianness {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    Endianness::Big => "Big",
                    Endianness::Little => "Little",
                    Endianness::Native => "Native",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Endianness {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Endianness {
        #[inline]
        fn eq(&self, other: &Endianness) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Endianness {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Endianness {}
    #[automatically_derived]
    impl ::core::clone::Clone for Endianness {
        #[inline]
        fn clone(&self) -> Endianness {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Endianness {}
    /// Recognizes an unsigned 1 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u8> {
    ///     be_u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u8> {
    ///     be_u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"\x01abcd"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn be_u8<Input, Error>(input: &mut Input) -> Result<u8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        u8(input)
    }
    /// Recognizes a big endian unsigned 2 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u16;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u16> {
    ///     be_u16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u16;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u16> {
    ///     be_u16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0001)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn be_u16<Input, Error>(input: &mut Input) -> Result<u16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("be_u16", move |input: &mut Input| be_uint(input, 2)).parse_next(input)
    }
    /// Recognizes a big endian unsigned 3 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u24;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u32> {
    ///     be_u24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u24;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     be_u24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x000102)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn be_u24<Input, Error>(input: &mut Input) -> Result<u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("be_u23", move |input: &mut Input| be_uint(input, 3)).parse_next(input)
    }
    /// Recognizes a big endian unsigned 4 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u32> {
    ///     be_u32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     be_u32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x00010203)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn be_u32<Input, Error>(input: &mut Input) -> Result<u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("be_u32", move |input: &mut Input| be_uint(input, 4)).parse_next(input)
    }
    /// Recognizes a big endian unsigned 8 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u64> {
    ///     be_u64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u64> {
    ///     be_u64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0001020304050607)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn be_u64<Input, Error>(input: &mut Input) -> Result<u64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("be_u64", move |input: &mut Input| be_uint(input, 8)).parse_next(input)
    }
    /// Recognizes a big endian unsigned 16 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_u128;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u128> {
    ///     be_u128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_u128;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u128> {
    ///     be_u128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x00010203040506070809101112131415)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn be_u128<Input, Error>(input: &mut Input) -> Result<u128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("be_u128", move |input: &mut Input| be_uint(input, 16)).parse_next(input)
    }
    #[inline]
    fn be_uint<Input, Uint, Error>(
        input: &mut Input,
        bound: usize,
    ) -> Result<Uint, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Uint: Default + Shl<u8, Output = Uint> + Add<Uint, Output = Uint> + From<u8>,
        Error: ParserError<Input>,
    {
        if true {
            match (&(bound), &(1)) {
                (left_val, right_val) => {
                    if *left_val == *right_val {
                        let kind = ::core::panicking::AssertKind::Ne;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!(
                                    "to_be_uint needs extra work to avoid overflow",
                                ),
                            ),
                        );
                    }
                }
            };
        }
        match input.offset_at(bound) {
            Ok(offset) => {
                let res = to_be_uint(input, offset);
                input.next_slice(offset);
                Ok(res)
            }
            Err(
                e,
            ) if <Input as StreamIsPartial>::is_partial_supported()
                && input.is_partial() => Err(ParserError::incomplete(input, e)),
            Err(_needed) => Err(ParserError::from_input(input)),
        }
    }
    #[inline]
    fn to_be_uint<Input, Uint>(number: &Input, offset: usize) -> Uint
    where
        Input: Stream,
        Uint: Default + Shl<u8, Output = Uint> + Add<Uint, Output = Uint>
            + From<<Input as Stream>::Token>,
    {
        let mut res = Uint::default();
        for (_, byte) in number.iter_offsets().take(offset) {
            res = (res << 8) + byte.into();
        }
        res
    }
    /// Recognizes a signed 1 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i8> {
    ///     be_i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i8> {
    ///       be_i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"\x01abcd"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn be_i8<Input, Error>(input: &mut Input) -> Result<i8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        i8(input)
    }
    /// Recognizes a big endian signed 2 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i16;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i16> {
    ///     be_i16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i16;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i16> {
    ///       be_i16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0001)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn be_i16<Input, Error>(input: &mut Input) -> Result<i16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_i16",
                move |input: &mut Input| {
                    be_uint::<_, u16, _>(input, 2).map(|n| n as i16)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a big endian signed 3 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i24;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i32> {
    ///     be_i24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i24;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///       be_i24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x000102)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn be_i24<Input, Error>(input: &mut Input) -> Result<i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_i24",
                move |input: &mut Input| {
                    be_uint::<_, u32, _>(input, 3)
                        .map(|n| {
                            let n = if n & 0x80_00_00 != 0 {
                                (n | 0xff_00_00_00) as i32
                            } else {
                                n as i32
                            };
                            n
                        })
                },
            )
            .parse_next(input)
    }
    /// Recognizes a big endian signed 4 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i32> {
    ///       be_i32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///       be_i32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x00010203)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(4))));
    /// ```
    #[inline(always)]
    pub fn be_i32<Input, Error>(input: &mut Input) -> Result<i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_i32",
                move |input: &mut Input| {
                    be_uint::<_, u32, _>(input, 4).map(|n| n as i32)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a big endian signed 8 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i64> {
    ///       be_i64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i64> {
    ///       be_i64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0001020304050607)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn be_i64<Input, Error>(input: &mut Input) -> Result<i64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_i64",
                move |input: &mut Input| {
                    be_uint::<_, u64, _>(input, 8).map(|n| n as i64)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a big endian signed 16 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_i128;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i128> {
    ///       be_i128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_i128;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i128> {
    ///       be_i128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x00010203040506070809101112131415)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn be_i128<Input, Error>(input: &mut Input) -> Result<i128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_i128",
                move |input: &mut Input| {
                    be_uint::<_, u128, _>(input, 16).map(|n| n as i128)
                },
            )
            .parse_next(input)
    }
    /// Recognizes an unsigned 1 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u8> {
    ///       le_u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u8> {
    ///       le_u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"\x01abcd"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn le_u8<Input, Error>(input: &mut Input) -> Result<u8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        u8(input)
    }
    /// Recognizes a little endian unsigned 2 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u16;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u16> {
    ///       le_u16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u16;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u16> {
    ///       le_u16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn le_u16<Input, Error>(input: &mut Input) -> Result<u16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("le_u16", move |input: &mut Input| le_uint(input, 2)).parse_next(input)
    }
    /// Recognizes a little endian unsigned 3 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u24;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u32> {
    ///       le_u24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u24;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///       le_u24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn le_u24<Input, Error>(input: &mut Input) -> Result<u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("le_u24", move |input: &mut Input| le_uint(input, 3)).parse_next(input)
    }
    /// Recognizes a little endian unsigned 4 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u32> {
    ///       le_u32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///       le_u32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x03020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn le_u32<Input, Error>(input: &mut Input) -> Result<u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("le_u32", move |input: &mut Input| le_uint(input, 4)).parse_next(input)
    }
    /// Recognizes a little endian unsigned 8 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u64> {
    ///       le_u64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u64> {
    ///       le_u64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0706050403020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn le_u64<Input, Error>(input: &mut Input) -> Result<u64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("le_u64", move |input: &mut Input| le_uint(input, 8)).parse_next(input)
    }
    /// Recognizes a little endian unsigned 16 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_u128;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u128> {
    ///       le_u128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_u128;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u128> {
    ///       le_u128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x15141312111009080706050403020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn le_u128<Input, Error>(input: &mut Input) -> Result<u128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace("le_u128", move |input: &mut Input| le_uint(input, 16)).parse_next(input)
    }
    #[inline]
    fn le_uint<Input, Uint, Error>(
        input: &mut Input,
        bound: usize,
    ) -> Result<Uint, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Uint: Default + Shl<u8, Output = Uint> + Add<Uint, Output = Uint> + From<u8>,
        Error: ParserError<Input>,
    {
        match input.offset_at(bound) {
            Ok(offset) => {
                let res = to_le_uint(input, offset);
                input.next_slice(offset);
                Ok(res)
            }
            Err(
                e,
            ) if <Input as StreamIsPartial>::is_partial_supported()
                && input.is_partial() => Err(ParserError::incomplete(input, e)),
            Err(_needed) => Err(ParserError::from_input(input)),
        }
    }
    #[inline]
    fn to_le_uint<Input, Uint>(number: &Input, offset: usize) -> Uint
    where
        Input: Stream,
        Uint: Default + Shl<u8, Output = Uint> + Add<Uint, Output = Uint>
            + From<<Input as Stream>::Token>,
    {
        let mut res = Uint::default();
        for (index, byte) in number.iter_offsets().take(offset) {
            res = res + (Uint::from(byte) << (8 * index as u8));
        }
        res
    }
    /// Recognizes a signed 1 byte integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i8> {
    ///       le_i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i8> {
    ///       le_i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"\x01abcd"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn le_i8<Input, Error>(input: &mut Input) -> Result<i8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        i8(input)
    }
    /// Recognizes a little endian signed 2 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i16;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i16> {
    ///       le_i16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i16;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i16> {
    ///       le_i16.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn le_i16<Input, Error>(input: &mut Input) -> Result<i16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_i16",
                move |input: &mut Input| {
                    le_uint::<_, u16, _>(input, 2).map(|n| n as i16)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian signed 3 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i24;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i32> {
    ///       le_i24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i24;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///       le_i24.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn le_i24<Input, Error>(input: &mut Input) -> Result<i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_i24",
                move |input: &mut Input| {
                    le_uint::<_, u32, _>(input, 3)
                        .map(|n| {
                            let n = if n & 0x80_00_00 != 0 {
                                (n | 0xff_00_00_00) as i32
                            } else {
                                n as i32
                            };
                            n
                        })
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian signed 4 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i32> {
    ///       le_i32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///       le_i32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x03020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn le_i32<Input, Error>(input: &mut Input) -> Result<i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_i32",
                move |input: &mut Input| {
                    le_uint::<_, u32, _>(input, 4).map(|n| n as i32)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian signed 8 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i64> {
    ///       le_i64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i64> {
    ///       le_i64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x0706050403020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn le_i64<Input, Error>(input: &mut Input) -> Result<i64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_i64",
                move |input: &mut Input| {
                    le_uint::<_, u64, _>(input, 8).map(|n| n as i64)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian signed 16 bytes integer.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_i128;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i128> {
    ///       le_i128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
    /// assert!(parser.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_i128;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i128> {
    ///       le_i128.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10\x11\x12\x13\x14\x15abcd"[..])), Ok((Partial::new(&b"abcd"[..]), 0x15141312111009080706050403020100)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn le_i128<Input, Error>(input: &mut Input) -> Result<i128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_i128",
                move |input: &mut Input| {
                    le_uint::<_, u128, _>(input, 16).map(|n| n as i128)
                },
            )
            .parse_next(input)
    }
    /// Recognizes an unsigned 1 byte integer
    ///
    /// <div class="warning">
    ///
    /// **Note:** that endianness does not apply to 1 byte numbers.
    ///
    /// </div>
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<u8> {
    ///       u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<u8> {
    ///       u8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"\x03abcefg"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn u8<Input, Error>(input: &mut Input) -> Result<u8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "u8",
                move |input: &mut Input| {
                    if <Input as StreamIsPartial>::is_partial_supported() {
                        u8_::<_, _, true>(input)
                    } else {
                        u8_::<_, _, false>(input)
                    }
                },
            )
            .parse_next(input)
    }
    fn u8_<Input, Error, const PARTIAL: bool>(input: &mut Input) -> Result<u8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        input
            .next_token()
            .ok_or_else(|| {
                if PARTIAL && input.is_partial() {
                    ParserError::incomplete(input, Needed::new(1))
                } else {
                    ParserError::from_input(input)
                }
            })
    }
    /// Recognizes an unsigned 2 bytes integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian u16 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian u16 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u16;
    ///
    /// fn be_u16(input: &mut &[u8]) -> ModalResult<u16> {
    ///     u16(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u16.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
    /// assert!(be_u16.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_u16(input: &mut &[u8]) -> ModalResult<u16> {
    ///     u16(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u16.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
    /// assert!(le_u16.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u16;
    ///
    /// fn be_u16(input: &mut Partial<&[u8]>) -> ModalResult<u16> {
    ///     u16(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u16.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0003)));
    /// assert_eq!(be_u16.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    ///
    /// fn le_u16(input: &mut Partial<&[u8]>) -> ModalResult< u16> {
    ///     u16(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u16.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0300)));
    /// assert_eq!(le_u16.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn u16<Input, Error>(endian: Endianness) -> impl Parser<Input, u16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_u16,
                Endianness::Little => le_u16,
                Endianness::Native => le_u16,
            }
        }(input)
    }
    /// Recognizes an unsigned 3 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian u24 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian u24 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u24;
    ///
    /// fn be_u24(input: &mut &[u8]) -> ModalResult<u32> {
    ///     u24(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u24.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
    /// assert!(be_u24.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_u24(input: &mut &[u8]) -> ModalResult<u32> {
    ///     u24(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u24.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
    /// assert!(le_u24.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u24;
    ///
    /// fn be_u24(input: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     u24(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u24.parse_peek(Partial::new(&b"\x00\x03\x05abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x000305)));
    /// assert_eq!(be_u24.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    ///
    /// fn le_u24(input: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     u24(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u24.parse_peek(Partial::new(&b"\x00\x03\x05abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x050300)));
    /// assert_eq!(le_u24.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn u24<Input, Error>(endian: Endianness) -> impl Parser<Input, u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_u24,
                Endianness::Little => le_u24,
                Endianness::Native => le_u24,
            }
        }(input)
    }
    /// Recognizes an unsigned 4 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian u32 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian u32 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u32;
    ///
    /// fn be_u32(input: &mut &[u8]) -> ModalResult<u32> {
    ///     u32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u32.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
    /// assert!(be_u32.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_u32(input: &mut &[u8]) -> ModalResult<u32> {
    ///     u32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u32.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
    /// assert!(le_u32.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u32;
    ///
    /// fn be_u32(input: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     u32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u32.parse_peek(Partial::new(&b"\x00\x03\x05\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x00030507)));
    /// assert_eq!(be_u32.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    ///
    /// fn le_u32(input: &mut Partial<&[u8]>) -> ModalResult<u32> {
    ///     u32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u32.parse_peek(Partial::new(&b"\x00\x03\x05\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x07050300)));
    /// assert_eq!(le_u32.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn u32<Input, Error>(endian: Endianness) -> impl Parser<Input, u32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_u32,
                Endianness::Little => le_u32,
                Endianness::Native => le_u32,
            }
        }(input)
    }
    /// Recognizes an unsigned 8 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian u64 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian u64 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u64;
    ///
    /// fn be_u64(input: &mut &[u8]) -> ModalResult<u64> {
    ///     u64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u64.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
    /// assert!(be_u64.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_u64(input: &mut &[u8]) -> ModalResult<u64> {
    ///     u64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u64.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
    /// assert!(le_u64.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u64;
    ///
    /// fn be_u64(input: &mut Partial<&[u8]>) -> ModalResult<u64> {
    ///     u64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u64.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0001020304050607)));
    /// assert_eq!(be_u64.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    ///
    /// fn le_u64(input: &mut Partial<&[u8]>) -> ModalResult<u64> {
    ///     u64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u64.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0706050403020100)));
    /// assert_eq!(le_u64.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn u64<Input, Error>(endian: Endianness) -> impl Parser<Input, u64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_u64,
                Endianness::Little => le_u64,
                Endianness::Native => le_u64,
            }
        }(input)
    }
    /// Recognizes an unsigned 16 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian u128 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian u128 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::u128;
    ///
    /// fn be_u128(input: &mut &[u8]) -> ModalResult<u128> {
    ///     u128(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u128.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
    /// assert!(be_u128.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_u128(input: &mut &[u8]) -> ModalResult<u128> {
    ///     u128(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u128.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
    /// assert!(le_u128.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::u128;
    ///
    /// fn be_u128(input: &mut Partial<&[u8]>) -> ModalResult<u128> {
    ///     u128(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_u128.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x00010203040506070001020304050607)));
    /// assert_eq!(be_u128.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    ///
    /// fn le_u128(input: &mut Partial<&[u8]>) -> ModalResult<u128> {
    ///     u128(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_u128.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x07060504030201000706050403020100)));
    /// assert_eq!(le_u128.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn u128<Input, Error>(endian: Endianness) -> impl Parser<Input, u128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_u128,
                Endianness::Little => le_u128,
                Endianness::Native => le_u128,
            }
        }(input)
    }
    /// Recognizes a signed 1 byte integer
    ///
    /// <div class="warning">
    ///
    /// **Note:** that endianness does not apply to 1 byte numbers.
    ///
    /// </div>
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i8;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<i8> {
    ///       i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"\x03abcefg"[..], 0x00)));
    /// assert!(parser.parse_peek(&b""[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i8;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<i8> {
    ///       i8.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"\x03abcefg"[..]), 0x00)));
    /// assert_eq!(parser.parse_peek(Partial::new(&b""[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn i8<Input, Error>(input: &mut Input) -> Result<i8, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "i8",
                move |input: &mut Input| {
                    if <Input as StreamIsPartial>::is_partial_supported() {
                        u8_::<_, _, true>(input)
                    } else {
                        u8_::<_, _, false>(input)
                    }
                        .map(|n| n as i8)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a signed 2 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian i16 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian i16 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i16;
    ///
    /// fn be_i16(input: &mut &[u8]) -> ModalResult<i16> {
    ///     i16(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i16.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0003)));
    /// assert!(be_i16.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_i16(input: &mut &[u8]) -> ModalResult<i16> {
    ///     i16(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i16.parse_peek(&b"\x00\x03abcefg"[..]), Ok((&b"abcefg"[..], 0x0300)));
    /// assert!(le_i16.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i16;
    ///
    /// fn be_i16(input: &mut Partial<&[u8]>) -> ModalResult<i16> {
    ///     i16(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i16.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0003)));
    /// assert_eq!(be_i16.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    ///
    /// fn le_i16(input: &mut Partial<&[u8]>) -> ModalResult<i16> {
    ///     i16(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i16.parse_peek(Partial::new(&b"\x00\x03abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0300)));
    /// assert_eq!(le_i16.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn i16<Input, Error>(endian: Endianness) -> impl Parser<Input, i16, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_i16,
                Endianness::Little => le_i16,
                Endianness::Native => le_i16,
            }
        }(input)
    }
    /// Recognizes a signed 3 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian i24 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian i24 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i24;
    ///
    /// fn be_i24(input: &mut &[u8]) -> ModalResult<i32> {
    ///     i24(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i24.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x000305)));
    /// assert!(be_i24.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_i24(input: &mut &[u8]) -> ModalResult<i32> {
    ///     i24(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i24.parse_peek(&b"\x00\x03\x05abcefg"[..]), Ok((&b"abcefg"[..], 0x050300)));
    /// assert!(le_i24.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i24;
    ///
    /// fn be_i24(input: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///     i24(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i24.parse_peek(Partial::new(&b"\x00\x03\x05abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x000305)));
    /// assert_eq!(be_i24.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    ///
    /// fn le_i24(input: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///     i24(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i24.parse_peek(Partial::new(&b"\x00\x03\x05abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x050300)));
    /// assert_eq!(le_i24.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    #[inline(always)]
    pub fn i24<Input, Error>(endian: Endianness) -> impl Parser<Input, i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_i24,
                Endianness::Little => le_i24,
                Endianness::Native => le_i24,
            }
        }(input)
    }
    /// Recognizes a signed 4 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian i32 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian i32 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i32;
    ///
    /// fn be_i32(input: &mut &[u8]) -> ModalResult<i32> {
    ///     i32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i32.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00030507)));
    /// assert!(be_i32.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_i32(input: &mut &[u8]) -> ModalResult<i32> {
    ///     i32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i32.parse_peek(&b"\x00\x03\x05\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07050300)));
    /// assert!(le_i32.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i32;
    ///
    /// fn be_i32(input: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///     i32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i32.parse_peek(Partial::new(&b"\x00\x03\x05\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x00030507)));
    /// assert_eq!(be_i32.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    ///
    /// fn le_i32(input: &mut Partial<&[u8]>) -> ModalResult<i32> {
    ///     i32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i32.parse_peek(Partial::new(&b"\x00\x03\x05\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x07050300)));
    /// assert_eq!(le_i32.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn i32<Input, Error>(endian: Endianness) -> impl Parser<Input, i32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_i32,
                Endianness::Little => le_i32,
                Endianness::Native => le_i32,
            }
        }(input)
    }
    /// Recognizes a signed 8 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian i64 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian i64 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i64;
    ///
    /// fn be_i64(input: &mut &[u8]) -> ModalResult<i64> {
    ///     i64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i64.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0001020304050607)));
    /// assert!(be_i64.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_i64(input: &mut &[u8]) -> ModalResult<i64> {
    ///     i64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i64.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x0706050403020100)));
    /// assert!(le_i64.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i64;
    ///
    /// fn be_i64(input: &mut Partial<&[u8]>) -> ModalResult<i64> {
    ///     i64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i64.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0001020304050607)));
    /// assert_eq!(be_i64.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    ///
    /// fn le_i64(input: &mut Partial<&[u8]>) -> ModalResult<i64> {
    ///     i64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i64.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x0706050403020100)));
    /// assert_eq!(le_i64.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn i64<Input, Error>(endian: Endianness) -> impl Parser<Input, i64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_i64,
                Endianness::Little => le_i64,
                Endianness::Native => le_i64,
            }
        }(input)
    }
    /// Recognizes a signed 16 byte integer
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian i128 integer,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian i128 integer.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::i128;
    ///
    /// fn be_i128(input: &mut &[u8]) -> ModalResult<i128> {
    ///     i128(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i128.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x00010203040506070001020304050607)));
    /// assert!(be_i128.parse_peek(&b"\x01"[..]).is_err());
    ///
    /// fn le_i128(input: &mut &[u8]) -> ModalResult<i128> {
    ///     i128(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i128.parse_peek(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..]), Ok((&b"abcefg"[..], 0x07060504030201000706050403020100)));
    /// assert!(le_i128.parse_peek(&b"\x01"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::i128;
    ///
    /// fn be_i128(input: &mut Partial<&[u8]>) -> ModalResult<i128> {
    ///     i128(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_i128.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x00010203040506070001020304050607)));
    /// assert_eq!(be_i128.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    ///
    /// fn le_i128(input: &mut Partial<&[u8]>) -> ModalResult<i128> {
    ///     i128(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_i128.parse_peek(Partial::new(&b"\x00\x01\x02\x03\x04\x05\x06\x07\x00\x01\x02\x03\x04\x05\x06\x07abcefg"[..])), Ok((Partial::new(&b"abcefg"[..]), 0x07060504030201000706050403020100)));
    /// assert_eq!(le_i128.parse_peek(Partial::new(&b"\x01"[..])), Err(ErrMode::Incomplete(Needed::new(15))));
    /// ```
    #[inline(always)]
    pub fn i128<Input, Error>(endian: Endianness) -> impl Parser<Input, i128, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_i128,
                Endianness::Little => le_i128,
                Endianness::Native => le_i128,
            }
        }(input)
    }
    /// Recognizes a big endian 4 bytes floating point number.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_f32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<f32> {
    ///       be_f32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&[0x41, 0x48, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
    /// assert!(parser.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_f32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<f32> {
    ///       be_f32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x40, 0x29, 0x00, 0x00][..])), Ok((Partial::new(&b""[..]), 2.640625)));
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x01][..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn be_f32<Input, Error>(input: &mut Input) -> Result<f32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_f32",
                move |input: &mut Input| {
                    be_uint::<_, u32, _>(input, 4).map(f32::from_bits)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a big endian 8 bytes floating point number.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::be_f64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<f64> {
    ///       be_f64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
    /// assert!(parser.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::be_f64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<f64> {
    ///       be_f64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x01][..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn be_f64<Input, Error>(input: &mut Input) -> Result<f64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_f64",
                move |input: &mut Input| {
                    be_uint::<_, u64, _>(input, 8).map(f64::from_bits)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian 4 bytes floating point number.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_f32;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<f32> {
    ///       le_f32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&[0x00, 0x00, 0x48, 0x41][..]), Ok((&b""[..], 12.5)));
    /// assert!(parser.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_f32;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<f32> {
    ///       le_f32.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x00, 0x00, 0x48, 0x41][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x01][..])), Err(ErrMode::Incomplete(Needed::new(3))));
    /// ```
    #[inline(always)]
    pub fn le_f32<Input, Error>(input: &mut Input) -> Result<f32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "le_f32",
                move |input: &mut Input| {
                    le_uint::<_, u32, _>(input, 4).map(f32::from_bits)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a little endian 8 bytes floating point number.
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::le_f64;
    ///
    /// fn parser(s: &mut &[u8]) -> ModalResult<f64> {
    ///       le_f64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..]), Ok((&b""[..], 12.5)));
    /// assert!(parser.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::binary::le_f64;
    ///
    /// fn parser(s: &mut Partial<&[u8]>) -> ModalResult<f64> {
    ///       le_f64.parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x41][..])), Ok((Partial::new(&b""[..]), 3145728.0)));
    /// assert_eq!(parser.parse_peek(Partial::new(&[0x01][..])), Err(ErrMode::Incomplete(Needed::new(7))));
    /// ```
    #[inline(always)]
    pub fn le_f64<Input, Error>(input: &mut Input) -> Result<f64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        trace(
                "be_f64",
                move |input: &mut Input| {
                    le_uint::<_, u64, _>(input, 8).map(f64::from_bits)
                },
            )
            .parse_next(input)
    }
    /// Recognizes a 4 byte floating point number
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian f32 float,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian f32 float.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::f32;
    ///
    /// fn be_f32(input: &mut &[u8]) -> ModalResult<f32> {
    ///     f32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_f32.parse_peek(&[0x41, 0x48, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
    /// assert!(be_f32.parse_peek(&b"abc"[..]).is_err());
    ///
    /// fn le_f32(input: &mut &[u8]) -> ModalResult<f32> {
    ///     f32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_f32.parse_peek(&[0x00, 0x00, 0x48, 0x41][..]), Ok((&b""[..], 12.5)));
    /// assert!(le_f32.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::f32;
    ///
    /// fn be_f32(input: &mut Partial<&[u8]>) -> ModalResult<f32> {
    ///     f32(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_f32.parse_peek(Partial::new(&[0x41, 0x48, 0x00, 0x00][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(be_f32.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    ///
    /// fn le_f32(input: &mut Partial<&[u8]>) -> ModalResult<f32> {
    ///     f32(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_f32.parse_peek(Partial::new(&[0x00, 0x00, 0x48, 0x41][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(le_f32.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn f32<Input, Error>(endian: Endianness) -> impl Parser<Input, f32, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_f32,
                Endianness::Little => le_f32,
                Endianness::Native => le_f32,
            }
        }(input)
    }
    /// Recognizes an 8 byte floating point number
    ///
    /// If the parameter is `winnow::binary::Endianness::Big`, parse a big endian f64 float,
    /// otherwise if `winnow::binary::Endianness::Little` parse a little endian f64 float.
    ///
    /// *Complete version*: returns an error if there is not enough input data
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// use winnow::binary::f64;
    ///
    /// fn be_f64(input: &mut &[u8]) -> ModalResult<f64> {
    ///     f64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_f64.parse_peek(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..]), Ok((&b""[..], 12.5)));
    /// assert!(be_f64.parse_peek(&b"abc"[..]).is_err());
    ///
    /// fn le_f64(input: &mut &[u8]) -> ModalResult<f64> {
    ///     f64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_f64.parse_peek(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..]), Ok((&b""[..], 12.5)));
    /// assert!(le_f64.parse_peek(&b"abc"[..]).is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::error::Needed::Size;
    /// # use winnow::Partial;
    /// use winnow::binary::f64;
    ///
    /// fn be_f64(input: &mut Partial<&[u8]>) -> ModalResult<f64> {
    ///     f64(winnow::binary::Endianness::Big).parse_next(input)
    /// };
    ///
    /// assert_eq!(be_f64.parse_peek(Partial::new(&[0x40, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(be_f64.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(5))));
    ///
    /// fn le_f64(input: &mut Partial<&[u8]>) -> ModalResult<f64> {
    ///     f64(winnow::binary::Endianness::Little).parse_next(input)
    /// };
    ///
    /// assert_eq!(le_f64.parse_peek(Partial::new(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..])), Ok((Partial::new(&b""[..]), 12.5)));
    /// assert_eq!(le_f64.parse_peek(Partial::new(&b"abc"[..])), Err(ErrMode::Incomplete(Needed::new(5))));
    /// ```
    #[inline(always)]
    pub fn f64<Input, Error>(endian: Endianness) -> impl Parser<Input, f64, Error>
    where
        Input: StreamIsPartial + Stream<Token = u8>,
        Error: ParserError<Input>,
    {
        move |input: &mut Input| {
            match endian {
                Endianness::Big => be_f64,
                Endianness::Little => le_f64,
                Endianness::Native => le_f64,
            }
        }(input)
    }
    /// Get a length-prefixed slice ([TLV](https://en.wikipedia.org/wiki/Type-length-value))
    ///
    /// To apply a parser to the returned slice, see [`length_and_then`].
    ///
    /// If the count is for something besides tokens, see [`length_repeat`].
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::Needed, stream::Partial};
    /// # use winnow::prelude::*;
    /// use winnow::Bytes;
    /// use winnow::binary::be_u16;
    /// use winnow::binary::length_take;
    ///
    /// type Stream<'i> = Partial<&'i Bytes>;
    ///
    /// fn stream(b: &[u8]) -> Stream<'_> {
    ///     Partial::new(Bytes::new(b))
    /// }
    ///
    /// fn parser<'i>(s: &mut Stream<'i>) -> ModalResult<&'i [u8]> {
    ///   length_take(be_u16).parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(stream(b"\x00\x03abcefg")), Ok((stream(&b"efg"[..]), &b"abc"[..])));
    /// assert_eq!(parser.parse_peek(stream(b"\x00\x03a")), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    pub fn length_take<Input, Count, Error, CountParser>(
        mut count: CountParser,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        Count: ToUsize,
        CountParser: Parser<Input, Count, Error>,
        Error: ParserError<Input>,
    {
        trace(
            "length_take",
            move |i: &mut Input| {
                let length = count.parse_next(i)?;
                crate::token::take(length).parse_next(i)
            },
        )
    }
    /// Parse a length-prefixed slice ([TLV](https://en.wikipedia.org/wiki/Type-length-value))
    ///
    /// *Complete version*: Returns an error if there is not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there is not enough data.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed, stream::{Partial, StreamIsPartial}};
    /// # use winnow::prelude::*;
    /// use winnow::Bytes;
    /// use winnow::binary::be_u16;
    /// use winnow::binary::length_and_then;
    ///
    /// type Stream<'i> = Partial<&'i Bytes>;
    ///
    /// fn stream(b: &[u8]) -> Stream<'_> {
    ///     Partial::new(Bytes::new(b))
    /// }
    ///
    /// fn complete_stream(b: &[u8]) -> Stream<'_> {
    ///     let mut p = Partial::new(Bytes::new(b));
    ///     let _ = p.complete();
    ///     p
    /// }
    ///
    /// fn parser<'i>(s: &mut Stream<'i>) -> ModalResult<&'i [u8]> {
    ///   length_and_then(be_u16, "abc").parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(stream(b"\x00\x03abcefg")), Ok((stream(&b"efg"[..]), &b"abc"[..])));
    /// assert!(parser.parse_peek(stream(b"\x00\x03123123")).is_err());
    /// assert_eq!(parser.parse_peek(stream(b"\x00\x03a")), Err(ErrMode::Incomplete(Needed::new(2))));
    /// ```
    pub fn length_and_then<Input, Output, Count, Error, CountParser, ParseNext>(
        mut count: CountParser,
        mut parser: ParseNext,
    ) -> impl Parser<Input, Output, Error>
    where
        Input: StreamIsPartial + Stream + UpdateSlice + Clone,
        Count: ToUsize,
        CountParser: Parser<Input, Count, Error>,
        ParseNext: Parser<Input, Output, Error>,
        Error: ParserError<Input>,
    {
        trace(
            "length_and_then",
            move |i: &mut Input| {
                let data = length_take(count.by_ref()).parse_next(i)?;
                let mut data = Input::update_slice(i.clone(), data);
                let _ = data.complete();
                let o = parser.by_ref().complete_err().parse_next(&mut data)?;
                Ok(o)
            },
        )
    }
    /// [`Accumulate`] a length-prefixed sequence of values ([TLV](https://en.wikipedia.org/wiki/Type-length-value))
    ///
    /// If the length represents token counts, see instead [`length_take`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "std")] {
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::InputError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::Bytes;
    /// use winnow::binary::u8;
    /// use winnow::binary::length_repeat;
    ///
    /// type Stream<'i> = &'i Bytes;
    ///
    /// fn stream(b: &[u8]) -> Stream<'_> {
    ///     Bytes::new(b)
    /// }
    ///
    /// fn parser<'i>(s: &mut Stream<'i>) -> ModalResult<Vec<&'i [u8]>> {
    ///   length_repeat(u8.map(|i| {
    ///      println!("got number: {}", i);
    ///      i
    ///   }), "abc").parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(stream(b"\x02abcabcabc")), Ok((stream(b"abc"), vec![&b"abc"[..], &b"abc"[..]])));
    /// assert!(parser.parse_peek(stream(b"\x03123123123")).is_err());
    /// # }
    /// ```
    pub fn length_repeat<
        Input,
        Output,
        Accumulator,
        Count,
        Error,
        CountParser,
        ParseNext,
    >(
        mut count: CountParser,
        mut parser: ParseNext,
    ) -> impl Parser<Input, Accumulator, Error>
    where
        Input: Stream,
        Count: ToUsize,
        Accumulator: Accumulate<Output>,
        CountParser: Parser<Input, Count, Error>,
        ParseNext: Parser<Input, Output, Error>,
        Error: ParserError<Input>,
    {
        trace(
            "length_repeat",
            move |i: &mut Input| {
                let n = count.parse_next(i)?;
                let n = n.to_usize();
                repeat(n, parser.by_ref()).parse_next(i)
            },
        )
    }
}
pub mod combinator {
    //! # List of parsers and combinators
    //!
    //! <div class="warning">
    //!
    //! **Note**: this list is meant to provide a nicer way to find a parser than reading through the documentation on docs.rs. Function combinators are organized in module so they are a bit easier to find.
    //!
    //! </div>
    //!
    //! ## Basic elements
    //!
    //! Those are used to take a series of tokens for the lowest level elements of your grammar, like, "here is a dot", or "here is an big endian integer".
    //!
    //! | combinator | usage | input | new input | output | comment |
    //! |---|---|---|---|---|---|
    //! | [`one_of`][crate::token::one_of] | `one_of(['a', 'b', 'c'])` |  `"abc"` |  `"bc"` | `Ok('a')` |Matches one of the provided [set of tokens][crate::stream::ContainsToken] (works with non ASCII characters too)|
    //! | [`none_of`][crate::token::none_of] | `none_of(['a', 'b', 'c'])` |  `"xyab"` |  `"yab"` | `Ok('x')` |Matches anything but one of the provided [set of tokens][crate::stream::ContainsToken]|
    //! | [`literal`][crate::token::literal] | `"hello"` |  `"hello world"` |  `" world"` | `Ok("hello")` |Recognizes a specific suite of characters or bytes (see also [`Caseless`][crate::ascii::Caseless])|
    //! | [`take`][crate::token::take] | `take(4)` |  `"hello"` |  `"o"` | `Ok("hell")` |Takes a specific number of bytes or characters|
    //! | [`take_while`][crate::token::take_while] | `take_while(0.., is_alphabetic)` |  `"abc123"` |  `"123"` | `Ok("abc")` |Returns the longest slice of bytes or characters for which the provided [set of tokens][crate::stream::ContainsToken] matches.|
    //! | [`take_till`][crate::token::take_till] | `take_till(0.., is_alphabetic)` |  `"123abc"` |  `"abc"` | `Ok("123")` |Returns a slice of bytes or characters until the provided [set of tokens][crate::stream::ContainsToken] matches. This is the reverse behaviour from `take_while`: `take_till(f)` is equivalent to `take_while(0.., \|c\| !f(c))`|
    //! | [`take_until`][crate::token::take_until] | `take_until(0.., "world")` |  `"Hello world"` |  `"world"` | `Ok("Hello ")` |Returns a slice of bytes or characters until the provided [literal][crate::token::literal] is found.|
    //!
    //! ## Choice combinators
    //!
    //! | combinator | usage | input | new input | output | comment |
    //! |---|---|---|---|---|---|
    //! | [`alt`] | `alt(("ab", "cd"))` |  `"cdef"` |  `"ef"` | `Ok("cd")` |Try a list of parsers and return the result of the first successful one|
    //! | [`dispatch`] | \- | \- | \- | \- | `match` for parsers |
    //!
    //! ## Sequence combinators
    //!
    //! | combinator | usage | input | new input | output | comment |
    //! |---|---|---|---|---|---|
    //! | [`(...)` (tuples)][crate::Parser] | `("ab", "XY", take(1))` | `"abXYZ!"` | `"!"` | `Ok(("ab", "XY", "Z"))` |Parse a series of values|
    //! | [`seq!`] | `seq!(_: '(', take(2), _: ')')` | `"(ab)cd"` | `"cd"` | `Ok("ab")` |Parse a series of values, discarding those you specify|
    //! | [`delimited`] | `delimited('(', take(2), ')')` | `"(ab)cd"` | `"cd"` | `Ok("ab")` |Parse three values, discarding the first and third value|
    //! | [`preceded`] | `preceded("ab", "XY")` | `"abXYZ"` | `"Z"` | `Ok("XY")` |Parse two values, discarding the first value|
    //! | [`terminated`] | `terminated("ab", "XY")` | `"abXYZ"` | `"Z"` | `Ok("ab")` |Parse two values, discarding the second value|
    //! | [`separated_pair`] | `separated_pair("hello", ',', "world")` | `"hello,world!"` | `"!"` | `Ok(("hello", "world"))` | Parse three values, discarding the middle value|
    //!
    //! ## Applying a parser multiple times
    //!
    //! | combinator | usage | input | new input | output | comment |
    //! |---|---|---|---|---|---|
    //! | [`repeat`] | `repeat(1..=3, "ab")` | `"ababc"` | `"c"` | `Ok(vec!["ab", "ab"])` |Applies the parser between m and n times (n included) and returns the list of results in a Vec|
    //! | [`repeat_till`] | `repeat_till(0.., "ab", "ef")` | `"ababefg"` | `"g"` | `Ok((vec!["ab", "ab"], "ef"))` |Applies the first parser until the second applies. Returns a tuple containing the list of results from the first in a Vec and the result of the second|
    //! | [`separated`] | `separated(1..=3, "ab", ",")` | `"ab,ab,ab."` | `"."` | `Ok(vec!["ab", "ab", "ab"])` |Applies the parser and separator between m and n times (n included) and returns the list of results in a Vec|
    //! | [`Repeat::fold`] | <code>repeat(1..=2, `be_u8`).fold(\|\| 0, \|acc, item\| acc + item)</code> | `[1, 2, 3]` | `[3]` | `Ok(3)` |Applies the parser between m and n times (n included) and folds the list of return value|
    //!
    //! ## Partial related
    //!
    //! - [`eof`]: Returns its input if it is at the end of input data
    //! - [`Parser::complete_err`]: Replaces an `Incomplete` returned by the child parser with an `Backtrack`
    //!
    //! ## Modifiers
    //!
    //! - [`cond`]: Conditional combinator. Wraps another parser and calls it if the condition is met
    //! - [`Parser::flat_map`]: method to map a new parser from the output of the first parser, then apply that parser over the rest of the input
    //! - [`Parser::value`]: method to replace the result of a parser
    //! - [`Parser::default_value`]: method to replace the result of a parser
    //! - [`Parser::void`]: method to discard the result of a parser
    //! - [`Parser::map`]: method to map a function on the result of a parser
    //! - [`Parser::and_then`]: Applies a second parser over the output of the first one
    //! - [`Parser::verify_map`]: Maps a function returning an `Option` on the output of a parser
    //! - [`Parser::try_map`]: Maps a function returning a `Result` on the output of a parser
    //! - [`Parser::parse_to`]: Apply [`std::str::FromStr`] to the output of the parser
    //! - [`not`]: Returns a result only if the embedded parser returns `Backtrack` or `Incomplete`. Does not consume the input
    //! - [`opt`]: Make the underlying parser optional
    //! - [`peek`]: Returns a result without consuming the input
    //! - [`Parser::take`]: If the child parser was successful, return the consumed input as the produced value
    //! - [`Parser::with_taken`]: If the child parser was successful, return a tuple of the consumed input and the produced output.
    //! - [`Parser::span`]: If the child parser was successful, return the location of the consumed input as the produced value
    //! - [`Parser::with_span`]: If the child parser was successful, return a tuple of the location of the consumed input and the produced output.
    //! - [`Parser::verify`]: Returns the result of the child parser if it satisfies a verification function
    //!
    //! ## Error management and debugging
    //!
    //! - [`cut_err`]: Commit the parse result, disallowing alternative parsers from being attempted
    //! - [`backtrack_err`]: Attempts a parse, allowing alternative parsers to be attempted despite
    //!   use of `cut_err`
    //! - [`Parser::context`]: Add context to the error if the parser fails
    //! - [`trace`]: Print the parse state with the `debug` feature flag
    //! - [`todo()`]: Placeholder parser
    //!
    //! ## Remaining combinators
    //!
    //! - [`empty`]: Succeed, consuming no input
    //! - [`fail`]: Inversion of [`empty`]. Always fails.
    //! - [`Parser::by_ref`]: Allow moving `&mut impl Parser` into other parsers
    //!
    //! ## Text parsing
    //!
    //! - [`any`][crate::token::any]: Matches one token
    //! - [`tab`][crate::ascii::tab]: Matches a tab character `\t`
    //! - [`crlf`][crate::ascii::crlf]: Recognizes the string `\r\n`
    //! - [`line_ending`][crate::ascii::line_ending]: Recognizes an end of line (both `\n` and `\r\n`)
    //! - [`newline`][crate::ascii::newline]: Matches a newline character `\n`
    //! - [`till_line_ending`][crate::ascii::till_line_ending]: Recognizes a string of any char except `\r` or `\n`
    //! - [`rest`][crate::token::rest]: Return the remaining input
    //!
    //! - [`alpha0`][crate::ascii::alpha0]: Recognizes zero or more lowercase and uppercase alphabetic characters: `[a-zA-Z]`. [`alpha1`][crate::ascii::alpha1] does the same but returns at least one character
    //! - [`alphanumeric0`][crate::ascii::alphanumeric0]: Recognizes zero or more numerical and alphabetic characters: `[0-9a-zA-Z]`. [`alphanumeric1`][crate::ascii::alphanumeric1] does the same but returns at least one character
    //! - [`space0`][crate::ascii::space0]: Recognizes zero or more spaces and tabs. [`space1`][crate::ascii::space1] does the same but returns at least one character
    //! - [`multispace0`][crate::ascii::multispace0]: Recognizes zero or more spaces, tabs, carriage returns and line feeds. [`multispace1`][crate::ascii::multispace1] does the same but returns at least one character
    //! - [`digit0`][crate::ascii::digit0]: Recognizes zero or more numerical characters: `[0-9]`. [`digit1`][crate::ascii::digit1] does the same but returns at least one character
    //! - [`hex_digit0`][crate::ascii::hex_digit0]: Recognizes zero or more hexadecimal numerical characters: `[0-9A-Fa-f]`. [`hex_digit1`][crate::ascii::hex_digit1] does the same but returns at least one character
    //! - [`oct_digit0`][crate::ascii::oct_digit0]: Recognizes zero or more octal characters: `[0-7]`. [`oct_digit1`][crate::ascii::oct_digit1] does the same but returns at least one character
    //!
    //! - [`float`][crate::ascii::float]: Parse a floating point number in a byte string
    //! - [`dec_int`][crate::ascii::dec_int]: Decode a variable-width, decimal signed integer
    //! - [`dec_uint`][crate::ascii::dec_uint]: Decode a variable-width, decimal unsigned integer
    //! - [`hex_uint`][crate::ascii::hex_uint]: Decode a variable-width, hexadecimal integer
    //!
    //! - [`take_escaped`][crate::ascii::take_escaped]: Recognize the input slice with escaped characters
    //! - [`escaped`][crate::ascii::escaped]: Parse escaped characters, unescaping them
    //!
    //! - [`expression()`]: Parse an operator precedence expression with Pratt parsing
    //!
    //! ### Character test functions
    //!
    //! Use these functions with a combinator like `take_while`:
    //!
    //! - [`AsChar::is_alpha`][crate::stream::AsChar::is_alpha]: Tests if byte is ASCII alphabetic: `[A-Za-z]`
    //! - [`AsChar::is_alphanum`][crate::stream::AsChar::is_alphanum]: Tests if byte is ASCII alphanumeric: `[A-Za-z0-9]`
    //! - [`AsChar::is_dec_digit`][crate::stream::AsChar::is_dec_digit]: Tests if byte is ASCII digit: `[0-9]`
    //! - [`AsChar::is_hex_digit`][crate::stream::AsChar::is_hex_digit]: Tests if byte is ASCII hex digit: `[0-9A-Fa-f]`
    //! - [`AsChar::is_oct_digit`][crate::stream::AsChar::is_oct_digit]: Tests if byte is ASCII octal digit: `[0-7]`
    //! - [`AsChar::is_space`][crate::stream::AsChar::is_space]: Tests if byte is ASCII space or tab: `[ \t]`
    //! - [`AsChar::is_newline`][crate::stream::AsChar::is_newline]: Tests if byte is ASCII newline: `[\n]`
    //!
    //! ## Binary format parsing
    //!
    //! - [`length_repeat`][crate::binary::length_repeat] Gets a number from the first parser, then applies the second parser that many times
    //! - [`length_take`][crate::binary::length_take]: Gets a number from the first parser, then takes a subslice of the input of that size, and returns that subslice
    //! - [`length_and_then`][crate::binary::length_and_then]: Gets a number from the first parser, takes a subslice of the input of that size, then applies the second parser on that subslice. If the second parser returns `Incomplete`, `length_value` will return an error
    //!
    //! ### Integers
    //!
    //! Parsing integers from binary formats can be done in two ways: With parser functions, or combinators with configurable endianness.
    //!
    //! - **configurable endianness:** [`i16`][crate::binary::i16], [`i32`][crate::binary::i32],
    //!   [`i64`][crate::binary::i64], [`u16`][crate::binary::u16], [`u32`][crate::binary::u32],
    //!   [`u64`][crate::binary::u64] are combinators that take as argument a
    //!   [`winnow::binary::Endianness`][crate::binary::Endianness], like this: `i16(endianness)`. If the
    //!   parameter is `winnow::binary::Endianness::Big`, parse a big endian `i16` integer, otherwise a
    //!   little endian `i16` integer.
    //! - **fixed endianness**: The functions are prefixed by `be_` for big endian numbers, and by `le_` for little endian numbers, and the suffix is the type they parse to. As an example, `be_u32` parses a big endian unsigned integer stored in 32 bits.
    //!   - [`be_f32`][crate::binary::be_f32], [`be_f64`][crate::binary::be_f64]: Big endian floating point numbers
    //!   - [`le_f32`][crate::binary::le_f32], [`le_f64`][crate::binary::le_f64]: Little endian floating point numbers
    //!   - [`be_i8`][crate::binary::be_i8], [`be_i16`][crate::binary::be_i16], [`be_i24`][crate::binary::be_i24], [`be_i32`][crate::binary::be_i32], [`be_i64`][crate::binary::be_i64], [`be_i128`][crate::binary::be_i128]: Big endian signed integers
    //!   - [`be_u8`][crate::binary::be_u8], [`be_u16`][crate::binary::be_u16], [`be_u24`][crate::binary::be_u24], [`be_u32`][crate::binary::be_u32], [`be_u64`][crate::binary::be_u64], [`be_u128`][crate::binary::be_u128]: Big endian unsigned integers
    //!   - [`le_i8`][crate::binary::le_i8], [`le_i16`][crate::binary::le_i16], [`le_i24`][crate::binary::le_i24], [`le_i32`][crate::binary::le_i32], [`le_i64`][crate::binary::le_i64], [`le_i128`][crate::binary::le_i128]: Little endian signed integers
    //!   - [`le_u8`][crate::binary::le_u8], [`le_u16`][crate::binary::le_u16], [`le_u24`][crate::binary::le_u24], [`le_u32`][crate::binary::le_u32], [`le_u64`][crate::binary::le_u64], [`le_u128`][crate::binary::le_u128]: Little endian unsigned integers
    //!
    //! ### Bit stream parsing
    //!
    //! - [`bits`][crate::binary::bits::bits]: Transforms the current input type (byte slice `&[u8]`) to a bit stream on which bit specific parsers and more general combinators can be applied
    //! - [`bytes`][crate::binary::bits::bytes]: Transforms its bits stream input back into a byte slice for the underlying parser
    //! - [`take`][crate::binary::bits::take]: Take a set number of bits
    //! - [`pattern`][crate::binary::bits::pattern]: Check if a set number of bits matches a pattern
    //! - [`bool`][crate::binary::bits::bool]: Match any one bit
    mod branch {
        use crate::combinator::trace;
        use crate::error::ParserError;
        use crate::stream::Stream;
        use crate::{Parser, Result};
        #[doc(inline)]
        pub use crate::dispatch;
        /// Helper trait for the [`alt()`] combinator.
        ///
        /// This trait is implemented for tuples of up to 21 elements
        pub trait Alt<I, O, E> {
            /// Tests each parser in the tuple and returns the result of the first one that succeeds
            fn choice(&mut self, input: &mut I) -> Result<O, E>;
        }
        /// Pick the first successful parser
        ///
        /// To stop on an error, rather than trying further cases, see
        /// [`cut_err`][crate::combinator::cut_err] ([example][crate::_tutorial::chapter_7]).
        ///
        /// For tight control over the error when no match is found, add a final case using [`fail`][crate::combinator::fail].
        /// Alternatively, with a [custom error type][crate::_topic::error], it is possible to track all
        /// errors or return the error of the parser that went the farthest in the input data.
        ///
        /// When the alternative cases have unique prefixes, [`dispatch`] can offer better performance.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::ascii::{alpha1, digit1};
        /// use winnow::combinator::alt;
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///   alt((alpha1, digit1)).parse_next(input)
        /// };
        ///
        /// // the first parser, alpha1, takes the input
        /// assert_eq!(parser.parse_peek("abc"), Ok(("", "abc")));
        ///
        /// // the first parser returns an error, so alt tries the second one
        /// assert_eq!(parser.parse_peek("123456"), Ok(("", "123456")));
        ///
        /// // both parsers failed, and with the default error type, alt will return the last error
        /// assert!(parser.parse_peek(" ").is_err());
        /// # }
        /// ```
        #[doc(alias = "choice")]
        #[inline(always)]
        pub fn alt<Input: Stream, Output, Error, Alternatives>(
            mut alternatives: Alternatives,
        ) -> impl Parser<Input, Output, Error>
        where
            Alternatives: Alt<Input, Output, Error>,
            Error: ParserError<Input>,
        {
            trace("alt", move |i: &mut Input| alternatives.choice(i))
        }
        impl<
            const N: usize,
            I: Stream,
            O,
            E: ParserError<I>,
            P: Parser<I, O, E>,
        > Alt<I, O, E> for [P; N] {
            fn choice(&mut self, input: &mut I) -> Result<O, E> {
                let mut error: Option<E> = None;
                let start = input.checkpoint();
                for branch in self {
                    input.reset(&start);
                    match branch.parse_next(input) {
                        Err(e) if e.is_backtrack() => {
                            error = match error {
                                Some(error) => Some(error.or(e)),
                                None => Some(e),
                            };
                        }
                        res => return res,
                    }
                }
                match error {
                    Some(e) => Err(e.append(input, &start)),
                    None => {
                        Err(
                            ParserError::assert(input, "`alt` needs at least one parser"),
                        )
                    }
                }
            }
        }
        impl<I: Stream, O, E: ParserError<I>, P: Parser<I, O, E>> Alt<I, O, E>
        for &mut [P] {
            fn choice(&mut self, input: &mut I) -> Result<O, E> {
                let mut error: Option<E> = None;
                let start = input.checkpoint();
                for branch in self.iter_mut() {
                    input.reset(&start);
                    match branch.parse_next(input) {
                        Err(e) if e.is_backtrack() => {
                            error = match error {
                                Some(error) => Some(error.or(e)),
                                None => Some(e),
                            };
                        }
                        res => return res,
                    }
                }
                match error {
                    Some(e) => Err(e.append(input, &start)),
                    None => {
                        Err(
                            ParserError::assert(input, "`alt` needs at least one parser"),
                        )
                    }
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                { Err(err.append(input, &start)) }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            { Err(err.append(input, &start)) }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4, Alt5) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        { Err(err.append(input, &start)) }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
            Alt6: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4, Alt5, Alt6) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        {
                                                            input.reset(&start);
                                                            match self.4.parse_next(input) {
                                                                Err(e) if e.is_backtrack() => {
                                                                    let err = err.or(e);
                                                                    { Err(err.append(input, &start)) }
                                                                }
                                                                res => res,
                                                            }
                                                        }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
            Alt6: Parser<I, Output, Error>,
            Alt7: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4, Alt5, Alt6, Alt7) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        {
                                                            input.reset(&start);
                                                            match self.4.parse_next(input) {
                                                                Err(e) if e.is_backtrack() => {
                                                                    let err = err.or(e);
                                                                    {
                                                                        input.reset(&start);
                                                                        match self.5.parse_next(input) {
                                                                            Err(e) if e.is_backtrack() => {
                                                                                let err = err.or(e);
                                                                                { Err(err.append(input, &start)) }
                                                                            }
                                                                            res => res,
                                                                        }
                                                                    }
                                                                }
                                                                res => res,
                                                            }
                                                        }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
            Alt6: Parser<I, Output, Error>,
            Alt7: Parser<I, Output, Error>,
            Alt8: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4, Alt5, Alt6, Alt7, Alt8) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        {
                                                            input.reset(&start);
                                                            match self.4.parse_next(input) {
                                                                Err(e) if e.is_backtrack() => {
                                                                    let err = err.or(e);
                                                                    {
                                                                        input.reset(&start);
                                                                        match self.5.parse_next(input) {
                                                                            Err(e) if e.is_backtrack() => {
                                                                                let err = err.or(e);
                                                                                {
                                                                                    input.reset(&start);
                                                                                    match self.6.parse_next(input) {
                                                                                        Err(e) if e.is_backtrack() => {
                                                                                            let err = err.or(e);
                                                                                            { Err(err.append(input, &start)) }
                                                                                        }
                                                                                        res => res,
                                                                                    }
                                                                                }
                                                                            }
                                                                            res => res,
                                                                        }
                                                                    }
                                                                }
                                                                res => res,
                                                            }
                                                        }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
            Alt6: Parser<I, Output, Error>,
            Alt7: Parser<I, Output, Error>,
            Alt8: Parser<I, Output, Error>,
            Alt9: Parser<I, Output, Error>,
        > Alt<I, Output, Error> for (Alt2, Alt3, Alt4, Alt5, Alt6, Alt7, Alt8, Alt9) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        {
                                                            input.reset(&start);
                                                            match self.4.parse_next(input) {
                                                                Err(e) if e.is_backtrack() => {
                                                                    let err = err.or(e);
                                                                    {
                                                                        input.reset(&start);
                                                                        match self.5.parse_next(input) {
                                                                            Err(e) if e.is_backtrack() => {
                                                                                let err = err.or(e);
                                                                                {
                                                                                    input.reset(&start);
                                                                                    match self.6.parse_next(input) {
                                                                                        Err(e) if e.is_backtrack() => {
                                                                                            let err = err.or(e);
                                                                                            {
                                                                                                input.reset(&start);
                                                                                                match self.7.parse_next(input) {
                                                                                                    Err(e) if e.is_backtrack() => {
                                                                                                        let err = err.or(e);
                                                                                                        { Err(err.append(input, &start)) }
                                                                                                    }
                                                                                                    res => res,
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        res => res,
                                                                                    }
                                                                                }
                                                                            }
                                                                            res => res,
                                                                        }
                                                                    }
                                                                }
                                                                res => res,
                                                            }
                                                        }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<
            I: Stream,
            Output,
            Error: ParserError<I>,
            Alt2: Parser<I, Output, Error>,
            Alt3: Parser<I, Output, Error>,
            Alt4: Parser<I, Output, Error>,
            Alt5: Parser<I, Output, Error>,
            Alt6: Parser<I, Output, Error>,
            Alt7: Parser<I, Output, Error>,
            Alt8: Parser<I, Output, Error>,
            Alt9: Parser<I, Output, Error>,
            Alt10: Parser<I, Output, Error>,
        > Alt<I, Output, Error>
        for (Alt2, Alt3, Alt4, Alt5, Alt6, Alt7, Alt8, Alt9, Alt10) {
            fn choice(&mut self, input: &mut I) -> Result<Output, Error> {
                let start = input.checkpoint();
                match self.0.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        match self.1.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                let err = e.or(e);
                                {
                                    input.reset(&start);
                                    match self.2.parse_next(input) {
                                        Err(e) if e.is_backtrack() => {
                                            let err = err.or(e);
                                            {
                                                input.reset(&start);
                                                match self.3.parse_next(input) {
                                                    Err(e) if e.is_backtrack() => {
                                                        let err = err.or(e);
                                                        {
                                                            input.reset(&start);
                                                            match self.4.parse_next(input) {
                                                                Err(e) if e.is_backtrack() => {
                                                                    let err = err.or(e);
                                                                    {
                                                                        input.reset(&start);
                                                                        match self.5.parse_next(input) {
                                                                            Err(e) if e.is_backtrack() => {
                                                                                let err = err.or(e);
                                                                                {
                                                                                    input.reset(&start);
                                                                                    match self.6.parse_next(input) {
                                                                                        Err(e) if e.is_backtrack() => {
                                                                                            let err = err.or(e);
                                                                                            {
                                                                                                input.reset(&start);
                                                                                                match self.7.parse_next(input) {
                                                                                                    Err(e) if e.is_backtrack() => {
                                                                                                        let err = err.or(e);
                                                                                                        {
                                                                                                            input.reset(&start);
                                                                                                            match self.8.parse_next(input) {
                                                                                                                Err(e) if e.is_backtrack() => {
                                                                                                                    let err = err.or(e);
                                                                                                                    { Err(err.append(input, &start)) }
                                                                                                                }
                                                                                                                res => res,
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                    res => res,
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        res => res,
                                                                                    }
                                                                                }
                                                                            }
                                                                            res => res,
                                                                        }
                                                                    }
                                                                }
                                                                res => res,
                                                            }
                                                        }
                                                    }
                                                    res => res,
                                                }
                                            }
                                        }
                                        res => res,
                                    }
                                }
                            }
                            res => res,
                        }
                    }
                    res => res,
                }
            }
        }
        impl<I: Stream, O, E: ParserError<I>, A: Parser<I, O, E>> Alt<I, O, E> for (A,) {
            fn choice(&mut self, input: &mut I) -> Result<O, E> {
                self.0.parse_next(input)
            }
        }
    }
    mod core {
        use crate::combinator::trace;
        use crate::error::{ModalError, ParserError};
        use crate::stream::Stream;
        use crate::{Parser, Result};
        /// Apply a [`Parser`], producing `None` on [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack].
        ///
        /// To chain an error up, see [`cut_err`].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// use winnow::combinator::opt;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(i: &mut &'i str) -> ModalResult<Option<&'i str>> {
        ///   opt(alpha1).parse_next(i)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd;"), Ok((";", Some("abcd"))));
        /// assert_eq!(parser.parse_peek("123;"), Ok(("123;", None)));
        /// # }
        /// ```
        pub fn opt<Input: Stream, Output, Error, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, Option<Output>, Error>
        where
            ParseNext: Parser<Input, Output, Error>,
            Error: ParserError<Input>,
        {
            trace(
                "opt",
                move |input: &mut Input| {
                    let start = input.checkpoint();
                    match parser.parse_next(input) {
                        Ok(o) => Ok(Some(o)),
                        Err(e) if e.is_backtrack() => {
                            input.reset(&start);
                            Ok(None)
                        }
                        Err(e) => Err(e),
                    }
                },
            )
        }
        /// Calls the parser if the condition is met.
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::combinator::opt;
        /// use winnow::combinator::cond;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(i: &mut &'i str) -> ModalResult<Option<&'i str>> {
        ///   let prefix = opt("-").parse_next(i)?;
        ///   let condition = prefix.is_some();
        ///   cond(condition, alpha1).parse_next(i)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("-abcd;"), Ok((";", Some("abcd"))));
        /// assert_eq!(parser.parse_peek("abcd;"), Ok(("abcd;", None)));
        /// assert!(parser.parse_peek("-123;").is_err());
        /// assert_eq!(parser.parse_peek("123;"), Ok(("123;", None)));
        /// # }
        /// ```
        pub fn cond<Input, Output, Error, ParseNext>(
            cond: bool,
            mut parser: ParseNext,
        ) -> impl Parser<Input, Option<Output>, Error>
        where
            Input: Stream,
            ParseNext: Parser<Input, Output, Error>,
            Error: ParserError<Input>,
        {
            trace(
                "cond",
                move |input: &mut Input| {
                    if cond { parser.parse_next(input).map(Some) } else { Ok(None) }
                },
            )
        }
        /// Apply the parser without advancing the input.
        ///
        /// To lookahead and only advance on success, see [`opt`].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// use winnow::combinator::peek;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     peek(alpha1).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcd;"), Ok(("abcd;", "abcd")));
        /// assert!(parser.parse_peek("123;").is_err());
        /// # }
        /// ```
        #[doc(alias = "look_ahead")]
        #[doc(alias = "rewind")]
        pub fn peek<Input, Output, Error, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            ParseNext: Parser<Input, Output, Error>,
        {
            trace(
                "peek",
                move |input: &mut Input| {
                    let start = input.checkpoint();
                    let res = parser.parse_next(input);
                    input.reset(&start);
                    res
                },
            )
        }
        /// Match the end of the [`Stream`]
        ///
        /// Otherwise, it will error.
        ///
        /// # Effective Signature
        ///
        /// Assuming you are parsing a `&str` [Stream]:
        /// ```rust
        /// # use winnow::prelude::*;;
        /// pub fn eof<'i>(input: &mut &'i str) -> ModalResult<&'i str>
        /// # {
        /// #     winnow::combinator::eof.parse_next(input)
        /// # }
        /// ```
        ///
        /// # Example
        ///
        /// ```rust
        /// # use std::str;
        /// # use winnow::combinator::eof;
        /// # use winnow::prelude::*;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     eof.parse_next(input)
        /// }
        /// assert!(parser.parse_peek("abc").is_err());
        /// assert_eq!(parser.parse_peek(""), Ok(("", "")));
        /// ```
        #[doc(alias = "end")]
        #[doc(alias = "eoi")]
        pub fn eof<Input, Error>(
            input: &mut Input,
        ) -> Result<<Input as Stream>::Slice, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
        {
            trace(
                    "eof",
                    move |input: &mut Input| {
                        if input.eof_offset() == 0 {
                            Ok(input.next_slice(0))
                        } else {
                            Err(ParserError::from_input(input))
                        }
                    },
                )
                .parse_next(input)
        }
        /// Succeeds if the child parser returns an error.
        ///
        /// <div class="warning">
        ///
        /// **Note:** This does not advance the [`Stream`]
        ///
        /// </div>
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// use winnow::combinator::not;
        /// use winnow::ascii::alpha1;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<()> {
        ///     not(alpha1).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("123"), Ok(("123", ())));
        /// assert!(parser.parse_peek("abcd").is_err());
        /// # }
        /// ```
        pub fn not<Input, Output, Error, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, (), Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            ParseNext: Parser<Input, Output, Error>,
        {
            trace(
                "not",
                move |input: &mut Input| {
                    let start = input.checkpoint();
                    let res = parser.parse_next(input);
                    input.reset(&start);
                    match res {
                        Ok(_) => Err(ParserError::from_input(input)),
                        Err(e) if e.is_backtrack() => Ok(()),
                        Err(e) => Err(e),
                    }
                },
            )
        }
        /// Transforms an [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack] (recoverable) to [`ErrMode::Cut`][crate::error::ErrMode::Cut] (unrecoverable)
        ///
        /// This commits the parse result, preventing alternative branch paths like with
        /// [`winnow::combinator::alt`][crate::combinator::alt].
        ///
        /// See the [tutorial][crate::_tutorial::chapter_7] for more details.
        ///
        /// # Example
        ///
        /// Without `cut_err`:
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::token::one_of;
        /// # use winnow::token::rest;
        /// # use winnow::ascii::digit1;
        /// # use winnow::combinator::alt;
        /// # use winnow::combinator::preceded;
        /// # use winnow::prelude::*;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///   alt((
        ///     preceded(one_of(['+', '-']), digit1),
        ///     rest
        ///   )).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("+10 ab"), Ok((" ab", "10")));
        /// assert_eq!(parser.parse_peek("ab"), Ok(("", "ab")));
        /// assert_eq!(parser.parse_peek("+"), Ok(("", "+")));
        /// # }
        /// ```
        ///
        /// With `cut_err`:
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, error::ContextError};
        /// # use winnow::prelude::*;
        /// # use winnow::token::one_of;
        /// # use winnow::token::rest;
        /// # use winnow::ascii::digit1;
        /// # use winnow::combinator::alt;
        /// # use winnow::combinator::preceded;
        /// use winnow::combinator::cut_err;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///   alt((
        ///     preceded(one_of(['+', '-']), cut_err(digit1)),
        ///     rest
        ///   )).parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("+10 ab"), Ok((" ab", "10")));
        /// assert_eq!(parser.parse_peek("ab"), Ok(("", "ab")));
        /// assert_eq!(parser.parse_peek("+"), Err(ErrMode::Cut(ContextError::new())));
        /// # }
        /// ```
        pub fn cut_err<Input, Output, Error, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input> + ModalError,
            ParseNext: Parser<Input, Output, Error>,
        {
            trace(
                "cut_err",
                move |input: &mut Input| {
                    parser.parse_next(input).map_err(|e| e.cut())
                },
            )
        }
        /// Transforms an [`ErrMode::Cut`][crate::error::ErrMode::Cut] (unrecoverable) to [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack] (recoverable)
        ///
        /// This attempts the parse, allowing other parsers to be tried on failure, like with
        /// [`winnow::combinator::alt`][crate::combinator::alt].
        pub fn backtrack_err<Input, Output, Error, ParseNext>(
            mut parser: ParseNext,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input> + ModalError,
            ParseNext: Parser<Input, Output, Error>,
        {
            trace(
                "backtrack_err",
                move |input: &mut Input| {
                    parser.parse_next(input).map_err(|e| e.backtrack())
                },
            )
        }
        /// A placeholder for a not-yet-implemented [`Parser`]
        ///
        /// This is analogous to the [`todo!`] macro and helps with prototyping.
        ///
        /// # Panic
        ///
        /// This will panic when parsing
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::prelude::*;
        /// # use winnow::combinator::todo;
        ///
        /// fn parser(input: &mut &str) -> ModalResult<u64> {
        ///     todo(input)
        /// }
        /// ```
        #[track_caller]
        pub fn todo<Input, Output, Error>(input: &mut Input) -> Result<Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
        {
            #![allow(clippy::todo)]
            trace(
                    "todo",
                    move |_input: &mut Input| {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "not yet implemented: {0}",
                                    format_args!("unimplemented parse"),
                                ),
                            );
                        }
                    },
                )
                .parse_next(input)
        }
        /// Succeed, consuming no input
        ///
        /// For example, it can be used as the last alternative in `alt` to
        /// specify the default case.
        ///
        /// Useful with:
        /// - [`Parser::value`]
        /// - [`Parser::default_value`]
        /// - [`Parser::map`]
        ///
        /// <div class="warning">
        ///
        /// **Note:** This never advances the [`Stream`]
        ///
        /// </div>
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::prelude::*;
        /// use winnow::combinator::alt;
        /// use winnow::combinator::empty;
        ///
        /// fn sign(input: &mut &str) -> ModalResult<isize> {
        ///     alt((
        ///         '-'.value(-1),
        ///         '+'.value(1),
        ///         empty.value(1)
        ///     )).parse_next(input)
        /// }
        /// assert_eq!(sign.parse_peek("+10"), Ok(("10", 1)));
        /// assert_eq!(sign.parse_peek("-10"), Ok(("10", -1)));
        /// assert_eq!(sign.parse_peek("10"), Ok(("10", 1)));
        /// ```
        #[doc(alias = "value")]
        #[doc(alias = "success")]
        #[inline]
        pub fn empty<Input, Error>(_input: &mut Input) -> Result<(), Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
        {
            Ok(())
        }
        /// A parser which always fails.
        ///
        /// For example, it can be used as the last alternative in `alt` to
        /// control the error message given.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::InputError};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::fail;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<(), InputError<&'i str>> {
        ///     fail.parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("string"), Err(ErrMode::Backtrack(InputError::at("string"))));
        /// ```
        #[doc(alias = "unexpected")]
        #[inline]
        pub fn fail<Input, Output, Error>(i: &mut Input) -> Result<Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
        {
            trace("fail", |i: &mut Input| Err(ParserError::from_input(i))).parse_next(i)
        }
    }
    mod debug {
        use crate::error::ParserError;
        use crate::stream::Stream;
        use crate::Parser;
        /// Trace the execution of the parser
        ///
        /// Note that [`Parser::context`] also provides high level trace information.
        ///
        /// See [tutorial][crate::_tutorial::chapter_8] for more details.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::token::take_while;
        /// # use winnow::stream::AsChar;
        /// # use winnow::prelude::*;
        /// use winnow::combinator::trace;
        ///
        /// fn short_alpha<'s>(s: &mut &'s [u8]) -> ModalResult<&'s [u8]> {
        ///   trace("short_alpha",
        ///     take_while(3..=6, AsChar::is_alpha)
        ///   ).parse_next(s)
        /// }
        ///
        /// assert_eq!(short_alpha.parse_peek(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
        /// assert_eq!(short_alpha.parse_peek(b"lengthy"), Ok((&b"y"[..], &b"length"[..])));
        /// assert_eq!(short_alpha.parse_peek(b"latin"), Ok((&b""[..], &b"latin"[..])));
        /// assert!(short_alpha.parse_peek(b"ed").is_err());
        /// assert!(short_alpha.parse_peek(b"12345").is_err());
        /// ```
        #[allow(unused_variables)]
        #[allow(unused_mut)]
        #[inline(always)]
        pub fn trace<I: Stream, O, E: ParserError<I>>(
            name: impl core::fmt::Display,
            parser: impl Parser<I, O, E>,
        ) -> impl Parser<I, O, E> {
            { parser }
        }
        #[allow(unused_variables)]
        pub(crate) fn trace_result<T, I: Stream, E: ParserError<I>>(
            name: impl core::fmt::Display,
            res: &Result<T, E>,
        ) {}
        pub(crate) struct DisplayDebug<D>(pub(crate) D);
        impl<D: core::fmt::Debug> core::fmt::Display for DisplayDebug<D> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_fmt(format_args!("{0:?}", self.0))
            }
        }
    }
    mod expression {
        use crate::combinator::empty;
        use crate::combinator::fail;
        use crate::combinator::opt;
        use crate::combinator::trace;
        use crate::error::ParserError;
        use crate::stream::Stream;
        use crate::stream::StreamIsPartial;
        use crate::Parser;
        use crate::Result;
        /// Parses an expression based on operator precedence.
        ///
        /// It uses a Pratt parsing algorithm, where operators are
        /// associated with a binding power. The higher the power,
        /// the more tightly an operator will bind to its operands.
        ///
        /// This method returns an [`Expression`], which configures
        /// the Pratt parser.
        ///
        /// Each operator type is configured with [`Prefix`], [`Postfix`],
        /// and [`Infix`]. These describe the operator's binding power,
        /// a function that applies the operator to its operand, and the
        /// operator's associativity (infix only).
        ///
        /// For a more full-featured example, look at the [C-style Expressions][crate::_topic::language#c-style-expressions]
        /// topic.
        ///
        /// # Example
        ///
        /// Parsing a simple arithmetic expression without parenthesis.
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::error::ContextError;
        /// # use winnow::ascii::digit1;
        /// # use winnow::combinator::{dispatch, fail};
        /// # use winnow::token::any;
        /// use winnow::combinator::expression;
        /// use winnow::combinator::{Prefix, Postfix, Infix};
        ///
        /// fn parser<'i>() -> impl Parser<&'i str, i32, ContextError> {
        ///     move |i: &mut &str| {
        ///         use Infix::*;
        ///         expression(digit1.parse_to::<i32>()) // operands are 32-bit integers
        ///             .prefix(dispatch! {any;
        ///                 '-' => Prefix(12, |_, a: i32| Ok(-a)),
        ///                 _ => fail,
        ///             })
        ///             .infix(dispatch! {any;
        ///                 '+' => Left(5, |_, a, b| Ok(a + b)),
        ///                 '-' => Left(5, |_, a, b| Ok(a - b)),
        ///                 '*' => Left(7, |_, a, b| Ok(a * b)),
        ///                 '/' => Left(7, |_, a: i32, b| Ok(a.checked_div(b).unwrap_or_default())),
        ///                 _ => fail,
        ///             })
        ///             .postfix(dispatch! {any;
        ///                 '!' => Postfix(15, |_, a| if a < 1 { Ok(1) } else { Ok((1..=a).fold(1, |acc, a| acc*a)) }),
        ///                 _ => fail,
        ///             })
        ///             .parse_next(i)
        ///     }
        /// }
        ///
        /// assert_eq!(parser().parse("1+1"), Ok(2));
        /// assert_eq!(parser().parse("0!"), Ok(1));
        /// assert_eq!(parser().parse("-1*5*2*10+30/3!"), Ok(-95));
        /// # }
        /// ```
        #[doc(alias = "pratt")]
        #[doc(alias = "separated")]
        #[doc(alias = "shunting_yard")]
        #[doc(alias = "precedence_climbing")]
        #[inline(always)]
        #[allow(clippy::type_complexity)]
        pub fn expression<I, ParseOperand, O, E>(
            parse_operand: ParseOperand,
        ) -> Expression<
            I,
            O,
            ParseOperand,
            impl Parser<I, Prefix<I, O, E>, E>,
            impl Parser<I, Postfix<I, O, E>, E>,
            impl Parser<I, Infix<I, O, E>, E>,
            E,
        >
        where
            I: Stream + StreamIsPartial,
            ParseOperand: Parser<I, O, E>,
            E: ParserError<I>,
        {
            Expression {
                precedence_level: 0,
                parse_operand,
                parse_prefix: fail,
                parse_postfix: fail,
                parse_infix: fail,
                marker: Default::default(),
            }
        }
        /// A helper struct for [`expression()`].
        ///
        /// Holds the configuration for the Pratt parser, including
        /// the operator and operand parsers. A precedence level can
        /// also be set, which is useful to disambiguate parse trees
        /// based on the parent operator's precedence.
        ///
        /// Implements [`Parser`]. When parsing an input, it applies
        /// the Pratt parser.
        pub struct Expression<I, O, ParseOperand, Pre, Post, Pix, E>
        where
            I: Stream + StreamIsPartial,
            ParseOperand: Parser<I, O, E>,
            E: ParserError<I>,
        {
            precedence_level: i64,
            parse_operand: ParseOperand,
            parse_prefix: Pre,
            parse_postfix: Post,
            parse_infix: Pix,
            marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<
            I,
            O,
            ParseOperand,
            Pre,
            Post,
            Pix,
            E,
        > Expression<I, O, ParseOperand, Pre, Post, Pix, E>
        where
            ParseOperand: Parser<I, O, E>,
            I: Stream + StreamIsPartial,
            E: ParserError<I>,
        {
            /// Sets the prefix operator parser.
            ///
            /// The parser should parse the input to a [`Prefix`],
            /// which contains the operator's binding power and
            /// a fold function which applies the operator to its
            /// operands.
            #[inline(always)]
            pub fn prefix<NewParsePrefix>(
                self,
                parser: NewParsePrefix,
            ) -> Expression<I, O, ParseOperand, NewParsePrefix, Post, Pix, E>
            where
                NewParsePrefix: Parser<I, Prefix<I, O, E>, E>,
            {
                Expression {
                    precedence_level: self.precedence_level,
                    parse_operand: self.parse_operand,
                    parse_prefix: parser,
                    parse_postfix: self.parse_postfix,
                    parse_infix: self.parse_infix,
                    marker: Default::default(),
                }
            }
            /// Sets the postfix operator parser.
            ///
            /// The parser should parse the input to a [`Postfix`],
            /// which contains the operator's binding power and
            /// a fold function which applies the operator to its
            /// operands.
            #[inline(always)]
            pub fn postfix<NewParsePostfix>(
                self,
                parser: NewParsePostfix,
            ) -> Expression<I, O, ParseOperand, Pre, NewParsePostfix, Pix, E>
            where
                NewParsePostfix: Parser<I, Postfix<I, O, E>, E>,
            {
                Expression {
                    precedence_level: self.precedence_level,
                    parse_operand: self.parse_operand,
                    parse_prefix: self.parse_prefix,
                    parse_postfix: parser,
                    parse_infix: self.parse_infix,
                    marker: Default::default(),
                }
            }
            /// Sets the infix operator parser.
            ///
            /// The parser should parse the input to a [`Infix`],
            /// which contains the operator's binding power and
            /// a fold function which applies the operator to its
            /// operands.
            #[inline(always)]
            pub fn infix<NewParseInfix>(
                self,
                parser: NewParseInfix,
            ) -> Expression<I, O, ParseOperand, Pre, Post, NewParseInfix, E>
            where
                NewParseInfix: Parser<I, Infix<I, O, E>, E>,
            {
                Expression {
                    precedence_level: self.precedence_level,
                    parse_operand: self.parse_operand,
                    parse_prefix: self.parse_prefix,
                    parse_postfix: self.parse_postfix,
                    parse_infix: parser,
                    marker: Default::default(),
                }
            }
            /// Sets the precedence level for the current instance of the parser.
            ///
            /// It defaults to 0, which is traditionally treated as the "lowest"
            /// possible precedence when parsing an expression.
            ///
            /// This is useful to disambiguate grammars based on the parent operator's
            /// precedence. This comes up primarily when parsing recursive expressions.
            ///
            /// The parsing machinery underpinning [`Expression`] assumes that a "more
            /// tightly binding" operator is numerically large, while a "more loosely
            /// binding" operator is numerically small. For example, `13` is a higher
            /// precedence level than `1` because `13 > 1`.
            ///
            /// Other ways of describing this relationship:
            /// - `13` has a higher precedence compared to `1`
            /// - `13` has a higher binding power compared to `1`
            ///
            /// Note: Binding power and precedence both refer to the same concept and
            /// may be used interchangeably.
            ///
            /// # Motivation
            ///
            /// If you don't understand why this is useful to have, this section tries
            /// to explain in more detail.
            ///
            /// The [C-style Expressions][crate::_topic::arithmetic#c-style-expression]
            /// example has source code for parsing the expression described below, and
            /// can provide a clearer usage example.
            ///
            /// Consider the following expression in the C language:
            ///
            /// ```c
            /// int x = (1 == 1 ? 0 : 1, -123); // <-- let's parse this
            /// printf("%d\n", x); // -123
            /// ```
            ///
            /// Let's look at the right-hand side of the expression on the first line,
            /// and replace some of the sub-expressions with symbols:
            ///
            /// ```text
            /// (1 == 1 ? 0 : 1, -123) // rhs
            /// (a      ? b : c, d  )  // symbolic
            /// (a ? b : c, d)         // remove whitespace
            /// (, (? a b c) d)        // prefix notation
            /// ```
            ///
            /// Written symbolically:
            /// - `a` is the condition, like `1 == 1`
            /// - `b` is the value when the condition is true
            /// - `c` is the value when the condition is false
            /// - `d` is a secondary expression unrelated to the ternary
            ///
            /// In prefix notation, it's easier to see the specific operators and what
            /// they bind to:
            /// - COMMA (`,`) binds to `(? a b c)` and `d`
            /// - TERNARY (`?`) binds to `a`, `b`, and `c`
            ///
            /// ## Parsing `c` and `d`
            ///
            /// Let's focus on parsing the sub-expressions `c` and `d`, as that
            /// motivates why a parser precedence level is necessary.
            ///
            /// To parse `c`, we would really like to re-use the parser produced by
            /// [`expression()`], because `c` is really *any* valid expression that
            /// can be parsed by `expression()` already.
            ///
            /// However, we can't re-use the parser naively. When parsing `c`, we need
            /// to "escape" from the inner parser when encountering the comma separating
            /// `c` from `d`.
            ///
            /// The reason we have to "escape" is because of how operator precedence is
            /// defined in the C language: the comma operator has the lowest precedence
            /// among all the operators. When we're parsing `c`, we're in the context of
            /// the ternary operator. We don't want to parse any valid expression! Just
            /// what the ternary operator captures.
            ///
            /// That's where the precedence level comes in: you specify the minimum
            /// precedence this parser is willing to accept. If you come across an
            /// expression in the top-level with a lower binding power than the starting
            /// precedence, you know to stop parsing.
            ///
            /// The parsing machinery inside of [`Expression`] handles most of this for
            /// you, but it can't determine what the precedence level should be for a
            /// given expression. That's a language-specific detail, and it depends on
            /// what you want to parse.
            #[inline(always)]
            pub fn current_precedence_level(
                mut self,
                level: i64,
            ) -> Expression<I, O, ParseOperand, Pre, Post, Pix, E> {
                self.precedence_level = level;
                self
            }
        }
        impl<I, O, Pop, Pre, Post, Pix, E> Parser<I, O, E>
        for Expression<I, O, Pop, Pre, Post, Pix, E>
        where
            I: Stream + StreamIsPartial,
            Pop: Parser<I, O, E>,
            Pix: Parser<I, Infix<I, O, E>, E>,
            Pre: Parser<I, Prefix<I, O, E>, E>,
            Post: Parser<I, Postfix<I, O, E>, E>,
            E: ParserError<I>,
        {
            #[inline(always)]
            fn parse_next(&mut self, input: &mut I) -> Result<O, E> {
                trace(
                        "expression",
                        move |i: &mut I| {
                            expression_impl(
                                i,
                                &mut self.parse_operand,
                                &mut self.parse_prefix,
                                &mut self.parse_postfix,
                                &mut self.parse_infix,
                                self.precedence_level,
                            )
                        },
                    )
                    .parse_next(input)
            }
        }
        /// Opaque implementation of the Pratt parser.
        fn expression_impl<I, O, Pop, Pre, Post, Pix, E>(
            i: &mut I,
            parse_operand: &mut Pop,
            prefix: &mut Pre,
            postfix: &mut Post,
            infix: &mut Pix,
            min_power: i64,
        ) -> Result<O, E>
        where
            I: Stream + StreamIsPartial,
            Pop: Parser<I, O, E>,
            Pix: Parser<I, Infix<I, O, E>, E>,
            Pre: Parser<I, Prefix<I, O, E>, E>,
            Post: Parser<I, Postfix<I, O, E>, E>,
            E: ParserError<I>,
        {
            let operand = opt(trace("operand", parse_operand.by_ref())).parse_next(i)?;
            let mut operand = if let Some(operand) = operand {
                operand
            } else {
                let len = i.eof_offset();
                let Prefix(power, fold_prefix) = trace("prefix", prefix.by_ref())
                    .parse_next(i)?;
                if i.eof_offset() == len {
                    return Err(E::assert(i, "`prefix` parsers must always consume"));
                }
                let operand = expression_impl(
                    i,
                    parse_operand,
                    prefix,
                    postfix,
                    infix,
                    power,
                )?;
                fold_prefix(i, operand)?
            };
            let mut prev_op_is_neither = None;
            'parse: while i.eof_offset() > 0 {
                let start = i.checkpoint();
                if let Some(Postfix(power, fold_postfix)) = opt(
                        trace("postfix", postfix.by_ref()),
                    )
                    .parse_next(i)?
                {
                    if power < min_power {
                        i.reset(&start);
                        break 'parse;
                    }
                    operand = fold_postfix(i, operand)?;
                    continue 'parse;
                }
                let start = i.checkpoint();
                let parse_result = opt(trace("infix", infix.by_ref())).parse_next(i)?;
                if let Some(infix_op) = parse_result {
                    let mut is_neither = None;
                    let (lpower, rpower, fold_infix) = match infix_op {
                        Infix::Right(p, f) => (p, p - 1, f),
                        Infix::Left(p, f) => (p, p + 1, f),
                        Infix::Neither(p, f) => {
                            is_neither = Some(p);
                            (p, p + 1, f)
                        }
                    };
                    if lpower < min_power
                        || match prev_op_is_neither {
                            None => false,
                            Some(p) => lpower == p,
                        }
                    {
                        i.reset(&start);
                        break 'parse;
                    }
                    prev_op_is_neither = is_neither;
                    let rhs = expression_impl(
                        i,
                        parse_operand,
                        prefix,
                        postfix,
                        infix,
                        rpower,
                    )?;
                    operand = fold_infix(i, operand, rhs)?;
                    continue 'parse;
                }
                break 'parse;
            }
            Ok(operand)
        }
        /// Define an [`expression()`]'s prefix operator
        ///
        /// It requires an operator binding power, as well as a
        /// fold function which applies the operator.
        pub struct Prefix<I, O, E>(
            /// Binding power
            pub i64,
            /// Unary operator
            pub fn(&mut I, O) -> Result<O, E>,
        );
        impl<I, O, E> Clone for Prefix<I, O, E> {
            #[inline(always)]
            fn clone(&self) -> Self {
                Prefix(self.0, self.1)
            }
        }
        impl<I: Stream, O, E: ParserError<I>> Parser<I, Prefix<I, O, E>, E>
        for Prefix<I, O, E> {
            #[inline(always)]
            fn parse_next(&mut self, input: &mut I) -> Result<Prefix<I, O, E>, E> {
                empty.value(self.clone()).parse_next(input)
            }
        }
        /// Define an [`expression()`]'s postfix operator
        ///
        /// It requires an operator binding power, as well as a
        /// fold function which applies the operator.
        pub struct Postfix<I, O, E>(
            /// Binding power
            pub i64,
            /// Unary operator
            pub fn(&mut I, O) -> Result<O, E>,
        );
        impl<I, O, E> Clone for Postfix<I, O, E> {
            #[inline(always)]
            fn clone(&self) -> Self {
                Postfix(self.0, self.1)
            }
        }
        impl<I: Stream, O, E: ParserError<I>> Parser<I, Postfix<I, O, E>, E>
        for Postfix<I, O, E> {
            #[inline(always)]
            fn parse_next(&mut self, input: &mut I) -> Result<Postfix<I, O, E>, E> {
                empty.value(self.clone()).parse_next(input)
            }
        }
        /// Define an [`expression()`]'s infix operator
        ///
        /// It requires an operator binding power, as well as a
        /// fold function which applies the operator.
        pub enum Infix<I, O, E> {
            /// Left-associative operator
            ///
            /// The operators will bind more tightly to their rightmost operands.
            ///
            /// e.g `A op B op C` -> `(A op B) op C`
            Left(
                /// Binding power
                i64,
                /// Binary operator
                fn(&mut I, O, O) -> Result<O, E>,
            ),
            /// Right-associative operator
            ///
            /// The operators will bind more tightly to their leftmost operands.
            ///
            /// e.g `A op B op C` -> `A op (B op C)`
            Right(
                /// Binding power
                i64,
                /// Binary operator
                fn(&mut I, O, O) -> Result<O, E>,
            ),
            /// Neither left or right associative
            ///
            /// `Infix::Neither` has similar associativity rules as `Assoc::Left`, but we stop
            /// parsing when the next operator is the same as the current one.
            ///
            /// e.g. `a == b == c` -> `(a == b)`, fail: `(== c)`
            Neither(
                /// Binding power
                i64,
                /// Binary operator
                fn(&mut I, O, O) -> Result<O, E>,
            ),
        }
        impl<I, O, E> Clone for Infix<I, O, E> {
            #[inline(always)]
            fn clone(&self) -> Self {
                match self {
                    Infix::Left(p, f) => Infix::Left(*p, *f),
                    Infix::Right(p, f) => Infix::Right(*p, *f),
                    Infix::Neither(p, f) => Infix::Neither(*p, *f),
                }
            }
        }
        impl<I: Stream, O, E: ParserError<I>> Parser<I, Infix<I, O, E>, E>
        for Infix<I, O, E> {
            #[inline(always)]
            fn parse_next(&mut self, input: &mut I) -> Result<Infix<I, O, E>, E> {
                empty.value(self.clone()).parse_next(input)
            }
        }
    }
    mod multi {
        //! Combinators applying their child parser multiple times
        use crate::combinator::trace;
        use crate::error::FromExternalError;
        use crate::error::ParserError;
        use crate::stream::Accumulate;
        use crate::stream::Range;
        use crate::stream::Stream;
        use crate::Parser;
        use crate::Result;
        /// Repeats the embedded parser, lazily returning the results
        ///
        /// This can serve as a building block for custom parsers like [`repeat`].
        /// To iterate over all of the input in your application, see [`Parser::parse_iter`].
        ///
        /// Call the iterator's [`ParserIterator::finish`] method to get the remaining input if successful,
        /// or the error value if we encountered an error.
        ///
        /// On [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack], iteration will stop. To instead chain an error up, see [`cut_err`][crate::combinator::cut_err].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::prelude::*;
        /// # use winnow::Result;
        /// use winnow::{combinator::iterator, ascii::alpha1, combinator::terminated};
        /// use std::collections::HashMap;
        ///
        /// let mut data = "abc|defg|hijkl|mnopqr|123";
        /// let mut it = iterator(&mut data, terminated(alpha1, "|"));
        ///
        /// let parsed = it.map(|v| (v, v.len())).collect::<HashMap<_,_>>();
        /// let res: Result<_> = it.finish();
        ///
        /// assert_eq!(parsed, [("abc", 3usize), ("defg", 4), ("hijkl", 5), ("mnopqr", 6)].iter().cloned().collect());
        /// assert_eq!(data, "123");
        /// # }
        /// ```
        pub fn iterator<Input, Output, Error, ParseNext>(
            input: &mut Input,
            parser: ParseNext,
        ) -> ParserIterator<'_, ParseNext, Input, Output, Error>
        where
            ParseNext: Parser<Input, Output, Error>,
            Input: Stream,
            Error: ParserError<Input>,
        {
            ParserIterator {
                parser,
                input,
                state: State::Running,
                marker: Default::default(),
            }
        }
        /// Main structure associated to [`iterator`].
        pub struct ParserIterator<'i, F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
        {
            parser: F,
            input: &'i mut I,
            state: State<E>,
            marker: core::marker::PhantomData<O>,
        }
        impl<F, I, O, E> ParserIterator<'_, F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
            E: ParserError<I>,
        {
            /// Returns the remaining input if parsing was successful, or the error if we encountered an error.
            pub fn finish(self) -> Result<(), E> {
                match self.state {
                    State::Running | State::Done => Ok(()),
                    State::Cut(e) => Err(e),
                }
            }
        }
        impl<F, I, O, E> core::iter::Iterator for &mut ParserIterator<'_, F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
            E: ParserError<I>,
        {
            type Item = O;
            fn next(&mut self) -> Option<Self::Item> {
                if #[allow(non_exhaustive_omitted_patterns)]
                match self.state {
                    State::Running => true,
                    _ => false,
                } {
                    let start = self.input.checkpoint();
                    match self.parser.parse_next(self.input) {
                        Ok(o) => {
                            self.state = State::Running;
                            Some(o)
                        }
                        Err(e) if e.is_backtrack() => {
                            self.input.reset(&start);
                            self.state = State::Done;
                            None
                        }
                        Err(e) => {
                            self.state = State::Cut(e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
        }
        enum State<E> {
            Running,
            Done,
            Cut(E),
        }
        /// [`Accumulate`] the output of a parser into a container, like `Vec`
        ///
        /// This stops before `n` when the parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
        /// [`cut_err`][crate::combinator::cut_err].
        ///
        /// To take a series of tokens, [`Accumulate`] into a `()`
        /// (e.g. with [`.map(|()| ())`][Parser::map])
        /// and then [`Parser::take`].
        ///
        /// <div class="warning">
        ///
        /// **Warning:** If the parser passed to `repeat` accepts empty inputs
        /// (like `alpha0` or `digit0`), `repeat` will return an error,
        /// to prevent going into an infinite loop.
        ///
        /// </div>
        ///
        /// # Example
        ///
        /// Zero or more repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::repeat;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   repeat(0.., "abc").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
        /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", vec![])));
        /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
        /// # }
        /// ```
        ///
        /// One or more repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::repeat;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   repeat(1.., "abc").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
        /// assert!(parser.parse_peek("123123").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// # }
        /// ```
        ///
        /// Fixed number of repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::repeat;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   repeat(2, "abc").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
        /// assert!(parser.parse_peek("abc123").is_err());
        /// assert!(parser.parse_peek("123123").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// assert_eq!(parser.parse_peek("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
        /// # }
        /// ```
        ///
        /// Arbitrary repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::repeat;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   repeat(0..=2, "abc").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
        /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", vec![])));
        /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
        /// assert_eq!(parser.parse_peek("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
        /// # }
        /// ```
        #[doc(alias = "many0")]
        #[doc(alias = "count")]
        #[doc(alias = "many0_count")]
        #[doc(alias = "many1")]
        #[doc(alias = "many1_count")]
        #[doc(alias = "many_m_n")]
        #[doc(alias = "repeated")]
        #[doc(alias = "skip_many")]
        #[doc(alias = "skip_many1")]
        #[inline(always)]
        pub fn repeat<Input, Output, Accumulator, Error, ParseNext>(
            occurrences: impl Into<Range>,
            parser: ParseNext,
        ) -> Repeat<ParseNext, Input, Output, Accumulator, Error>
        where
            Input: Stream,
            Accumulator: Accumulate<Output>,
            ParseNext: Parser<Input, Output, Error>,
            Error: ParserError<Input>,
        {
            Repeat {
                occurrences: occurrences.into(),
                parser,
                marker: Default::default(),
            }
        }
        /// Customizable [`Parser`] implementation for [`repeat`]
        pub struct Repeat<P, I, O, C, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            C: Accumulate<O>,
            E: ParserError<I>,
        {
            occurrences: Range,
            parser: P,
            marker: core::marker::PhantomData<(I, O, C, E)>,
        }
        impl<ParseNext, Input, Output, Error> Repeat<ParseNext, Input, Output, (), Error>
        where
            ParseNext: Parser<Input, Output, Error>,
            Input: Stream,
            Error: ParserError<Input>,
        {
            /// Repeats the embedded parser, calling `op` to gather the results
            ///
            /// This stops before `n` when the parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
            /// [`cut_err`][crate::combinator::cut_err].
            ///
            /// # Arguments
            /// * `init` A function returning the initial value.
            /// * `op` The function that combines a result of `f` with
            ///   the current accumulator.
            ///
            /// <div class="warning">
            ///
            /// **Warning:** If the parser passed to [`repeat`] accepts empty inputs
            /// (like `alpha0` or `digit0`), `fold` will return an error,
            /// to prevent going into an infinite loop.
            ///
            /// </div>
            ///
            /// # Example
            ///
            /// Zero or more repetitions:
            /// ```rust
            /// # use winnow::{error::ErrMode, error::Needed};
            /// # use winnow::prelude::*;
            /// use winnow::combinator::repeat;
            ///
            /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
            ///   repeat(
            ///     0..,
            ///     "abc"
            ///   ).fold(
            ///     Vec::new,
            ///     |mut acc: Vec<_>, item| {
            ///       acc.push(item);
            ///       acc
            ///     }
            ///   ).parse_next(s)
            /// }
            ///
            /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
            /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
            /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", vec![])));
            /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
            /// ```
            ///
            /// One or more repetitions:
            /// ```rust
            /// # use winnow::{error::ErrMode, error::Needed};
            /// # use winnow::prelude::*;
            /// use winnow::combinator::repeat;
            ///
            /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
            ///   repeat(
            ///     1..,
            ///     "abc",
            ///   ).fold(
            ///     Vec::new,
            ///     |mut acc: Vec<_>, item| {
            ///       acc.push(item);
            ///       acc
            ///     }
            ///   ).parse_next(s)
            /// }
            ///
            /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
            /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
            /// assert!(parser.parse_peek("123123").is_err());
            /// assert!(parser.parse_peek("").is_err());
            /// ```
            ///
            /// Arbitrary number of repetitions:
            /// ```rust
            /// # use winnow::{error::ErrMode, error::Needed};
            /// # use winnow::prelude::*;
            /// use winnow::combinator::repeat;
            ///
            /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
            ///   repeat(
            ///     0..=2,
            ///     "abc",
            ///   ).fold(
            ///     Vec::new,
            ///     |mut acc: Vec<_>, item| {
            ///       acc.push(item);
            ///       acc
            ///     }
            ///   ).parse_next(s)
            /// }
            ///
            /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", vec!["abc", "abc"])));
            /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", vec!["abc"])));
            /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", vec![])));
            /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
            /// assert_eq!(parser.parse_peek("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
            /// ```
            #[doc(alias = "fold_many0")]
            #[doc(alias = "fold_many1")]
            #[doc(alias = "fold_many_m_n")]
            #[doc(alias = "fold_repeat")]
            #[inline(always)]
            pub fn fold<Init, Op, Result>(
                mut self,
                mut init: Init,
                mut op: Op,
            ) -> impl Parser<Input, Result, Error>
            where
                Init: FnMut() -> Result,
                Op: FnMut(Result, Output) -> Result,
            {
                let Range { start_inclusive, end_inclusive } = self.occurrences;
                trace(
                    "repeat_fold",
                    move |i: &mut Input| {
                        match (start_inclusive, end_inclusive) {
                            (0, None) => {
                                fold_repeat0_(&mut self.parser, &mut init, &mut op, i)
                            }
                            (1, None) => {
                                fold_repeat1_(&mut self.parser, &mut init, &mut op, i)
                            }
                            (start, end) if Some(start) == end => {
                                fold_repeat_n_(
                                    start,
                                    &mut self.parser,
                                    &mut init,
                                    &mut op,
                                    i,
                                )
                            }
                            (start, end) => {
                                fold_repeat_m_n_(
                                    start,
                                    end.unwrap_or(usize::MAX),
                                    &mut self.parser,
                                    &mut init,
                                    &mut op,
                                    i,
                                )
                            }
                        }
                    },
                )
            }
            /// Akin to [`Repeat::fold`], but for containers that can reject an element.
            ///
            /// This stops before `n` when the parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
            /// [`cut_err`][crate::combinator::cut_err]. Additionally, if the fold function returns `None`, the parser will
            /// stop and return an error.
            ///
            /// # Arguments
            /// * `init` A function returning the initial value.
            /// * `op` The function that combines a result of `f` with
            ///   the current accumulator.
            ///
            /// <div class="warning">
            ///
            /// **Warning:** If the parser passed to [`repeat`] accepts empty inputs
            /// (like `alpha0` or `digit0`), `verify_fold` will return an error,
            /// to prevent going into an infinite loop.
            ///
            /// </div>
            ///
            /// # Example
            ///
            /// Guaranteeing that the input had unique elements:
            /// ```rust
            /// # use winnow::{error::ErrMode, error::Needed};
            /// # use winnow::prelude::*;
            /// use winnow::combinator::repeat;
            /// use std::collections::HashSet;
            ///
            /// fn parser<'i>(s: &mut &'i str) -> ModalResult<HashSet<&'i str>> {
            ///   repeat(
            ///     0..,
            ///     "abc"
            ///   ).verify_fold(
            ///     HashSet::new,
            ///     |mut acc: HashSet<_>, item| {
            ///       if acc.insert(item) {
            ///          Some(acc)
            ///       } else {
            ///          None
            ///       }
            ///     }
            ///   ).parse_next(s)
            /// }
            ///
            /// assert_eq!(parser.parse_peek("abc"), Ok(("", HashSet::from(["abc"]))));
            /// assert!(parser.parse_peek("abcabc").is_err());
            /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", HashSet::from(["abc"]))));
            /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", HashSet::from([]))));
            /// assert_eq!(parser.parse_peek(""), Ok(("", HashSet::from([]))));
            /// ```
            #[inline(always)]
            pub fn verify_fold<Init, Op, Result>(
                mut self,
                mut init: Init,
                mut op: Op,
            ) -> impl Parser<Input, Result, Error>
            where
                Init: FnMut() -> Result,
                Op: FnMut(Result, Output) -> Option<Result>,
            {
                let Range { start_inclusive, end_inclusive } = self.occurrences;
                trace(
                    "repeat_verify_fold",
                    move |input: &mut Input| {
                        verify_fold_m_n(
                            start_inclusive,
                            end_inclusive.unwrap_or(usize::MAX),
                            &mut self.parser,
                            &mut init,
                            &mut op,
                            input,
                        )
                    },
                )
            }
            /// Akin to [`Repeat::fold`], but for containers that can error when an element is accumulated.
            ///
            /// This stops before `n` when the parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
            /// [`cut_err`][crate::combinator::cut_err]. Additionally, if the fold function returns an error, the parser will
            /// stop and return it.
            ///
            /// # Arguments
            /// * `init` A function returning the initial value.
            /// * `op` The function that combines a result of `f` with
            ///   the current accumulator.
            ///
            /// <div class="warning">
            ///
            /// **Warning:** If the parser passed to [`repeat`] accepts empty inputs
            /// (like `alpha0` or `digit0`), `try_fold` will return an error,
            /// to prevent going into an infinite loop.
            ///
            /// </div>
            ///
            /// # Example
            ///
            /// Writing the output to a vector of bytes:
            /// ```rust
            /// # use winnow::{error::ErrMode, error::Needed};
            /// # use winnow::prelude::*;
            /// use winnow::combinator::repeat;
            /// use std::io::Write;
            /// use std::io::Error;
            ///
            /// fn parser(s: &mut &str) -> ModalResult<Vec<u8>> {
            ///   repeat(
            ///     0..,
            ///     "abc"
            ///   ).try_fold(
            ///     Vec::new,
            ///     |mut acc, item: &str| -> Result<_, Error> {
            ///       acc.write(item.as_bytes())?;
            ///       Ok(acc)
            ///     }
            ///   ).parse_next(s)
            /// }
            ///
            /// assert_eq!(parser.parse_peek("abc"), Ok(("", b"abc".to_vec())));
            /// assert_eq!(parser.parse_peek("abc123"), Ok(("123", b"abc".to_vec())));
            /// assert_eq!(parser.parse_peek("123123"), Ok(("123123", vec![])));
            /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
            #[inline(always)]
            pub fn try_fold<Init, Op, OpError, Result>(
                mut self,
                mut init: Init,
                mut op: Op,
            ) -> impl Parser<Input, Result, Error>
            where
                Init: FnMut() -> Result,
                Op: FnMut(Result, Output) -> core::result::Result<Result, OpError>,
                Error: FromExternalError<Input, OpError>,
            {
                let Range { start_inclusive, end_inclusive } = self.occurrences;
                trace(
                    "repeat_try_fold",
                    move |input: &mut Input| {
                        try_fold_m_n(
                            start_inclusive,
                            end_inclusive.unwrap_or(usize::MAX),
                            &mut self.parser,
                            &mut init,
                            &mut op,
                            input,
                        )
                    },
                )
            }
        }
        impl<P, I, O, C, E> Parser<I, C, E> for Repeat<P, I, O, C, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            C: Accumulate<O>,
            E: ParserError<I>,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<C, E> {
                let Range { start_inclusive, end_inclusive } = self.occurrences;
                trace(
                        "repeat",
                        move |i: &mut I| {
                            match (start_inclusive, end_inclusive) {
                                (0, None) => {
                                    fold_repeat0_(
                                        &mut self.parser,
                                        &mut || C::initial(None),
                                        &mut |mut acc, o| {
                                            acc.accumulate(o);
                                            acc
                                        },
                                        i,
                                    )
                                }
                                (1, None) => {
                                    fold_repeat1_(
                                        &mut self.parser,
                                        &mut || C::initial(None),
                                        &mut |mut acc, o| {
                                            acc.accumulate(o);
                                            acc
                                        },
                                        i,
                                    )
                                }
                                (min, end) if Some(min) == end => {
                                    fold_repeat_n_(
                                        min,
                                        &mut self.parser,
                                        &mut || C::initial(Some(min)),
                                        &mut |mut acc, o| {
                                            acc.accumulate(o);
                                            acc
                                        },
                                        i,
                                    )
                                }
                                (min, end) => {
                                    fold_repeat_m_n_(
                                        min,
                                        end.unwrap_or(usize::MAX),
                                        &mut self.parser,
                                        &mut || C::initial(Some(min)),
                                        &mut |mut acc, o| {
                                            acc.accumulate(o);
                                            acc
                                        },
                                        i,
                                    )
                                }
                            }
                        },
                    )
                    .parse_next(i)
            }
        }
        fn fold_repeat0_<I, O, E, P, N, F, R>(
            parser: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> R,
            E: ParserError<I>,
        {
            let mut res = init();
            loop {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match parser.parse_next(input) {
                    Ok(output) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`repeat` parsers must always consume",
                                ),
                            );
                        }
                        res = fold(res, output);
                    }
                    Err(err) if err.is_backtrack() => {
                        input.reset(&start);
                        return Ok(res);
                    }
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        }
        fn fold_repeat1_<I, O, E, P, N, F, R>(
            parser: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> R,
            E: ParserError<I>,
        {
            let start = input.checkpoint();
            match parser.parse_next(input) {
                Err(err) => Err(err.append(input, &start)),
                Ok(output) => {
                    let init = init();
                    let mut res = fold(init, output);
                    loop {
                        let start = input.checkpoint();
                        let len = input.eof_offset();
                        match parser.parse_next(input) {
                            Err(err) if err.is_backtrack() => {
                                input.reset(&start);
                                break;
                            }
                            Err(err) => return Err(err),
                            Ok(output) => {
                                if input.eof_offset() == len {
                                    return Err(
                                        ParserError::assert(
                                            input,
                                            "`repeat` parsers must always consume",
                                        ),
                                    );
                                }
                                res = fold(res, output);
                            }
                        }
                    }
                    Ok(res)
                }
            }
        }
        fn fold_repeat_n_<I, O, E, P, N, F, R>(
            count: usize,
            parse: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> R,
            E: ParserError<I>,
        {
            let mut res = init();
            for _ in 0..count {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match parse.parse_next(input) {
                    Ok(output) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`repeat` parsers must always consume",
                                ),
                            );
                        }
                        res = fold(res, output);
                    }
                    Err(err) => {
                        return Err(err.append(input, &start));
                    }
                }
            }
            Ok(res)
        }
        fn fold_repeat_m_n_<I, O, E, P, N, F, R>(
            min: usize,
            max: usize,
            parse: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> R,
            E: ParserError<I>,
        {
            if min > max {
                return Err(
                    ParserError::assert(
                        input,
                        "range should be ascending, rather than descending",
                    ),
                );
            }
            let mut res = init();
            for count in 0..max {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match parse.parse_next(input) {
                    Ok(output) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`repeat` parsers must always consume",
                                ),
                            );
                        }
                        res = fold(res, output);
                    }
                    Err(err) if err.is_backtrack() => {
                        if count < min {
                            return Err(err.append(input, &start));
                        } else {
                            input.reset(&start);
                            break;
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(res)
        }
        fn verify_fold_m_n<I, O, E, P, N, F, R>(
            min: usize,
            max: usize,
            parse: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> Option<R>,
            E: ParserError<I>,
        {
            if min > max {
                return Err(
                    ParserError::assert(
                        input,
                        "range should be ascending, rather than descending",
                    ),
                );
            }
            let mut res = init();
            for count in 0..max {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match parse.parse_next(input) {
                    Ok(output) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`repeat` parsers must always consume",
                                ),
                            );
                        }
                        let Some(res_) = fold(res, output) else {
                            input.reset(&start);
                            let res = Err(ParserError::from_input(input));
                            super::debug::trace_result("verify_fold", &res);
                            return res;
                        };
                        res = res_;
                    }
                    Err(err) if err.is_backtrack() => {
                        if count < min {
                            return Err(err.append(input, &start));
                        } else {
                            input.reset(&start);
                            break;
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(res)
        }
        fn try_fold_m_n<I, O, E, P, N, F, R, RE>(
            min: usize,
            max: usize,
            parse: &mut P,
            init: &mut N,
            fold: &mut F,
            input: &mut I,
        ) -> Result<R, E>
        where
            I: Stream,
            P: Parser<I, O, E>,
            N: FnMut() -> R,
            F: FnMut(R, O) -> Result<R, RE>,
            E: ParserError<I> + FromExternalError<I, RE>,
        {
            if min > max {
                return Err(
                    ParserError::assert(
                        input,
                        "range should be ascending, rather than descending",
                    ),
                );
            }
            let mut res = init();
            for count in 0..max {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match parse.parse_next(input) {
                    Ok(output) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`repeat` parsers must always consume",
                                ),
                            );
                        }
                        match fold(res, output) {
                            Ok(res_) => res = res_,
                            Err(err) => {
                                input.reset(&start);
                                let res = Err(E::from_external_error(input, err));
                                super::debug::trace_result("try_fold", &res);
                                return res;
                            }
                        }
                    }
                    Err(err) if err.is_backtrack() => {
                        if count < min {
                            return Err(err.append(input, &start));
                        } else {
                            input.reset(&start);
                            break;
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(res)
        }
        /// [`Accumulate`] the output of parser `f` into a container, like `Vec`, until the parser `g`
        /// produces a result.
        ///
        /// Returns a tuple of the results of `f` in a `Vec` and the result of `g`.
        ///
        /// `f` keeps going so long as `g` produces [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see [`cut_err`][crate::combinator::cut_err].
        ///
        /// To take a series of tokens, [`Accumulate`] into a `()`
        /// (e.g. with [`.map(|((), _)| ())`][Parser::map])
        /// and then [`Parser::take`].
        ///
        /// See also
        /// - [`take_till`][crate::token::take_till] for recognizing up-to a member of a [set of tokens][crate::stream::ContainsToken]
        /// - [`take_until`][crate::token::take_until] for recognizing up-to a [`literal`][crate::token::literal] (w/ optional simd optimizations)
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::repeat_till;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<(Vec<&'i str>, &'i str)> {
        ///   repeat_till(0.., "abc", "end").parse_next(s)
        /// };
        ///
        /// assert_eq!(parser.parse_peek("abcabcend"), Ok(("", (vec!["abc", "abc"], "end"))));
        /// assert!(parser.parse_peek("abc123end").is_err());
        /// assert!(parser.parse_peek("123123end").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// assert_eq!(parser.parse_peek("abcendefg"), Ok(("efg", (vec!["abc"], "end"))));
        /// # }
        /// ```
        #[doc(alias = "many_till0")]
        pub fn repeat_till<
            Input,
            Output,
            Accumulator,
            Terminator,
            Error,
            ParseNext,
            TerminatorParser,
        >(
            occurrences: impl Into<Range>,
            mut parse: ParseNext,
            mut terminator: TerminatorParser,
        ) -> impl Parser<Input, (Accumulator, Terminator), Error>
        where
            Input: Stream,
            Accumulator: Accumulate<Output>,
            ParseNext: Parser<Input, Output, Error>,
            TerminatorParser: Parser<Input, Terminator, Error>,
            Error: ParserError<Input>,
        {
            let Range { start_inclusive, end_inclusive } = occurrences.into();
            trace(
                "repeat_till",
                move |i: &mut Input| {
                    match (start_inclusive, end_inclusive) {
                        (0, None) => repeat_till0_(&mut parse, &mut terminator, i),
                        (start, end) => {
                            repeat_till_m_n_(
                                start,
                                end.unwrap_or(usize::MAX),
                                &mut parse,
                                &mut terminator,
                                i,
                            )
                        }
                    }
                },
            )
        }
        fn repeat_till0_<I, O, C, P, E, F, G>(
            f: &mut F,
            g: &mut G,
            i: &mut I,
        ) -> Result<(C, P), E>
        where
            I: Stream,
            C: Accumulate<O>,
            F: Parser<I, O, E>,
            G: Parser<I, P, E>,
            E: ParserError<I>,
        {
            let mut res = C::initial(None);
            loop {
                let start = i.checkpoint();
                let len = i.eof_offset();
                match g.parse_next(i) {
                    Ok(o) => return Ok((res, o)),
                    Err(e) if e.is_backtrack() => {
                        i.reset(&start);
                        match f.parse_next(i) {
                            Err(e) => return Err(e.append(i, &start)),
                            Ok(o) => {
                                if i.eof_offset() == len {
                                    return Err(
                                        ParserError::assert(
                                            i,
                                            "`repeat` parsers must always consume",
                                        ),
                                    );
                                }
                                res.accumulate(o);
                            }
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        fn repeat_till_m_n_<I, O, C, P, E, F, G>(
            min: usize,
            max: usize,
            f: &mut F,
            g: &mut G,
            i: &mut I,
        ) -> Result<(C, P), E>
        where
            I: Stream,
            C: Accumulate<O>,
            F: Parser<I, O, E>,
            G: Parser<I, P, E>,
            E: ParserError<I>,
        {
            if min > max {
                return Err(
                    ParserError::assert(
                        i,
                        "range should be ascending, rather than descending",
                    ),
                );
            }
            let mut res = C::initial(Some(min));
            let start = i.checkpoint();
            for _ in 0..min {
                match f.parse_next(i) {
                    Ok(o) => {
                        res.accumulate(o);
                    }
                    Err(e) => {
                        return Err(e.append(i, &start));
                    }
                }
            }
            for count in min..=max {
                let start = i.checkpoint();
                let len = i.eof_offset();
                match g.parse_next(i) {
                    Ok(o) => return Ok((res, o)),
                    Err(err) if err.is_backtrack() => {
                        if count == max {
                            return Err(err);
                        }
                        i.reset(&start);
                        match f.parse_next(i) {
                            Err(e) => {
                                return Err(e.append(i, &start));
                            }
                            Ok(o) => {
                                if i.eof_offset() == len {
                                    return Err(
                                        ParserError::assert(
                                            i,
                                            "`repeat` parsers must always consume",
                                        ),
                                    );
                                }
                                res.accumulate(o);
                            }
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            ::core::panicking::panic("internal error: entered unreachable code")
        }
        /// [`Accumulate`] the output of a parser, interleaved with `sep`
        ///
        /// This stops when either parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
        /// [`cut_err`][crate::combinator::cut_err].
        ///
        /// To take a series of tokens, [`Accumulate`] into a `()`
        /// (e.g. with [`.map(|()| ())`][Parser::map])
        /// and then [`Parser::take`].
        ///
        /// <div class="warning">
        ///
        /// **Warning:** If the separator parser accepts empty inputs
        /// (like `alpha0` or `digit0`), `separated` will return an error,
        /// to prevent going into an infinite loop.
        ///
        /// </div>
        ///
        /// # Example
        ///
        /// Zero or more repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   separated(0.., "abc", "|").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abc|abc|abc"), Ok(("", vec!["abc", "abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123abc"), Ok(("123abc", vec!["abc"])));
        /// assert_eq!(parser.parse_peek("abc|def"), Ok(("|def", vec!["abc"])));
        /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
        /// assert_eq!(parser.parse_peek("def|abc"), Ok(("def|abc", vec![])));
        /// # }
        /// ```
        ///
        /// One or more repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   separated(1.., "abc", "|").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abc|abc|abc"), Ok(("", vec!["abc", "abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123abc"), Ok(("123abc", vec!["abc"])));
        /// assert_eq!(parser.parse_peek("abc|def"), Ok(("|def", vec!["abc"])));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("def|abc").is_err());
        /// # }
        /// ```
        ///
        /// Fixed number of repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   separated(2, "abc", "|").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abc|abc|abc"), Ok(("|abc", vec!["abc", "abc"])));
        /// assert!(parser.parse_peek("abc123abc").is_err());
        /// assert!(parser.parse_peek("abc|def").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("def|abc").is_err());
        /// # }
        /// ```
        ///
        /// Arbitrary repetitions:
        /// ```rust
        /// # #[cfg(feature = "std")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<Vec<&'i str>> {
        ///   separated(0..=2, "abc", "|").parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abc|abc|abc"), Ok(("|abc", vec!["abc", "abc"])));
        /// assert_eq!(parser.parse_peek("abc123abc"), Ok(("123abc", vec!["abc"])));
        /// assert_eq!(parser.parse_peek("abc|def"), Ok(("|def", vec!["abc"])));
        /// assert_eq!(parser.parse_peek(""), Ok(("", vec![])));
        /// assert_eq!(parser.parse_peek("def|abc"), Ok(("def|abc", vec![])));
        /// # }
        /// ```
        #[doc(alias = "sep_by")]
        #[doc(alias = "sep_by1")]
        #[doc(alias = "separated_list0")]
        #[doc(alias = "separated_list1")]
        #[doc(alias = "separated_m_n")]
        #[inline(always)]
        pub fn separated<Input, Output, Accumulator, Sep, Error, ParseNext, SepParser>(
            occurrences: impl Into<Range>,
            mut parser: ParseNext,
            mut separator: SepParser,
        ) -> impl Parser<Input, Accumulator, Error>
        where
            Input: Stream,
            Accumulator: Accumulate<Output>,
            ParseNext: Parser<Input, Output, Error>,
            SepParser: Parser<Input, Sep, Error>,
            Error: ParserError<Input>,
        {
            let Range { start_inclusive, end_inclusive } = occurrences.into();
            trace(
                "separated",
                move |input: &mut Input| {
                    match (start_inclusive, end_inclusive) {
                        (0, None) => separated0_(&mut parser, &mut separator, input),
                        (1, None) => separated1_(&mut parser, &mut separator, input),
                        (start, end) if Some(start) == end => {
                            separated_n_(start, &mut parser, &mut separator, input)
                        }
                        (start, end) => {
                            separated_m_n_(
                                start,
                                end.unwrap_or(usize::MAX),
                                &mut parser,
                                &mut separator,
                                input,
                            )
                        }
                    }
                },
            )
        }
        fn separated0_<I, O, C, O2, E, P, S>(
            parser: &mut P,
            separator: &mut S,
            input: &mut I,
        ) -> Result<C, E>
        where
            I: Stream,
            C: Accumulate<O>,
            P: Parser<I, O, E>,
            S: Parser<I, O2, E>,
            E: ParserError<I>,
        {
            let mut acc = C::initial(None);
            let start = input.checkpoint();
            match parser.parse_next(input) {
                Err(e) if e.is_backtrack() => {
                    input.reset(&start);
                    return Ok(acc);
                }
                Err(e) => return Err(e),
                Ok(o) => {
                    acc.accumulate(o);
                }
            }
            loop {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match separator.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        return Ok(acc);
                    }
                    Err(e) => return Err(e),
                    Ok(_) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`separated` separator parser must always consume",
                                ),
                            );
                        }
                        match parser.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                input.reset(&start);
                                return Ok(acc);
                            }
                            Err(e) => return Err(e),
                            Ok(o) => {
                                acc.accumulate(o);
                            }
                        }
                    }
                }
            }
        }
        fn separated1_<I, O, C, O2, E, P, S>(
            parser: &mut P,
            separator: &mut S,
            input: &mut I,
        ) -> Result<C, E>
        where
            I: Stream,
            C: Accumulate<O>,
            P: Parser<I, O, E>,
            S: Parser<I, O2, E>,
            E: ParserError<I>,
        {
            let mut acc = C::initial(None);
            match parser.parse_next(input) {
                Err(e) => return Err(e),
                Ok(o) => {
                    acc.accumulate(o);
                }
            }
            loop {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match separator.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&start);
                        return Ok(acc);
                    }
                    Err(e) => return Err(e),
                    Ok(_) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`separated` separator parser must always consume",
                                ),
                            );
                        }
                        match parser.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                input.reset(&start);
                                return Ok(acc);
                            }
                            Err(e) => return Err(e),
                            Ok(o) => {
                                acc.accumulate(o);
                            }
                        }
                    }
                }
            }
        }
        fn separated_n_<I, O, C, O2, E, P, S>(
            count: usize,
            parser: &mut P,
            separator: &mut S,
            input: &mut I,
        ) -> Result<C, E>
        where
            I: Stream,
            C: Accumulate<O>,
            P: Parser<I, O, E>,
            S: Parser<I, O2, E>,
            E: ParserError<I>,
        {
            let mut acc = C::initial(Some(count));
            if count == 0 {
                return Ok(acc);
            }
            let start = input.checkpoint();
            match parser.parse_next(input) {
                Err(e) => {
                    return Err(e.append(input, &start));
                }
                Ok(o) => {
                    acc.accumulate(o);
                }
            }
            for _ in 1..count {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match separator.parse_next(input) {
                    Err(e) => {
                        return Err(e.append(input, &start));
                    }
                    Ok(_) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`separated` separator parser must always consume",
                                ),
                            );
                        }
                        match parser.parse_next(input) {
                            Err(e) => {
                                return Err(e.append(input, &start));
                            }
                            Ok(o) => {
                                acc.accumulate(o);
                            }
                        }
                    }
                }
            }
            Ok(acc)
        }
        fn separated_m_n_<I, O, C, O2, E, P, S>(
            min: usize,
            max: usize,
            parser: &mut P,
            separator: &mut S,
            input: &mut I,
        ) -> Result<C, E>
        where
            I: Stream,
            C: Accumulate<O>,
            P: Parser<I, O, E>,
            S: Parser<I, O2, E>,
            E: ParserError<I>,
        {
            if min > max {
                return Err(
                    ParserError::assert(
                        input,
                        "range should be ascending, rather than descending",
                    ),
                );
            }
            let mut acc = C::initial(Some(min));
            let start = input.checkpoint();
            match parser.parse_next(input) {
                Err(e) if e.is_backtrack() => {
                    if min == 0 {
                        input.reset(&start);
                        return Ok(acc);
                    } else {
                        return Err(e.append(input, &start));
                    }
                }
                Err(e) => return Err(e),
                Ok(o) => {
                    acc.accumulate(o);
                }
            }
            for index in 1..max {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match separator.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        if index < min {
                            return Err(e.append(input, &start));
                        } else {
                            input.reset(&start);
                            return Ok(acc);
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(_) => {
                        if input.eof_offset() == len {
                            return Err(
                                ParserError::assert(
                                    input,
                                    "`separated` separator parser must always consume",
                                ),
                            );
                        }
                        match parser.parse_next(input) {
                            Err(e) if e.is_backtrack() => {
                                if index < min {
                                    return Err(e.append(input, &start));
                                } else {
                                    input.reset(&start);
                                    return Ok(acc);
                                }
                            }
                            Err(e) => {
                                return Err(e);
                            }
                            Ok(o) => {
                                acc.accumulate(o);
                            }
                        }
                    }
                }
            }
            Ok(acc)
        }
        /// Alternates between two parsers, merging the results (left associative)
        ///
        /// This stops when either parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
        /// [`cut_err`][crate::combinator::cut_err].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated_foldl1;
        /// use winnow::ascii::dec_int;
        ///
        /// fn parser(s: &mut &str) -> ModalResult<i32> {
        ///   separated_foldl1(dec_int, "-", |l, _, r| l - r).parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("9-3-5"), Ok(("", 1)));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("def|abc").is_err());
        /// # }
        /// ```
        pub fn separated_foldl1<Input, Output, Sep, Error, ParseNext, SepParser, Op>(
            mut parser: ParseNext,
            mut sep: SepParser,
            mut op: Op,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            ParseNext: Parser<Input, Output, Error>,
            SepParser: Parser<Input, Sep, Error>,
            Error: ParserError<Input>,
            Op: FnMut(Output, Sep, Output) -> Output,
        {
            trace(
                "separated_foldl1",
                move |i: &mut Input| {
                    let mut ol = parser.parse_next(i)?;
                    loop {
                        let start = i.checkpoint();
                        let len = i.eof_offset();
                        match sep.parse_next(i) {
                            Err(e) if e.is_backtrack() => {
                                i.reset(&start);
                                return Ok(ol);
                            }
                            Err(e) => return Err(e),
                            Ok(s) => {
                                if i.eof_offset() == len {
                                    return Err(
                                        ParserError::assert(
                                            i,
                                            "`repeat` parsers must always consume",
                                        ),
                                    );
                                }
                                match parser.parse_next(i) {
                                    Err(e) if e.is_backtrack() => {
                                        i.reset(&start);
                                        return Ok(ol);
                                    }
                                    Err(e) => return Err(e),
                                    Ok(or) => {
                                        ol = op(ol, s, or);
                                    }
                                }
                            }
                        }
                    }
                },
            )
        }
        /// Alternates between two parsers, merging the results (right associative)
        ///
        /// This stops when either parser returns [`ErrMode::Backtrack`][crate::error::ErrMode::Backtrack]. To instead chain an error up, see
        /// [`cut_err`][crate::combinator::cut_err].
        ///
        /// # Example
        ///
        /// ```rust
        /// # #[cfg(feature = "ascii")] {
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated_foldr1;
        /// use winnow::ascii::dec_uint;
        ///
        /// fn parser(s: &mut &str) -> ModalResult<u32> {
        ///   separated_foldr1(dec_uint, "^", |l: u32, _, r: u32| l.pow(r)).parse_next(s)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("2^3^2"), Ok(("", 512)));
        /// assert_eq!(parser.parse_peek("2"), Ok(("", 2)));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("def|abc").is_err());
        /// # }
        /// ```
        pub fn separated_foldr1<Input, Output, Sep, Error, ParseNext, SepParser, Op>(
            mut parser: ParseNext,
            mut sep: SepParser,
            mut op: Op,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            ParseNext: Parser<Input, Output, Error>,
            SepParser: Parser<Input, Sep, Error>,
            Error: ParserError<Input>,
            Op: FnMut(Output, Sep, Output) -> Output,
        {
            trace(
                "separated_foldr1",
                move |i: &mut Input| {
                    let ol = parser.parse_next(i)?;
                    let all: alloc::vec::Vec<(Sep, Output)> = repeat(
                            0..,
                            (sep.by_ref(), parser.by_ref()),
                        )
                        .parse_next(i)?;
                    if let Some((s, or)) = all
                        .into_iter()
                        .rev()
                        .reduce(|(sr, or), (sl, ol)| (sl, op(ol, sr, or)))
                    {
                        let merged = op(ol, s, or);
                        Ok(merged)
                    } else {
                        Ok(ol)
                    }
                },
            )
        }
        /// Repeats the embedded parser, filling the given slice with results.
        ///
        /// This parser fails if the input runs out before the given slice is full.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// use winnow::combinator::fill;
        ///
        /// fn parser<'i>(s: &mut &'i str) -> ModalResult<[&'i str; 2]> {
        ///   let mut buf = ["", ""];
        ///   fill("abc", &mut buf).parse_next(s)?;
        ///   Ok(buf)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcabc"), Ok(("", ["abc", "abc"])));
        /// assert!(parser.parse_peek("abc123").is_err());
        /// assert!(parser.parse_peek("123123").is_err());
        /// assert!(parser.parse_peek("").is_err());
        /// assert_eq!(parser.parse_peek("abcabcabc"), Ok(("abc", ["abc", "abc"])));
        /// ```
        pub fn fill<'i, Input, Output, Error, ParseNext>(
            mut parser: ParseNext,
            buf: &'i mut [Output],
        ) -> impl Parser<Input, (), Error> + 'i
        where
            Input: Stream + 'i,
            ParseNext: Parser<Input, Output, Error> + 'i,
            Error: ParserError<Input> + 'i,
        {
            trace(
                "fill",
                move |i: &mut Input| {
                    for elem in buf.iter_mut() {
                        let start = i.checkpoint();
                        match parser.parse_next(i) {
                            Ok(o) => {
                                *elem = o;
                            }
                            Err(e) => {
                                return Err(e.append(i, &start));
                            }
                        }
                    }
                    Ok(())
                },
            )
        }
    }
    mod sequence {
        use crate::combinator::trace;
        use crate::error::ParserError;
        use crate::stream::Stream;
        use crate::Parser;
        #[doc(inline)]
        pub use crate::seq;
        #[doc(inline)]
        pub use crate::unordered_seq;
        /// Sequence two parsers, only returning the output from the second.
        ///
        /// See also [`seq`] to generalize this across any number of fields.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// # use winnow::error::Needed::Size;
        /// use winnow::combinator::preceded;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     preceded("abc", "efg").parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcefg"), Ok(("", "efg")));
        /// assert_eq!(parser.parse_peek("abcefghij"), Ok(("hij", "efg")));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("123").is_err());
        /// ```
        #[doc(alias = "ignore_then")]
        pub fn preceded<Input, Ignored, Output, Error, IgnoredParser, ParseNext>(
            mut ignored: IgnoredParser,
            mut parser: ParseNext,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            IgnoredParser: Parser<Input, Ignored, Error>,
            ParseNext: Parser<Input, Output, Error>,
        {
            trace(
                "preceded",
                move |input: &mut Input| {
                    let _ = ignored.parse_next(input)?;
                    parser.parse_next(input)
                },
            )
        }
        /// Sequence two parsers, only returning the output of the first.
        ///
        /// See also [`seq`] to generalize this across any number of fields.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::prelude::*;
        /// # use winnow::error::Needed::Size;
        /// use winnow::combinator::terminated;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     terminated("abc", "efg").parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abcefg"), Ok(("", "abc")));
        /// assert_eq!(parser.parse_peek("abcefghij"), Ok(("hij", "abc")));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("123").is_err());
        /// ```
        #[doc(alias = "then_ignore")]
        pub fn terminated<Input, Output, Ignored, Error, ParseNext, IgnoredParser>(
            mut parser: ParseNext,
            mut ignored: IgnoredParser,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            ParseNext: Parser<Input, Output, Error>,
            IgnoredParser: Parser<Input, Ignored, Error>,
        {
            trace(
                "terminated",
                move |input: &mut Input| {
                    let o = parser.parse_next(input)?;
                    ignored.parse_next(input).map(|_| o)
                },
            )
        }
        /// Sequence three parsers, only returning the values of the first and third.
        ///
        /// See also [`seq`] to generalize this across any number of fields.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::error::Needed::Size;
        /// # use winnow::prelude::*;
        /// use winnow::combinator::separated_pair;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<(&'i str, &'i str)> {
        ///     separated_pair("abc", "|", "efg").parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("abc|efg"), Ok(("", ("abc", "efg"))));
        /// assert_eq!(parser.parse_peek("abc|efghij"), Ok(("hij", ("abc", "efg"))));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("123").is_err());
        /// ```
        pub fn separated_pair<Input, O1, Sep, O2, Error, P1, SepParser, P2>(
            mut first: P1,
            mut sep: SepParser,
            mut second: P2,
        ) -> impl Parser<Input, (O1, O2), Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            P1: Parser<Input, O1, Error>,
            SepParser: Parser<Input, Sep, Error>,
            P2: Parser<Input, O2, Error>,
        {
            trace(
                "separated_pair",
                move |input: &mut Input| {
                    let o1 = first.parse_next(input)?;
                    let _ = sep.parse_next(input)?;
                    second.parse_next(input).map(|o2| (o1, o2))
                },
            )
        }
        /// Sequence three parsers, only returning the output of the second.
        ///
        /// See also [`seq`] to generalize this across any number of fields.
        ///
        /// # Example
        ///
        /// ```rust
        /// # use winnow::{error::ErrMode, error::Needed};
        /// # use winnow::error::Needed::Size;
        /// # use winnow::prelude::*;
        /// use winnow::combinator::delimited;
        ///
        /// fn parser<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
        ///     delimited("(", "abc", ")").parse_next(input)
        /// }
        ///
        /// assert_eq!(parser.parse_peek("(abc)"), Ok(("", "abc")));
        /// assert_eq!(parser.parse_peek("(abc)def"), Ok(("def", "abc")));
        /// assert!(parser.parse_peek("").is_err());
        /// assert!(parser.parse_peek("123").is_err());
        /// ```
        #[doc(alias = "between")]
        #[doc(alias = "padded")]
        pub fn delimited<
            Input,
            Ignored1,
            Output,
            Ignored2,
            Error,
            IgnoredParser1,
            ParseNext,
            IgnoredParser2,
        >(
            mut ignored1: IgnoredParser1,
            mut parser: ParseNext,
            mut ignored2: IgnoredParser2,
        ) -> impl Parser<Input, Output, Error>
        where
            Input: Stream,
            Error: ParserError<Input>,
            IgnoredParser1: Parser<Input, Ignored1, Error>,
            ParseNext: Parser<Input, Output, Error>,
            IgnoredParser2: Parser<Input, Ignored2, Error>,
        {
            trace(
                "delimited",
                move |input: &mut Input| {
                    let _ = ignored1.parse_next(input)?;
                    let o2 = parser.parse_next(input)?;
                    ignored2.parse_next(input).map(|_| o2)
                },
            )
        }
    }
    pub mod impls {
        //! Opaque implementations of [`Parser`]
        use crate::combinator::trace;
        use crate::combinator::trace_result;
        use crate::combinator::DisplayDebug;
        use crate::error::ParseError;
        use crate::error::{AddContext, FromExternalError, ParserError};
        use crate::stream::StreamIsPartial;
        use crate::stream::{Location, Stream};
        use crate::{Parser, Result};
        use core::borrow::Borrow;
        use core::ops::Range;
        /// Iterator implementation for [`Parser::parse_iter`]
        pub struct ParseIter<'p, P, I, O, E>
        where
            I: Stream,
        {
            pub(crate) parser: &'p mut P,
            pub(crate) input: Option<I>,
            pub(crate) start: Option<I::Checkpoint>,
            pub(crate) marker: core::marker::PhantomData<(O, E)>,
        }
        impl<'p, I, O, E, P> Iterator for ParseIter<'p, P, I, O, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            I: StreamIsPartial,
            E: ParserError<I>,
            <E as ParserError<I>>::Inner: ParserError<I>,
        {
            type Item = Result<O, ParseError<I, <E as ParserError<I>>::Inner>>;
            fn next(&mut self) -> Option<Self::Item> {
                let input = self.input.as_mut()?;
                let len = input.eof_offset();
                if len == 0 {
                    self.input = None;
                    return None;
                }
                let mut output = self.parser.parse_next(input);
                if output.is_ok() && input.eof_offset() == len {
                    let err = <E as ParserError<
                        I,
                    >>::assert(
                        input,
                        "`Parser::parse_iter` parsers must always consume",
                    );
                    output = Err(err);
                }
                match output {
                    Ok(output) => Some(Ok(output)),
                    Err(err) => {
                        let err = err
                            .into_inner()
                            .unwrap_or_else(|_err| {
                                {
                                    ::core::panicking::panic_fmt(
                                        format_args!(
                                            "complete parsers should not report `ErrMode::Incomplete(_)`",
                                        ),
                                    );
                                }
                            });
                        let input = self.input.take()?;
                        let start = self.start.take()?;
                        Some(Err(ParseError::new(input, start, err)))
                    }
                }
            }
        }
        /// [`Parser`] implementation for [`Parser::by_ref`]
        pub struct ByRef<'p, P, I, O, E> {
            pub(crate) p: &'p mut P,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<I, O, E, P> Parser<I, O, E> for ByRef<'_, P, I, O, E>
        where
            P: Parser<I, O, E>,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<O, E> {
                self.p.parse_next(i)
            }
        }
        /// [`Parser`] implementation for [`Parser::map`]
        pub struct Map<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> O2,
        {
            pub(crate) parser: F,
            pub(crate) map: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, O2)>,
        }
        impl<F, G, I, O, O2, E> Parser<I, O2, E> for Map<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> O2,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O2, E> {
                match self.parser.parse_next(i) {
                    Err(e) => Err(e),
                    Ok(o) => Ok((self.map)(o)),
                }
            }
        }
        /// [`Parser`] implementation for [`Parser::try_map`]
        pub struct TryMap<F, G, I, O, O2, E, E2>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> Result<O2, E2>,
            I: Stream,
            E: FromExternalError<I, E2>,
            E: ParserError<I>,
        {
            pub(crate) parser: F,
            pub(crate) map: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, O2, E, E2)>,
        }
        impl<F, G, I, O, O2, E, E2> Parser<I, O2, E> for TryMap<F, G, I, O, O2, E, E2>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> Result<O2, E2>,
            I: Stream,
            E: FromExternalError<I, E2>,
            E: ParserError<I>,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O2, E> {
                let start = input.checkpoint();
                let o = self.parser.parse_next(input)?;
                let res = (self.map)(o)
                    .map_err(|err| {
                        input.reset(&start);
                        E::from_external_error(input, err)
                    });
                trace_result("verify", &res);
                res
            }
        }
        /// [`Parser`] implementation for [`Parser::verify_map`]
        pub struct VerifyMap<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> Option<O2>,
            I: Stream,
            E: ParserError<I>,
        {
            pub(crate) parser: F,
            pub(crate) map: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, O2)>,
        }
        impl<F, G, I, O, O2, E> Parser<I, O2, E> for VerifyMap<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> Option<O2>,
            I: Stream,
            E: ParserError<I>,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O2, E> {
                let start = input.checkpoint();
                let o = self.parser.parse_next(input)?;
                let res = (self.map)(o)
                    .ok_or_else(|| {
                        input.reset(&start);
                        ParserError::from_input(input)
                    });
                trace_result("verify", &res);
                res
            }
        }
        /// [`Parser`] implementation for [`Parser::and_then`]
        pub struct AndThen<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: Parser<O, O2, E>,
            O: StreamIsPartial,
            I: Stream,
        {
            pub(crate) outer: F,
            pub(crate) inner: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, O2, E)>,
        }
        impl<F, G, I, O, O2, E> Parser<I, O2, E> for AndThen<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: Parser<O, O2, E>,
            O: StreamIsPartial,
            I: Stream,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<O2, E> {
                let start = i.checkpoint();
                let mut o = self.outer.parse_next(i)?;
                let _ = o.complete();
                let o2 = self
                    .inner
                    .parse_next(&mut o)
                    .map_err(|err| {
                        i.reset(&start);
                        err
                    })?;
                Ok(o2)
            }
        }
        /// [`Parser`] implementation for [`Parser::parse_to`]
        pub struct ParseTo<P, I, O, O2, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            O: crate::stream::ParseSlice<O2>,
            E: ParserError<I>,
        {
            pub(crate) p: P,
            pub(crate) marker: core::marker::PhantomData<(I, O, O2, E)>,
        }
        impl<P, I, O, O2, E> Parser<I, O2, E> for ParseTo<P, I, O, O2, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            O: crate::stream::ParseSlice<O2>,
            E: ParserError<I>,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O2, E> {
                let start = i.checkpoint();
                let o = self.p.parse_next(i)?;
                let res = o
                    .parse_slice()
                    .ok_or_else(|| {
                        i.reset(&start);
                        ParserError::from_input(i)
                    });
                trace_result("verify", &res);
                res
            }
        }
        /// [`Parser`] implementation for [`Parser::flat_map`]
        pub struct FlatMap<F, G, H, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> H,
            H: Parser<I, O2, E>,
        {
            pub(crate) f: F,
            pub(crate) g: G,
            pub(crate) marker: core::marker::PhantomData<(H, I, O, O2, E)>,
        }
        impl<F, G, H, I, O, O2, E> Parser<I, O2, E> for FlatMap<F, G, H, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(O) -> H,
            H: Parser<I, O2, E>,
        {
            #[inline(always)]
            fn parse_next(&mut self, i: &mut I) -> Result<O2, E> {
                let o = self.f.parse_next(i)?;
                (self.g)(o).parse_next(i)
            }
        }
        /// [`Parser`] implementation for [`Parser::complete_err`]
        pub struct CompleteErr<P, I, O, E> {
            pub(crate) p: P,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<P, I, O, E> Parser<I, O, E> for CompleteErr<P, I, O, E>
        where
            P: Parser<I, O, E>,
            I: Stream,
            E: ParserError<I>,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O, E> {
                trace(
                        "complete_err",
                        |input: &mut I| {
                            match (self.p).parse_next(input) {
                                Err(err) => {
                                    match err.needed() {
                                        Some(_) => Err(ParserError::from_input(input)),
                                        None => Err(err),
                                    }
                                }
                                rest => rest,
                            }
                        },
                    )
                    .parse_next(input)
            }
        }
        /// [`Parser`] implementation for [`Parser::verify`]
        pub struct Verify<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(&O2) -> bool,
            I: Stream,
            O: Borrow<O2>,
            O2: ?Sized,
            E: ParserError<I>,
        {
            pub(crate) parser: F,
            pub(crate) filter: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, O2)>,
        }
        impl<F, G, I, O, O2, E> Parser<I, O, E> for Verify<F, G, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            G: FnMut(&O2) -> bool,
            I: Stream,
            O: Borrow<O2>,
            O2: ?Sized,
            E: ParserError<I>,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O, E> {
                let start = input.checkpoint();
                let o = self.parser.parse_next(input)?;
                let res = (self.filter)(o.borrow())
                    .then_some(o)
                    .ok_or_else(|| {
                        input.reset(&start);
                        ParserError::from_input(input)
                    });
                trace_result("verify", &res);
                res
            }
        }
        /// [`Parser`] implementation for [`Parser::value`]
        pub struct Value<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O2: Clone,
        {
            pub(crate) parser: F,
            pub(crate) val: O2,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<F, I, O, O2, E> Parser<I, O2, E> for Value<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O2: Clone,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O2, E> {
                (self.parser).parse_next(input).map(|_| self.val.clone())
            }
        }
        /// [`Parser`] implementation for [`Parser::default_value`]
        pub struct DefaultValue<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O2: core::default::Default,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(O2, I, O, E)>,
        }
        impl<F, I, O, O2, E> Parser<I, O2, E> for DefaultValue<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O2: core::default::Default,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<O2, E> {
                (self.parser).parse_next(input).map(|_| O2::default())
            }
        }
        /// [`Parser`] implementation for [`Parser::void`]
        pub struct Void<F, I, O, E>
        where
            F: Parser<I, O, E>,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<F, I, O, E> Parser<I, (), E> for Void<F, I, O, E>
        where
            F: Parser<I, O, E>,
        {
            #[inline(always)]
            fn parse_next(&mut self, input: &mut I) -> Result<(), E> {
                (self.parser).parse_next(input).map(|_| ())
            }
        }
        /// [`Parser`] implementation for [`Parser::take`]
        pub struct Take<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<I, O, E, F> Parser<I, <I as Stream>::Slice, E> for Take<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<<I as Stream>::Slice, E> {
                let checkpoint = input.checkpoint();
                match (self.parser).parse_next(input) {
                    Ok(_) => {
                        let offset = input.offset_from(&checkpoint);
                        input.reset(&checkpoint);
                        let taken = input.next_slice(offset);
                        Ok(taken)
                    }
                    Err(e) => Err(e),
                }
            }
        }
        /// [`Parser`] implementation for [`Parser::with_taken`]
        pub struct WithTaken<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<F, I, O, E> Parser<I, (O, <I as Stream>::Slice), E>
        for WithTaken<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream,
        {
            #[inline]
            fn parse_next(
                &mut self,
                input: &mut I,
            ) -> Result<(O, <I as Stream>::Slice), E> {
                let checkpoint = input.checkpoint();
                match (self.parser).parse_next(input) {
                    Ok(result) => {
                        let offset = input.offset_from(&checkpoint);
                        input.reset(&checkpoint);
                        let taken = input.next_slice(offset);
                        Ok((result, taken))
                    }
                    Err(e) => Err(e),
                }
            }
        }
        /// [`Parser`] implementation for [`Parser::span`]
        pub struct Span<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream + Location,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<I, O, E, F> Parser<I, Range<usize>, E> for Span<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream + Location,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<Range<usize>, E> {
                let start = input.current_token_start();
                self.parser
                    .parse_next(input)
                    .map(move |_| {
                        let end = input.previous_token_end();
                        start..end
                    })
            }
        }
        /// [`Parser`] implementation for [`Parser::with_span`]
        pub struct WithSpan<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream + Location,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<F, I, O, E> Parser<I, (O, Range<usize>), E> for WithSpan<F, I, O, E>
        where
            F: Parser<I, O, E>,
            I: Stream + Location,
        {
            #[inline]
            fn parse_next(&mut self, input: &mut I) -> Result<(O, Range<usize>), E> {
                let start = input.current_token_start();
                self.parser
                    .parse_next(input)
                    .map(move |output| {
                        let end = input.previous_token_end();
                        (output, (start..end))
                    })
            }
        }
        /// [`Parser`] implementation for [`Parser::output_into`]
        pub struct OutputInto<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O: Into<O2>,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, O2, E)>,
        }
        impl<F, I, O, O2, E> Parser<I, O2, E> for OutputInto<F, I, O, O2, E>
        where
            F: Parser<I, O, E>,
            O: Into<O2>,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O2, E> {
                self.parser.parse_next(i).map(|o| o.into())
            }
        }
        /// [`Parser`] implementation for [`Parser::err_into`]
        pub struct ErrInto<F, I, O, E, E2>
        where
            F: Parser<I, O, E>,
            E: Into<E2>,
        {
            pub(crate) parser: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, E2)>,
        }
        impl<F, I, O, E, E2> Parser<I, O, E2> for ErrInto<F, I, O, E, E2>
        where
            F: Parser<I, O, E>,
            E: Into<E2>,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O, E2> {
                self.parser.parse_next(i).map_err(|err| err.into())
            }
        }
        /// [`Parser`] implementation for [`Parser::context`]
        pub struct Context<F, I, O, E, C>
        where
            F: Parser<I, O, E>,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            C: Clone + core::fmt::Debug,
        {
            pub(crate) parser: F,
            pub(crate) context: C,
            pub(crate) marker: core::marker::PhantomData<(I, O, E)>,
        }
        impl<F, I, O, E, C> Parser<I, O, E> for Context<F, I, O, E, C>
        where
            F: Parser<I, O, E>,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            C: Clone + core::fmt::Debug,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O, E> {
                let context = self.context.clone();
                trace(
                        DisplayDebug(self.context.clone()),
                        move |i: &mut I| {
                            let start = i.checkpoint();
                            (self.parser)
                                .parse_next(i)
                                .map_err(|err| err.add_context(i, &start, context.clone()))
                        },
                    )
                    .parse_next(i)
            }
        }
        /// [`Parser`] implementation for [`Parser::context`]
        pub struct ContextWith<P, I, O, E, F, C, FI>
        where
            P: Parser<I, O, E>,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            F: Fn() -> FI + Clone,
            C: core::fmt::Debug,
            FI: Iterator<Item = C>,
        {
            pub(crate) parser: P,
            pub(crate) context: F,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, C, FI)>,
        }
        impl<P, I, O, E, F, C, FI> Parser<I, O, E> for ContextWith<P, I, O, E, F, C, FI>
        where
            P: Parser<I, O, E>,
            I: Stream,
            E: AddContext<I, C>,
            E: ParserError<I>,
            F: Fn() -> FI + Clone,
            C: core::fmt::Debug,
            FI: Iterator<Item = C>,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O, E> {
                let context = self.context.clone();
                let start = i.checkpoint();
                (self.parser)
                    .parse_next(i)
                    .map_err(|mut err| {
                        for context in context() {
                            err = err.add_context(i, &start, context);
                        }
                        err
                    })
            }
        }
        /// [`Parser`] implementation for [`Parser::map_err`]
        pub struct MapErr<F, G, I, O, E, E2>
        where
            F: Parser<I, O, E>,
            G: FnMut(E) -> E2,
        {
            pub(crate) parser: F,
            pub(crate) map: G,
            pub(crate) marker: core::marker::PhantomData<(I, O, E, E2)>,
        }
        impl<F, G, I, O, E, E2> Parser<I, O, E2> for MapErr<F, G, I, O, E, E2>
        where
            F: Parser<I, O, E>,
            G: FnMut(E) -> E2,
        {
            #[inline]
            fn parse_next(&mut self, i: &mut I) -> Result<O, E2> {
                match self.parser.parse_next(i) {
                    Err(e) => Err((self.map)(e)),
                    Ok(o) => Ok(o),
                }
            }
        }
    }
    pub use self::branch::{alt, dispatch, Alt};
    pub use self::core::{
        backtrack_err, cond, cut_err, empty, eof, fail, not, opt, peek, todo,
    };
    pub use self::debug::trace;
    pub use self::expression::{expression, Expression, Infix, Postfix, Prefix};
    pub use self::multi::separated_foldr1;
    pub use self::multi::{
        fill, iterator, repeat, repeat_till, separated, separated_foldl1, ParserIterator,
        Repeat,
    };
    pub use self::sequence::{
        delimited, preceded, separated_pair, seq, terminated, unordered_seq,
    };
    pub(crate) use self::debug::{trace_result, DisplayDebug};
    #[allow(unused_imports)]
    use crate::Parser;
}
pub mod token {
    //! Parsers extracting tokens from the stream
    use crate::combinator::trace;
    use crate::combinator::DisplayDebug;
    use crate::error::Needed;
    use crate::error::ParserError;
    use crate::stream::Range;
    use crate::stream::{Compare, CompareResult, ContainsToken, FindSlice, Stream};
    use crate::stream::{StreamIsPartial, ToUsize};
    use crate::Parser;
    use crate::Result;
    use core::result::Result::Ok;
    /// Matches one token
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn any(input: &mut &str) -> ModalResult<char>
    /// # {
    /// #     winnow::token::any.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{token::any, error::ErrMode, error::ContextError};
    /// # use winnow::prelude::*;
    /// fn parser(input: &mut &str) -> ModalResult<char> {
    ///     any.parse_next(input)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("abc"), Ok(("bc",'a')));
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{token::any, error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// assert_eq!(any::<_, ErrMode<ContextError>>.parse_peek(Partial::new("abc")), Ok((Partial::new("bc"),'a')));
    /// assert_eq!(any::<_, ErrMode<ContextError>>.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    #[doc(alias = "token")]
    pub fn any<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Token, Error>
    where
        Input: StreamIsPartial + Stream,
        Error: ParserError<Input>,
    {
        trace(
                "any",
                move |input: &mut Input| {
                    if <Input as StreamIsPartial>::is_partial_supported() {
                        any_::<_, _, true>(input)
                    } else {
                        any_::<_, _, false>(input)
                    }
                },
            )
            .parse_next(input)
    }
    fn any_<I, E: ParserError<I>, const PARTIAL: bool>(
        input: &mut I,
    ) -> Result<<I as Stream>::Token, E>
    where
        I: StreamIsPartial,
        I: Stream,
    {
        input
            .next_token()
            .ok_or_else(|| {
                if PARTIAL && input.is_partial() {
                    ParserError::incomplete(input, Needed::new(1))
                } else {
                    ParserError::from_input(input)
                }
            })
    }
    /// Recognizes a literal
    ///
    /// The input data will be compared to the literal combinator's argument and will return the part of
    /// the input that matches the argument
    ///
    /// It will return `Err(ErrMode::Backtrack(_))` if the input doesn't match the literal
    ///
    /// <div class="warning">
    ///
    /// **Note:** [`Parser`] is implemented for strings and byte strings as a convenience (complete
    /// only)
    ///
    /// </div>
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// # use winnow::error::ContextError;
    /// pub fn literal(literal: &str) -> impl Parser<&str, &str, ContextError>
    /// # {
    /// #     winnow::token::literal(literal)
    /// # }
    /// ```
    ///
    /// # Example
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// #
    /// fn parser<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   "Hello".parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("Hello, World!"), Ok((", World!", "Hello")));
    /// assert!(parser.parse_peek("Something").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    ///
    /// fn parser<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   "Hello".parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek(Partial::new("Hello, World!")), Ok((Partial::new(", World!"), "Hello")));
    /// assert!(parser.parse_peek(Partial::new("Something")).is_err());
    /// assert!(parser.parse_peek(Partial::new("S")).is_err());
    /// assert_eq!(parser.parse_peek(Partial::new("H")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    ///
    /// ```rust
    /// # #[cfg(feature = "ascii")] {
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::literal;
    /// use winnow::ascii::Caseless;
    ///
    /// fn parser<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   literal(Caseless("hello")).parse_next(s)
    /// }
    ///
    /// assert_eq!(parser.parse_peek("Hello, World!"), Ok((", World!", "Hello")));
    /// assert_eq!(parser.parse_peek("hello, World!"), Ok((", World!", "hello")));
    /// assert_eq!(parser.parse_peek("HeLlO, World!"), Ok((", World!", "HeLlO")));
    /// assert!(parser.parse_peek("Something").is_err());
    /// assert!(parser.parse_peek("").is_err());
    /// # }
    /// ```
    #[inline(always)]
    #[doc(alias = "tag")]
    #[doc(alias = "bytes")]
    #[doc(alias = "just")]
    pub fn literal<Literal, Input, Error>(
        literal: Literal,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream + Compare<Literal>,
        Literal: Clone + core::fmt::Debug,
        Error: ParserError<Input>,
    {
        trace(
            DisplayDebug(literal.clone()),
            move |i: &mut Input| {
                let t = literal.clone();
                if <Input as StreamIsPartial>::is_partial_supported() {
                    literal_::<_, _, _, true>(i, t)
                } else {
                    literal_::<_, _, _, false>(i, t)
                }
            },
        )
    }
    fn literal_<T, I, Error: ParserError<I>, const PARTIAL: bool>(
        i: &mut I,
        t: T,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream + Compare<T>,
        T: core::fmt::Debug,
    {
        match i.compare(t) {
            CompareResult::Ok(len) => Ok(i.next_slice(len)),
            CompareResult::Incomplete if PARTIAL && i.is_partial() => {
                Err(ParserError::incomplete(i, Needed::Unknown))
            }
            CompareResult::Incomplete | CompareResult::Error => {
                Err(ParserError::from_input(i))
            }
        }
    }
    /// Recognize a token that matches a [set of tokens][ContainsToken]
    ///
    /// <div class="warning">
    ///
    /// **Note:** [`Parser`] is implemented as a convenience (complete
    /// only) for
    /// - `u8`
    /// - `char`
    ///
    /// </div>
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// # use winnow::stream::ContainsToken;
    /// # use winnow::error::ContextError;
    /// pub fn one_of<'i>(set: impl ContainsToken<char>) -> impl Parser<&'i str, char, ContextError>
    /// # {
    /// #     winnow::token::one_of(set)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// # use winnow::token::one_of;
    /// assert_eq!(one_of::<_, _, ContextError>(['a', 'b', 'c']).parse_peek("b"), Ok(("", 'b')));
    /// assert!(one_of::<_, _, ContextError>('a').parse_peek("bc").is_err());
    /// assert!(one_of::<_, _, ContextError>('a').parse_peek("").is_err());
    ///
    /// fn parser_fn(i: &mut &str) -> ModalResult<char> {
    ///     one_of(|c| c == 'a' || c == 'b').parse_next(i)
    /// }
    /// assert_eq!(parser_fn.parse_peek("abc"), Ok(("bc", 'a')));
    /// assert!(parser_fn.parse_peek("cd").is_err());
    /// assert!(parser_fn.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::Partial;
    /// # use winnow::token::one_of;
    /// assert_eq!(one_of::<_, _, ErrMode<ContextError>>(['a', 'b', 'c']).parse_peek(Partial::new("b")), Ok((Partial::new(""), 'b')));
    /// assert!(one_of::<_, _, ErrMode<ContextError>>('a').parse_peek(Partial::new("bc")).is_err());
    /// assert_eq!(one_of::<_, _, ErrMode<ContextError>>('a').parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    ///
    /// fn parser_fn(i: &mut Partial<&str>) -> ModalResult<char> {
    ///     one_of(|c| c == 'a' || c == 'b').parse_next(i)
    /// }
    /// assert_eq!(parser_fn.parse_peek(Partial::new("abc")), Ok((Partial::new("bc"), 'a')));
    /// assert!(parser_fn.parse_peek(Partial::new("cd")).is_err());
    /// assert_eq!(parser_fn.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    #[doc(alias = "char")]
    #[doc(alias = "token")]
    #[doc(alias = "satisfy")]
    pub fn one_of<Input, Set, Error>(
        set: Set,
    ) -> impl Parser<Input, <Input as Stream>::Token, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: Clone,
        Set: ContainsToken<<Input as Stream>::Token>,
        Error: ParserError<Input>,
    {
        trace(
            "one_of",
            any.verify(move |t: &<Input as Stream>::Token| set.contains_token(t.clone())),
        )
    }
    /// Recognize a token that does not match a [set of tokens][ContainsToken]
    ///
    /// *Complete version*: Will return an error if there's not enough input data.
    ///
    /// *[Partial version][crate::_topic::partial]*: Will return `Err(winnow::error::ErrMode::Incomplete(_))` if there's not enough input data.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// # use winnow::stream::ContainsToken;
    /// # use winnow::error::ContextError;
    /// pub fn none_of<'i>(set: impl ContainsToken<char>) -> impl Parser<&'i str, char, ContextError>
    /// # {
    /// #     winnow::token::none_of(set)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError};
    /// # use winnow::prelude::*;
    /// # use winnow::token::none_of;
    /// assert_eq!(none_of::<_, _, ContextError>(['a', 'b', 'c']).parse_peek("z"), Ok(("", 'z')));
    /// assert!(none_of::<_, _, ContextError>(['a', 'b']).parse_peek("a").is_err());
    /// assert!(none_of::<_, _, ContextError>('a').parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// # use winnow::token::none_of;
    /// assert_eq!(none_of::<_, _, ErrMode<ContextError>>(['a', 'b', 'c']).parse_peek(Partial::new("z")), Ok((Partial::new(""), 'z')));
    /// assert!(none_of::<_, _, ErrMode<ContextError>>(['a', 'b']).parse_peek(Partial::new("a")).is_err());
    /// assert_eq!(none_of::<_, _, ErrMode<ContextError>>('a').parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    pub fn none_of<Input, Set, Error>(
        set: Set,
    ) -> impl Parser<Input, <Input as Stream>::Token, Error>
    where
        Input: StreamIsPartial + Stream,
        <Input as Stream>::Token: Clone,
        Set: ContainsToken<<Input as Stream>::Token>,
        Error: ParserError<Input>,
    {
        trace(
            "none_of",
            any
                .verify(move |t: &<Input as Stream>::Token| {
                    !set.contains_token(t.clone())
                }),
        )
    }
    /// Recognize the longest (m <= len <= n) input slice that matches a [set of tokens][ContainsToken]
    ///
    /// It will return an `ErrMode::Backtrack(_)` if the set of tokens wasn't met or is out
    /// of range (m <= len <= n).
    ///
    /// *[Partial version][crate::_topic::partial]* will return a `ErrMode::Incomplete(Needed::new(1))` if a member of the set of tokens reaches the end of the input or is too short.
    ///
    /// To take a series of tokens, use [`repeat`][crate::combinator::repeat] to [`Accumulate`][crate::stream::Accumulate] into a `()` and then [`Parser::take`].
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] with `0..` or `1..` [ranges][Range]:
    /// ```rust
    /// # use std::ops::RangeFrom;
    /// # use winnow::prelude::*;
    /// # use winnow::stream::ContainsToken;
    /// # use winnow::error::ContextError;
    /// pub fn take_while<'i>(occurrences: RangeFrom<usize>, set: impl ContainsToken<char>) -> impl Parser<&'i str, &'i str, ContextError>
    /// # {
    /// #     winnow::token::take_while(occurrences, set)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// Zero or more tokens:
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn alpha<'i>(s: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    ///   take_while(0.., AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(alpha.parse_peek(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
    /// assert_eq!(alpha.parse_peek(b"12345"), Ok((&b"12345"[..], &b""[..])));
    /// assert_eq!(alpha.parse_peek(b"latin"), Ok((&b""[..], &b"latin"[..])));
    /// assert_eq!(alpha.parse_peek(b""), Ok((&b""[..], &b""[..])));
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn alpha<'i>(s: &mut Partial<&'i [u8]>) -> ModalResult<&'i [u8]> {
    ///   take_while(0.., AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(alpha.parse_peek(Partial::new(b"latin123")), Ok((Partial::new(&b"123"[..]), &b"latin"[..])));
    /// assert_eq!(alpha.parse_peek(Partial::new(b"12345")), Ok((Partial::new(&b"12345"[..]), &b""[..])));
    /// assert_eq!(alpha.parse_peek(Partial::new(b"latin")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(alpha.parse_peek(Partial::new(b"")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    ///
    /// One or more tokens:
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn alpha<'i>(s: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    ///   take_while(1.., AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(alpha.parse_peek(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
    /// assert_eq!(alpha.parse_peek(b"latin"), Ok((&b""[..], &b"latin"[..])));
    /// assert!(alpha.parse_peek(b"12345").is_err());
    ///
    /// fn hex<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   take_while(1.., ('0'..='9', 'A'..='F')).parse_next(s)
    /// }
    ///
    /// assert_eq!(hex.parse_peek("123 and voila"), Ok((" and voila", "123")));
    /// assert_eq!(hex.parse_peek("DEADBEEF and others"), Ok((" and others", "DEADBEEF")));
    /// assert_eq!(hex.parse_peek("BADBABEsomething"), Ok(("something", "BADBABE")));
    /// assert_eq!(hex.parse_peek("D15EA5E"), Ok(("", "D15EA5E")));
    /// assert!(hex.parse_peek("").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn alpha<'i>(s: &mut Partial<&'i [u8]>) -> ModalResult<&'i [u8]> {
    ///   take_while(1.., AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(alpha.parse_peek(Partial::new(b"latin123")), Ok((Partial::new(&b"123"[..]), &b"latin"[..])));
    /// assert_eq!(alpha.parse_peek(Partial::new(b"latin")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert!(alpha.parse_peek(Partial::new(b"12345")).is_err());
    ///
    /// fn hex<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take_while(1.., ('0'..='9', 'A'..='F')).parse_next(s)
    /// }
    ///
    /// assert_eq!(hex.parse_peek(Partial::new("123 and voila")), Ok((Partial::new(" and voila"), "123")));
    /// assert_eq!(hex.parse_peek(Partial::new("DEADBEEF and others")), Ok((Partial::new(" and others"), "DEADBEEF")));
    /// assert_eq!(hex.parse_peek(Partial::new("BADBABEsomething")), Ok((Partial::new("something"), "BADBABE")));
    /// assert_eq!(hex.parse_peek(Partial::new("D15EA5E")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(hex.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    ///
    /// Arbitrary amount of tokens:
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn short_alpha<'i>(s: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    ///   take_while(3..=6, AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(short_alpha.parse_peek(b"latin123"), Ok((&b"123"[..], &b"latin"[..])));
    /// assert_eq!(short_alpha.parse_peek(b"lengthy"), Ok((&b"y"[..], &b"length"[..])));
    /// assert_eq!(short_alpha.parse_peek(b"latin"), Ok((&b""[..], &b"latin"[..])));
    /// assert!(short_alpha.parse_peek(b"ed").is_err());
    /// assert!(short_alpha.parse_peek(b"12345").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_while;
    /// use winnow::stream::AsChar;
    ///
    /// fn short_alpha<'i>(s: &mut Partial<&'i [u8]>) -> ModalResult<&'i [u8]> {
    ///   take_while(3..=6, AsChar::is_alpha).parse_next(s)
    /// }
    ///
    /// assert_eq!(short_alpha.parse_peek(Partial::new(b"latin123")), Ok((Partial::new(&b"123"[..]), &b"latin"[..])));
    /// assert_eq!(short_alpha.parse_peek(Partial::new(b"lengthy")), Ok((Partial::new(&b"y"[..]), &b"length"[..])));
    /// assert_eq!(short_alpha.parse_peek(Partial::new(b"latin")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(short_alpha.parse_peek(Partial::new(b"ed")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert!(short_alpha.parse_peek(Partial::new(b"12345")).is_err());
    /// ```
    #[inline(always)]
    #[doc(alias = "is_a")]
    #[doc(alias = "take_while0")]
    #[doc(alias = "take_while1")]
    pub fn take_while<Set, Input, Error>(
        occurrences: impl Into<Range>,
        set: Set,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        Set: ContainsToken<<Input as Stream>::Token>,
        Error: ParserError<Input>,
    {
        let Range { start_inclusive, end_inclusive } = occurrences.into();
        trace(
            "take_while",
            move |i: &mut Input| {
                match (start_inclusive, end_inclusive) {
                    (0, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till0::<_, _, _, true>(i, |c| !set.contains_token(c))
                        } else {
                            take_till0::<_, _, _, false>(i, |c| !set.contains_token(c))
                        }
                    }
                    (1, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till1::<_, _, _, true>(i, |c| !set.contains_token(c))
                        } else {
                            take_till1::<_, _, _, false>(i, |c| !set.contains_token(c))
                        }
                    }
                    (start, end) => {
                        let end = end.unwrap_or(usize::MAX);
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till_m_n::<
                                _,
                                _,
                                _,
                                true,
                            >(i, start, end, |c| !set.contains_token(c))
                        } else {
                            take_till_m_n::<
                                _,
                                _,
                                _,
                                false,
                            >(i, start, end, |c| !set.contains_token(c))
                        }
                    }
                }
            },
        )
    }
    fn take_till0<
        P,
        I: StreamIsPartial + Stream,
        E: ParserError<I>,
        const PARTIAL: bool,
    >(input: &mut I, predicate: P) -> Result<<I as Stream>::Slice, E>
    where
        P: Fn(I::Token) -> bool,
    {
        let offset = match input.offset_for(predicate) {
            Some(offset) => offset,
            None if PARTIAL && input.is_partial() => {
                return Err(ParserError::incomplete(input, Needed::new(1)));
            }
            None => input.eof_offset(),
        };
        Ok(input.next_slice(offset))
    }
    fn take_till1<
        P,
        I: StreamIsPartial + Stream,
        E: ParserError<I>,
        const PARTIAL: bool,
    >(input: &mut I, predicate: P) -> Result<<I as Stream>::Slice, E>
    where
        P: Fn(I::Token) -> bool,
    {
        let offset = match input.offset_for(predicate) {
            Some(offset) => offset,
            None if PARTIAL && input.is_partial() => {
                return Err(ParserError::incomplete(input, Needed::new(1)));
            }
            None => input.eof_offset(),
        };
        if offset == 0 {
            Err(ParserError::from_input(input))
        } else {
            Ok(input.next_slice(offset))
        }
    }
    fn take_till_m_n<P, I, Error: ParserError<I>, const PARTIAL: bool>(
        input: &mut I,
        m: usize,
        n: usize,
        predicate: P,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream,
        P: Fn(I::Token) -> bool,
    {
        if n < m {
            return Err(
                ParserError::assert(
                    input,
                    "`occurrences` should be ascending, rather than descending",
                ),
            );
        }
        let mut final_count = 0;
        for (processed, (offset, token)) in input.iter_offsets().enumerate() {
            if predicate(token) {
                if processed < m {
                    return Err(ParserError::from_input(input));
                } else {
                    return Ok(input.next_slice(offset));
                }
            } else {
                if processed == n {
                    return Ok(input.next_slice(offset));
                }
                final_count = processed + 1;
            }
        }
        if PARTIAL && input.is_partial() {
            if final_count == n {
                Ok(input.finish())
            } else {
                let needed = if m > input.eof_offset() {
                    m - input.eof_offset()
                } else {
                    1
                };
                Err(ParserError::incomplete(input, Needed::new(needed)))
            }
        } else {
            if m <= final_count {
                Ok(input.finish())
            } else {
                Err(ParserError::from_input(input))
            }
        }
    }
    /// Recognize the longest input slice (if any) till a member of a [set of tokens][ContainsToken] is found.
    ///
    /// It doesn't consume the terminating token from the set.
    ///
    /// *[Partial version][crate::_topic::partial]* will return a `ErrMode::Incomplete(Needed::new(1))` if the match reaches the
    /// end of input or if there was not match.
    ///
    /// See also
    /// - [`take_until`] for recognizing up-to a [`literal`] (w/ optional simd optimizations)
    /// - [`repeat_till`][crate::combinator::repeat_till] with [`Parser::take`] for taking tokens up to a [`Parser`]
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] with `0..` or `1..` [ranges][Range]:
    /// ```rust
    /// # use std::ops::RangeFrom;
    /// # use winnow::prelude::*;
    /// # use winnow::stream::ContainsToken;
    /// # use winnow::error::ContextError;
    /// pub fn take_till<'i>(occurrences: RangeFrom<usize>, set: impl ContainsToken<char>) -> impl Parser<&'i str, &'i str, ContextError>
    /// # {
    /// #     winnow::token::take_till(occurrences, set)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_till;
    ///
    /// fn till_colon<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   take_till(0.., |c| c == ':').parse_next(s)
    /// }
    ///
    /// assert_eq!(till_colon.parse_peek("latin:123"), Ok((":123", "latin")));
    /// assert_eq!(till_colon.parse_peek(":empty matched"), Ok((":empty matched", ""))); //allowed
    /// assert_eq!(till_colon.parse_peek("12345"), Ok(("", "12345")));
    /// assert_eq!(till_colon.parse_peek(""), Ok(("", "")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_till;
    ///
    /// fn till_colon<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take_till(0.., |c| c == ':').parse_next(s)
    /// }
    ///
    /// assert_eq!(till_colon.parse_peek(Partial::new("latin:123")), Ok((Partial::new(":123"), "latin")));
    /// assert_eq!(till_colon.parse_peek(Partial::new(":empty matched")), Ok((Partial::new(":empty matched"), ""))); //allowed
    /// assert_eq!(till_colon.parse_peek(Partial::new("12345")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// assert_eq!(till_colon.parse_peek(Partial::new("")), Err(ErrMode::Incomplete(Needed::new(1))));
    /// ```
    #[inline(always)]
    #[doc(alias = "is_not")]
    pub fn take_till<Set, Input, Error>(
        occurrences: impl Into<Range>,
        set: Set,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        Set: ContainsToken<<Input as Stream>::Token>,
        Error: ParserError<Input>,
    {
        let Range { start_inclusive, end_inclusive } = occurrences.into();
        trace(
            "take_till",
            move |i: &mut Input| {
                match (start_inclusive, end_inclusive) {
                    (0, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till0::<_, _, _, true>(i, |c| set.contains_token(c))
                        } else {
                            take_till0::<_, _, _, false>(i, |c| set.contains_token(c))
                        }
                    }
                    (1, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till1::<_, _, _, true>(i, |c| set.contains_token(c))
                        } else {
                            take_till1::<_, _, _, false>(i, |c| set.contains_token(c))
                        }
                    }
                    (start, end) => {
                        let end = end.unwrap_or(usize::MAX);
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_till_m_n::<
                                _,
                                _,
                                _,
                                true,
                            >(i, start, end, |c| set.contains_token(c))
                        } else {
                            take_till_m_n::<
                                _,
                                _,
                                _,
                                false,
                            >(i, start, end, |c| set.contains_token(c))
                        }
                    }
                }
            },
        )
    }
    /// Recognize an input slice containing the first N input elements (I[..N]).
    ///
    /// *Complete version*: It will return `Err(ErrMode::Backtrack(_))` if the input is shorter than the argument.
    ///
    /// *[Partial version][crate::_topic::partial]*: if the input has less than N elements, `take` will
    /// return a `ErrMode::Incomplete(Needed::new(M))` where M is the number of
    /// additional bytes the parser would need to succeed.
    /// It is well defined for `&[u8]` as the number of elements is the byte size,
    /// but for types like `&str`, we cannot know how many bytes correspond for
    /// the next few chars, so the result will be `ErrMode::Incomplete(Needed::Unknown)`
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] with `0..` or `1..` ranges:
    /// ```rust
    /// # use std::ops::RangeFrom;
    /// # use winnow::prelude::*;
    /// # use winnow::stream::ContainsToken;
    /// # use winnow::error::ContextError;
    /// pub fn take<'i>(token_count: usize) -> impl Parser<&'i str, &'i str, ContextError>
    /// # {
    /// #     winnow::token::take(token_count)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take;
    ///
    /// fn take6<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   take(6usize).parse_next(s)
    /// }
    ///
    /// assert_eq!(take6.parse_peek("1234567"), Ok(("7", "123456")));
    /// assert_eq!(take6.parse_peek("things"), Ok(("", "things")));
    /// assert!(take6.parse_peek("short").is_err());
    /// assert!(take6.parse_peek("").is_err());
    /// ```
    ///
    /// The units that are taken will depend on the input type. For example, for a
    /// `&str` it will take a number of `char`'s, whereas for a `&[u8]` it will
    /// take that many `u8`'s:
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// use winnow::error::ContextError;
    /// use winnow::token::take;
    ///
    /// assert_eq!(take::<_, _, ContextError>(1usize).parse_peek("💙"), Ok(("", "💙")));
    /// assert_eq!(take::<_, _, ContextError>(1usize).parse_peek("💙".as_bytes()), Ok((b"\x9F\x92\x99".as_ref(), b"\xF0".as_ref())));
    /// ```
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::error::{ErrMode, ContextError, Needed};
    /// # use winnow::Partial;
    /// use winnow::token::take;
    ///
    /// fn take6<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take(6usize).parse_next(s)
    /// }
    ///
    /// assert_eq!(take6.parse_peek(Partial::new("1234567")), Ok((Partial::new("7"), "123456")));
    /// assert_eq!(take6.parse_peek(Partial::new("things")), Ok((Partial::new(""), "things")));
    /// // `Unknown` as we don't know the number of bytes that `count` corresponds to
    /// assert_eq!(take6.parse_peek(Partial::new("short")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// ```
    #[inline(always)]
    pub fn take<UsizeLike, Input, Error>(
        token_count: UsizeLike,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream,
        UsizeLike: ToUsize,
        Error: ParserError<Input>,
    {
        let c = token_count.to_usize();
        trace(
            "take",
            move |i: &mut Input| {
                if <Input as StreamIsPartial>::is_partial_supported() {
                    take_::<_, _, true>(i, c)
                } else {
                    take_::<_, _, false>(i, c)
                }
            },
        )
    }
    fn take_<I, Error: ParserError<I>, const PARTIAL: bool>(
        i: &mut I,
        c: usize,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream,
    {
        match i.offset_at(c) {
            Ok(offset) => Ok(i.next_slice(offset)),
            Err(e) if PARTIAL && i.is_partial() => Err(ParserError::incomplete(i, e)),
            Err(_needed) => Err(ParserError::from_input(i)),
        }
    }
    /// Recognize the input slice up to the first occurrence of a [literal].
    ///
    /// Feature `simd` will enable the use of [`memchr`](https://docs.rs/memchr/latest/memchr/).
    ///
    /// It doesn't consume the literal.
    ///
    /// *Complete version*: It will return `Err(ErrMode::Backtrack(_))`
    /// if the literal wasn't met.
    ///
    /// *[Partial version][crate::_topic::partial]*: will return a `ErrMode::Incomplete(Needed::new(N))` if the input doesn't
    /// contain the literal or if the input is smaller than the literal.
    ///
    /// See also
    /// - [`take_till`] for recognizing up-to a [set of tokens][ContainsToken]
    /// - [`repeat_till`][crate::combinator::repeat_till] with [`Parser::take`] for taking tokens up to a [`Parser`]
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream] with `0..` or `1..` [ranges][Range]:
    /// ```rust
    /// # use std::ops::RangeFrom;
    /// # use winnow::prelude::*;;
    /// # use winnow::error::ContextError;
    /// pub fn take_until(occurrences: RangeFrom<usize>, literal: &str) -> impl Parser<&str, &str, ContextError>
    /// # {
    /// #     winnow::token::take_until(occurrences, literal)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_until;
    ///
    /// fn until_eof<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   take_until(0.., "eof").parse_next(s)
    /// }
    ///
    /// assert_eq!(until_eof.parse_peek("hello, worldeof"), Ok(("eof", "hello, world")));
    /// assert!(until_eof.parse_peek("hello, world").is_err());
    /// assert!(until_eof.parse_peek("").is_err());
    /// assert_eq!(until_eof.parse_peek("1eof2eof"), Ok(("eof2eof", "1")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_until;
    ///
    /// fn until_eof<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take_until(0.., "eof").parse_next(s)
    /// }
    ///
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, worldeof")), Ok((Partial::new("eof"), "hello, world")));
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, world")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, worldeo")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert_eq!(until_eof.parse_peek(Partial::new("1eof2eof")), Ok((Partial::new("eof2eof"), "1")));
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// use winnow::token::take_until;
    ///
    /// fn until_eof<'i>(s: &mut &'i str) -> ModalResult<&'i str> {
    ///   take_until(1.., "eof").parse_next(s)
    /// }
    ///
    /// assert_eq!(until_eof.parse_peek("hello, worldeof"), Ok(("eof", "hello, world")));
    /// assert!(until_eof.parse_peek("hello, world").is_err());
    /// assert!(until_eof.parse_peek("").is_err());
    /// assert_eq!(until_eof.parse_peek("1eof2eof"), Ok(("eof2eof", "1")));
    /// assert!(until_eof.parse_peek("eof").is_err());
    /// ```
    ///
    /// ```rust
    /// # use winnow::{error::ErrMode, error::ContextError, error::Needed};
    /// # use winnow::prelude::*;
    /// # use winnow::Partial;
    /// use winnow::token::take_until;
    ///
    /// fn until_eof<'i>(s: &mut Partial<&'i str>) -> ModalResult<&'i str> {
    ///   take_until(1.., "eof").parse_next(s)
    /// }
    ///
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, worldeof")), Ok((Partial::new("eof"), "hello, world")));
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, world")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert_eq!(until_eof.parse_peek(Partial::new("hello, worldeo")), Err(ErrMode::Incomplete(Needed::Unknown)));
    /// assert_eq!(until_eof.parse_peek(Partial::new("1eof2eof")), Ok((Partial::new("eof2eof"), "1")));
    /// assert!(until_eof.parse_peek(Partial::new("eof")).is_err());
    /// ```
    #[inline(always)]
    pub fn take_until<Literal, Input, Error>(
        occurrences: impl Into<Range>,
        literal: Literal,
    ) -> impl Parser<Input, <Input as Stream>::Slice, Error>
    where
        Input: StreamIsPartial + Stream + FindSlice<Literal>,
        Literal: Clone,
        Error: ParserError<Input>,
    {
        let Range { start_inclusive, end_inclusive } = occurrences.into();
        trace(
            "take_until",
            move |i: &mut Input| {
                match (start_inclusive, end_inclusive) {
                    (0, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_until0_::<_, _, _, true>(i, literal.clone())
                        } else {
                            take_until0_::<_, _, _, false>(i, literal.clone())
                        }
                    }
                    (1, None) => {
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_until1_::<_, _, _, true>(i, literal.clone())
                        } else {
                            take_until1_::<_, _, _, false>(i, literal.clone())
                        }
                    }
                    (start, end) => {
                        let end = end.unwrap_or(usize::MAX);
                        if <Input as StreamIsPartial>::is_partial_supported() {
                            take_until_m_n_::<
                                _,
                                _,
                                _,
                                true,
                            >(i, start, end, literal.clone())
                        } else {
                            take_until_m_n_::<
                                _,
                                _,
                                _,
                                false,
                            >(i, start, end, literal.clone())
                        }
                    }
                }
            },
        )
    }
    fn take_until0_<T, I, Error: ParserError<I>, const PARTIAL: bool>(
        i: &mut I,
        t: T,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream + FindSlice<T>,
    {
        match i.find_slice(t) {
            Some(range) => Ok(i.next_slice(range.start)),
            None if PARTIAL && i.is_partial() => {
                Err(ParserError::incomplete(i, Needed::Unknown))
            }
            None => Err(ParserError::from_input(i)),
        }
    }
    fn take_until1_<T, I, Error: ParserError<I>, const PARTIAL: bool>(
        i: &mut I,
        t: T,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream + FindSlice<T>,
    {
        match i.find_slice(t) {
            None if PARTIAL && i.is_partial() => {
                Err(ParserError::incomplete(i, Needed::Unknown))
            }
            None => Err(ParserError::from_input(i)),
            Some(range) => {
                if range.start == 0 {
                    Err(ParserError::from_input(i))
                } else {
                    Ok(i.next_slice(range.start))
                }
            }
        }
    }
    fn take_until_m_n_<T, I, Error: ParserError<I>, const PARTIAL: bool>(
        i: &mut I,
        start: usize,
        end: usize,
        t: T,
    ) -> Result<<I as Stream>::Slice, Error>
    where
        I: StreamIsPartial,
        I: Stream + FindSlice<T>,
    {
        if end < start {
            return Err(
                ParserError::assert(
                    i,
                    "`occurrences` should be ascending, rather than descending",
                ),
            );
        }
        match i.find_slice(t) {
            Some(range) => {
                let start_offset = i.offset_at(start);
                let end_offset = i.offset_at(end).unwrap_or_else(|_err| i.eof_offset());
                if start_offset.map(|s| range.start < s).unwrap_or(true) {
                    if PARTIAL && i.is_partial() {
                        return Err(ParserError::incomplete(i, Needed::Unknown));
                    } else {
                        return Err(ParserError::from_input(i));
                    }
                }
                if end_offset < range.start {
                    return Err(ParserError::from_input(i));
                }
                Ok(i.next_slice(range.start))
            }
            None if PARTIAL && i.is_partial() => {
                Err(ParserError::incomplete(i, Needed::Unknown))
            }
            None => Err(ParserError::from_input(i)),
        }
    }
    /// Return the remaining input.
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn rest<'i>(input: &mut &'i str) -> ModalResult<&'i str>
    /// # {
    /// #     winnow::token::rest.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::error::ContextError;
    /// use winnow::token::rest;
    /// assert_eq!(rest::<_,ContextError>.parse_peek("abc"), Ok(("", "abc")));
    /// assert_eq!(rest::<_,ContextError>.parse_peek(""), Ok(("", "")));
    /// ```
    #[inline]
    pub fn rest<Input, Error>(
        input: &mut Input,
    ) -> Result<<Input as Stream>::Slice, Error>
    where
        Input: Stream,
        Error: ParserError<Input>,
    {
        trace("rest", move |input: &mut Input| Ok(input.finish())).parse_next(input)
    }
    /// Return the length of the remaining input.
    ///
    /// <div class="warning">
    ///
    /// Note: this does not advance the [`Stream`]
    ///
    /// </div>
    ///
    /// # Effective Signature
    ///
    /// Assuming you are parsing a `&str` [Stream]:
    /// ```rust
    /// # use winnow::prelude::*;;
    /// pub fn rest_len(input: &mut &str) -> ModalResult<usize>
    /// # {
    /// #     winnow::token::rest_len.parse_next(input)
    /// # }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # use winnow::prelude::*;
    /// # use winnow::error::ContextError;
    /// use winnow::token::rest_len;
    /// assert_eq!(rest_len::<_,ContextError>.parse_peek("abc"), Ok(("abc", 3)));
    /// assert_eq!(rest_len::<_,ContextError>.parse_peek(""), Ok(("", 0)));
    /// ```
    #[inline]
    pub fn rest_len<Input, Error>(input: &mut Input) -> Result<usize, Error>
    where
        Input: Stream,
        Error: ParserError<Input>,
    {
        trace(
                "rest_len",
                move |input: &mut Input| {
                    let len = input.eof_offset();
                    Ok(len)
                },
            )
            .parse_next(input)
    }
}
/// Core concepts available for glob import
///
/// Including
/// - [`StreamIsPartial`][crate::stream::StreamIsPartial]
/// - [`Parser`]
///
/// ## Example
///
/// ```rust
/// # #[cfg(feature = "ascii")] {
/// use winnow::prelude::*;
///
/// fn parse_data(input: &mut &str) -> ModalResult<u64> {
///     // ...
/// #   winnow::ascii::dec_uint(input)
/// }
///
/// fn main() {
///   let result = parse_data.parse("100");
///   assert_eq!(result, Ok(100));
/// }
/// # }
/// ```
pub mod prelude {
    pub use crate::error::ModalError as _;
    pub use crate::error::ParserError as _;
    pub use crate::stream::AsChar as _;
    pub use crate::stream::ContainsToken as _;
    pub use crate::stream::Stream as _;
    pub use crate::stream::StreamIsPartial as _;
    pub use crate::ModalParser;
    pub use crate::ModalResult;
    pub use crate::Parser;
}
pub use error::ModalResult;
pub use error::Result;
pub use parser::{ModalParser, Parser};
pub use stream::BStr;
pub use stream::Bytes;
pub use stream::LocatingSlice;
pub use stream::Partial;
pub use stream::Stateful;
pub use stream::Str;
