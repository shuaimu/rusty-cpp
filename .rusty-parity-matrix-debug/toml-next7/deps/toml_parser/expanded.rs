#![feature(prelude_import)]
//! TOML lexer and parser
//!
//! Characteristics:
//! - Error recovery
//! - Lazy validation
//! - `forbid(unsafe)` by default, requiring the `unsafe` feature otherwise
//! - `no_std` support, including putting users in charge of allocation choices (including not
//!   allocating)
//!
//! Full parsing is broken into three phases:
//! 1. [Lexing tokens][lexer]
//! 2. [Parsing tokens][parser] (push parser)
//! 3. Organizing the physical layout into the logical layout,
//!    including [decoding keys and values][decoder]
#![forbid(unsafe_code)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
extern crate alloc;
#[macro_use]
mod macros {}
mod error {
    use crate::Span;
    pub trait ErrorSink {
        fn report_error(&mut self, error: ParseError);
    }
    impl<F> ErrorSink for F
    where
        F: FnMut(ParseError),
    {
        fn report_error(&mut self, error: ParseError) {
            (self)(error);
        }
    }
    impl ErrorSink for () {
        fn report_error(&mut self, _error: ParseError) {}
    }
    impl ErrorSink for Option<ParseError> {
        fn report_error(&mut self, error: ParseError) {
            self.get_or_insert(error);
        }
    }
    #[allow(unused_qualifications)]
    impl ErrorSink for alloc::vec::Vec<ParseError> {
        fn report_error(&mut self, error: ParseError) {
            self.push(error);
        }
    }
    #[non_exhaustive]
    pub struct ParseError {
        context: Option<Span>,
        description: ErrorStr,
        expected: Option<&'static [Expected]>,
        unexpected: Option<Span>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ParseError {
        #[inline]
        fn clone(&self) -> ParseError {
            ParseError {
                context: ::core::clone::Clone::clone(&self.context),
                description: ::core::clone::Clone::clone(&self.description),
                expected: ::core::clone::Clone::clone(&self.expected),
                unexpected: ::core::clone::Clone::clone(&self.unexpected),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for ParseError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ParseError {
        #[inline]
        fn eq(&self, other: &ParseError) -> bool {
            self.context == other.context && self.description == other.description
                && self.expected == other.expected && self.unexpected == other.unexpected
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for ParseError {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<Option<Span>>;
            let _: ::core::cmp::AssertParamIsEq<ErrorStr>;
            let _: ::core::cmp::AssertParamIsEq<Option<&'static [Expected]>>;
            let _: ::core::cmp::AssertParamIsEq<Option<Span>>;
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ParseError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "ParseError",
                "context",
                &self.context,
                "description",
                &self.description,
                "expected",
                &self.expected,
                "unexpected",
                &&self.unexpected,
            )
        }
    }
    impl ParseError {
        pub fn new(description: impl Into<ErrorStr>) -> Self {
            Self {
                context: None,
                description: description.into(),
                expected: None,
                unexpected: None,
            }
        }
        pub fn with_context(mut self, context: Span) -> Self {
            self.context = Some(context);
            self
        }
        pub fn with_expected(mut self, expected: &'static [Expected]) -> Self {
            self.expected = Some(expected);
            self
        }
        pub fn with_unexpected(mut self, unexpected: Span) -> Self {
            self.unexpected = Some(unexpected);
            self
        }
        pub fn context(&self) -> Option<Span> {
            self.context
        }
        pub fn description(&self) -> &str {
            &self.description
        }
        pub fn expected(&self) -> Option<&'static [Expected]> {
            self.expected
        }
        pub fn unexpected(&self) -> Option<Span> {
            self.unexpected
        }
        pub(crate) fn rebase_spans(mut self, offset: usize) -> Self {
            if let Some(context) = self.context.as_mut() {
                *context += offset;
            }
            if let Some(unexpected) = self.unexpected.as_mut() {
                *unexpected += offset;
            }
            self
        }
    }
    type ErrorStr = alloc::borrow::Cow<'static, str>;
    #[non_exhaustive]
    pub enum Expected {
        Literal(&'static str),
        Description(&'static str),
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Expected {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Expected {}
    #[automatically_derived]
    impl ::core::clone::Clone for Expected {
        #[inline]
        fn clone(&self) -> Expected {
            let _: ::core::clone::AssertParamIsClone<&'static str>;
            let _: ::core::clone::AssertParamIsClone<&'static str>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Expected {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Expected {
        #[inline]
        fn eq(&self, other: &Expected) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (Expected::Literal(__self_0), Expected::Literal(__arg1_0)) => {
                        __self_0 == __arg1_0
                    }
                    (
                        Expected::Description(__self_0),
                        Expected::Description(__arg1_0),
                    ) => __self_0 == __arg1_0,
                    _ => unsafe { ::core::intrinsics::unreachable() }
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Expected {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'static str>;
            let _: ::core::cmp::AssertParamIsEq<&'static str>;
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Expected {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Expected::Literal(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Literal",
                        &__self_0,
                    )
                }
                Expected::Description(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Description",
                        &__self_0,
                    )
                }
            }
        }
    }
}
mod source {
    use crate::ErrorSink;
    use crate::Expected;
    use crate::decoder::Encoding;
    use crate::decoder::StringBuilder;
    use crate::lexer::Lexer;
    /// Data encoded as TOML
    pub struct Source<'i> {
        input: &'i str,
    }
    #[automatically_derived]
    impl<'i> ::core::marker::Copy for Source<'i> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'i> ::core::clone::TrivialClone for Source<'i> {}
    #[automatically_derived]
    impl<'i> ::core::clone::Clone for Source<'i> {
        #[inline]
        fn clone(&self) -> Source<'i> {
            let _: ::core::clone::AssertParamIsClone<&'i str>;
            *self
        }
    }
    #[automatically_derived]
    impl<'i> ::core::fmt::Debug for Source<'i> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Source",
                "input",
                &&self.input,
            )
        }
    }
    #[automatically_derived]
    impl<'i> ::core::marker::StructuralPartialEq for Source<'i> {}
    #[automatically_derived]
    impl<'i> ::core::cmp::PartialEq for Source<'i> {
        #[inline]
        fn eq(&self, other: &Source<'i>) -> bool {
            self.input == other.input
        }
    }
    #[automatically_derived]
    impl<'i> ::core::cmp::Eq for Source<'i> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'i str>;
        }
    }
    impl<'i> Source<'i> {
        pub fn new(input: &'i str) -> Self {
            Self { input }
        }
        /// Start lexing the TOML encoded data
        pub fn lex(&self) -> Lexer<'i> {
            Lexer::new(self.input)
        }
        /// Access the TOML encoded `&str`
        pub fn input(&self) -> &'i str {
            self.input
        }
        /// Return a subslice of the input
        pub fn get(&self, span: impl SourceIndex) -> Option<Raw<'i>> {
            span.get(self)
        }
        /// Return a subslice of the input
        fn get_raw_str(&self, span: Span) -> Option<&'i str> {
            let index = span.start()..span.end();
            self.input.get(index)
        }
    }
    /// A slice of [`Source`]
    pub struct Raw<'i> {
        raw: &'i str,
        encoding: Option<Encoding>,
        span: Span,
    }
    #[automatically_derived]
    impl<'i> ::core::marker::Copy for Raw<'i> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'i> ::core::clone::TrivialClone for Raw<'i> {}
    #[automatically_derived]
    impl<'i> ::core::clone::Clone for Raw<'i> {
        #[inline]
        fn clone(&self) -> Raw<'i> {
            let _: ::core::clone::AssertParamIsClone<&'i str>;
            let _: ::core::clone::AssertParamIsClone<Option<Encoding>>;
            let _: ::core::clone::AssertParamIsClone<Span>;
            *self
        }
    }
    #[automatically_derived]
    impl<'i> ::core::fmt::Debug for Raw<'i> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Raw",
                "raw",
                &self.raw,
                "encoding",
                &self.encoding,
                "span",
                &&self.span,
            )
        }
    }
    impl<'i> Raw<'i> {
        pub fn new_unchecked(
            raw: &'i str,
            encoding: Option<Encoding>,
            span: Span,
        ) -> Self {
            Self { raw, encoding, span }
        }
        pub fn decode_key(
            &self,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            let mut error = |err: crate::ParseError| {
                error.report_error(err.rebase_spans(self.span.start));
            };
            match self.encoding {
                Some(Encoding::LiteralString) => {
                    crate::decoder::string::decode_literal_string(
                        *self,
                        output,
                        &mut error,
                    );
                }
                Some(Encoding::BasicString) => {
                    crate::decoder::string::decode_basic_string(
                        *self,
                        output,
                        &mut error,
                    );
                }
                Some(Encoding::MlLiteralString) => {
                    error
                        .report_error(
                            crate::ParseError::new(
                                    "keys cannot be multi-line literal strings",
                                )
                                .with_expected(
                                    &[
                                        Expected::Description("basic string"),
                                        Expected::Description("literal string"),
                                    ],
                                )
                                .with_unexpected(Span::new_unchecked(0, self.len())),
                        );
                    crate::decoder::string::decode_ml_literal_string(
                        *self,
                        output,
                        &mut error,
                    );
                }
                Some(Encoding::MlBasicString) => {
                    error
                        .report_error(
                            crate::ParseError::new(
                                    "keys cannot be multi-line basic strings",
                                )
                                .with_expected(
                                    &[
                                        Expected::Description("basic string"),
                                        Expected::Description("literal string"),
                                    ],
                                )
                                .with_unexpected(Span::new_unchecked(0, self.len())),
                        );
                    crate::decoder::string::decode_ml_basic_string(
                        *self,
                        output,
                        &mut error,
                    );
                }
                None => {
                    crate::decoder::string::decode_unquoted_key(
                        *self,
                        output,
                        &mut error,
                    )
                }
            }
        }
        #[must_use]
        pub fn decode_scalar(
            &self,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> crate::decoder::scalar::ScalarKind {
            let mut error = |err: crate::ParseError| {
                error.report_error(err.rebase_spans(self.span.start));
            };
            match self.encoding {
                Some(Encoding::LiteralString) => {
                    crate::decoder::string::decode_literal_string(
                        *self,
                        output,
                        &mut error,
                    );
                    crate::decoder::scalar::ScalarKind::String
                }
                Some(Encoding::BasicString) => {
                    crate::decoder::string::decode_basic_string(
                        *self,
                        output,
                        &mut error,
                    );
                    crate::decoder::scalar::ScalarKind::String
                }
                Some(Encoding::MlLiteralString) => {
                    crate::decoder::string::decode_ml_literal_string(
                        *self,
                        output,
                        &mut error,
                    );
                    crate::decoder::scalar::ScalarKind::String
                }
                Some(Encoding::MlBasicString) => {
                    crate::decoder::string::decode_ml_basic_string(
                        *self,
                        output,
                        &mut error,
                    );
                    crate::decoder::scalar::ScalarKind::String
                }
                None => {
                    crate::decoder::scalar::decode_unquoted_scalar(
                        *self,
                        output,
                        &mut error,
                    )
                }
            }
        }
        pub fn decode_whitespace(&self, _error: &mut dyn ErrorSink) {}
        pub fn decode_comment(&self, error: &mut dyn ErrorSink) {
            let mut error = |err: crate::ParseError| {
                error.report_error(err.rebase_spans(self.span.start));
            };
            crate::decoder::ws::decode_comment(*self, &mut error);
        }
        pub fn decode_newline(&self, error: &mut dyn ErrorSink) {
            let mut error = |err: crate::ParseError| {
                error.report_error(err.rebase_spans(self.span.start));
            };
            crate::decoder::ws::decode_newline(*self, &mut error);
        }
        pub fn as_str(&self) -> &'i str {
            self.raw
        }
        pub fn as_bytes(&self) -> &'i [u8] {
            self.raw.as_bytes()
        }
        pub fn len(&self) -> usize {
            self.raw.len()
        }
        pub fn is_empty(&self) -> bool {
            self.raw.is_empty()
        }
    }
    /// Location within the [`Source`]
    pub struct Span {
        start: usize,
        end: usize,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Span {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Span {}
    #[automatically_derived]
    impl ::core::clone::Clone for Span {
        #[inline]
        fn clone(&self) -> Span {
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for Span {
        #[inline]
        fn default() -> Span {
            Span {
                start: ::core::default::Default::default(),
                end: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Span {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Span {
        #[inline]
        fn eq(&self, other: &Span) -> bool {
            self.start == other.start && self.end == other.end
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Span {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<usize>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for Span {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Span,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            match ::core::cmp::PartialOrd::partial_cmp(&self.start, &other.start) {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                    ::core::cmp::PartialOrd::partial_cmp(&self.end, &other.end)
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for Span {
        #[inline]
        fn cmp(&self, other: &Span) -> ::core::cmp::Ordering {
            match ::core::cmp::Ord::cmp(&self.start, &other.start) {
                ::core::cmp::Ordering::Equal => {
                    ::core::cmp::Ord::cmp(&self.end, &other.end)
                }
                cmp => cmp,
            }
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Span {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            ::core::hash::Hash::hash(&self.start, state);
            ::core::hash::Hash::hash(&self.end, state)
        }
    }
    impl Span {
        pub fn new_unchecked(start: usize, end: usize) -> Self {
            Self { start, end }
        }
        pub fn is_empty(&self) -> bool {
            self.end <= self.start
        }
        pub fn len(&self) -> usize {
            self.end - self.start
        }
        pub fn start(&self) -> usize {
            self.start
        }
        pub fn end(&self) -> usize {
            self.end
        }
        pub fn before(&self) -> Self {
            Self::new_unchecked(self.start, self.start)
        }
        pub fn after(&self) -> Self {
            Self::new_unchecked(self.end, self.end)
        }
        /// Extend this `Raw` to the end of `after`
        #[must_use]
        pub fn append(&self, after: Self) -> Self {
            Self::new_unchecked(self.start, after.end)
        }
    }
    impl core::fmt::Debug for Span {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            (self.start..self.end).fmt(f)
        }
    }
    impl core::ops::Add<usize> for Span {
        type Output = Self;
        fn add(self, offset: usize) -> Self::Output {
            Self::Output {
                start: self.start + offset,
                end: self.end + offset,
            }
        }
    }
    impl core::ops::Add<Span> for usize {
        type Output = Span;
        fn add(self, span: Span) -> Self::Output {
            Self::Output {
                start: span.start + self,
                end: span.end + self,
            }
        }
    }
    impl core::ops::AddAssign<usize> for Span {
        fn add_assign(&mut self, rhs: usize) {
            self.start += rhs;
            self.end += rhs;
        }
    }
    /// A helper trait used for indexing operations on [`Source`]
    pub trait SourceIndex: sealed::Sealed {
        /// Return a subslice of the input
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>>;
    }
    impl SourceIndex for Span {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            (&self).get(source)
        }
    }
    impl SourceIndex for &Span {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            let encoding = None;
            source.get_raw_str(*self).map(|s| Raw::new_unchecked(s, encoding, *self))
        }
    }
    impl SourceIndex for crate::lexer::Token {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            (&self).get(source)
        }
    }
    impl SourceIndex for &crate::lexer::Token {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            let encoding = self.kind().encoding();
            source
                .get_raw_str(self.span())
                .map(|s| Raw::new_unchecked(s, encoding, self.span()))
        }
    }
    impl SourceIndex for crate::parser::Event {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            (&self).get(source)
        }
    }
    impl SourceIndex for &crate::parser::Event {
        fn get<'i>(self, source: &Source<'i>) -> Option<Raw<'i>> {
            let encoding = self.encoding();
            source
                .get_raw_str(self.span())
                .map(|s| Raw::new_unchecked(s, encoding, self.span()))
        }
    }
    mod sealed {
        pub trait Sealed {}
        impl Sealed for crate::Span {}
        impl Sealed for &crate::Span {}
        impl Sealed for crate::lexer::Token {}
        impl Sealed for &crate::lexer::Token {}
        impl Sealed for crate::parser::Event {}
        impl Sealed for &crate::parser::Event {}
    }
}
pub mod decoder {
    //! Decode [raw][crate::Raw] TOML values into Rust native types
    //!
    //! See
    //! - [`Raw::decode_key`][crate::Raw::decode_key]
    //! - [`Raw::decode_scalar`][crate::Raw::decode_scalar]
    //! - [`Raw::decode_whitespace`][crate::Raw::decode_whitespace]
    //! - [`Raw::decode_comment`][crate::Raw::decode_comment]
    //! - [`Raw::decode_newline`][crate::Raw::decode_newline]
    use alloc::borrow::Cow;
    use alloc::string::String;
    pub(crate) mod scalar {
        use winnow::stream::ContainsToken as _;
        use winnow::stream::FindSlice as _;
        use winnow::stream::Offset as _;
        use winnow::stream::Stream as _;
        use crate::ErrorSink;
        use crate::Expected;
        use crate::ParseError;
        use crate::Raw;
        use crate::Span;
        use crate::decoder::StringBuilder;
        const ALLOCATION_ERROR: &str = "could not allocate for string";
        pub enum ScalarKind {
            String,
            Boolean(bool),
            DateTime,
            Float,
            Integer(IntegerRadix),
        }
        #[automatically_derived]
        impl ::core::marker::Copy for ScalarKind {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for ScalarKind {}
        #[automatically_derived]
        impl ::core::clone::Clone for ScalarKind {
            #[inline]
            fn clone(&self) -> ScalarKind {
                let _: ::core::clone::AssertParamIsClone<bool>;
                let _: ::core::clone::AssertParamIsClone<IntegerRadix>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for ScalarKind {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for ScalarKind {
            #[inline]
            fn eq(&self, other: &ScalarKind) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
                    && match (self, other) {
                        (
                            ScalarKind::Boolean(__self_0),
                            ScalarKind::Boolean(__arg1_0),
                        ) => __self_0 == __arg1_0,
                        (
                            ScalarKind::Integer(__self_0),
                            ScalarKind::Integer(__arg1_0),
                        ) => __self_0 == __arg1_0,
                        _ => true,
                    }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for ScalarKind {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<bool>;
                let _: ::core::cmp::AssertParamIsEq<IntegerRadix>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for ScalarKind {
            #[inline]
            fn partial_cmp(
                &self,
                other: &ScalarKind,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                match (self, other) {
                    (ScalarKind::Boolean(__self_0), ScalarKind::Boolean(__arg1_0)) => {
                        ::core::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0)
                    }
                    (ScalarKind::Integer(__self_0), ScalarKind::Integer(__arg1_0)) => {
                        ::core::cmp::PartialOrd::partial_cmp(__self_0, __arg1_0)
                    }
                    _ => {
                        ::core::cmp::PartialOrd::partial_cmp(
                            &__self_discr,
                            &__arg1_discr,
                        )
                    }
                }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for ScalarKind {
            #[inline]
            fn cmp(&self, other: &ScalarKind) -> ::core::cmp::Ordering {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                match ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr) {
                    ::core::cmp::Ordering::Equal => {
                        match (self, other) {
                            (
                                ScalarKind::Boolean(__self_0),
                                ScalarKind::Boolean(__arg1_0),
                            ) => ::core::cmp::Ord::cmp(__self_0, __arg1_0),
                            (
                                ScalarKind::Integer(__self_0),
                                ScalarKind::Integer(__arg1_0),
                            ) => ::core::cmp::Ord::cmp(__self_0, __arg1_0),
                            _ => ::core::cmp::Ordering::Equal,
                        }
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for ScalarKind {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_discr, state);
                match self {
                    ScalarKind::Boolean(__self_0) => {
                        ::core::hash::Hash::hash(__self_0, state)
                    }
                    ScalarKind::Integer(__self_0) => {
                        ::core::hash::Hash::hash(__self_0, state)
                    }
                    _ => {}
                }
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for ScalarKind {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    ScalarKind::String => ::core::fmt::Formatter::write_str(f, "String"),
                    ScalarKind::Boolean(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Boolean",
                            &__self_0,
                        )
                    }
                    ScalarKind::DateTime => {
                        ::core::fmt::Formatter::write_str(f, "DateTime")
                    }
                    ScalarKind::Float => ::core::fmt::Formatter::write_str(f, "Float"),
                    ScalarKind::Integer(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "Integer",
                            &__self_0,
                        )
                    }
                }
            }
        }
        impl ScalarKind {
            pub fn description(&self) -> &'static str {
                match self {
                    Self::String => "string",
                    Self::Boolean(_) => "boolean",
                    Self::DateTime => "date-time",
                    Self::Float => "float",
                    Self::Integer(radix) => radix.description(),
                }
            }
            pub fn invalid_description(&self) -> &'static str {
                match self {
                    Self::String => "invalid string",
                    Self::Boolean(_) => "invalid boolean",
                    Self::DateTime => "invalid date-time",
                    Self::Float => "invalid float",
                    Self::Integer(radix) => radix.invalid_description(),
                }
            }
        }
        pub enum IntegerRadix {
            #[default]
            Dec,
            Hex,
            Oct,
            Bin,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for IntegerRadix {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for IntegerRadix {}
        #[automatically_derived]
        impl ::core::clone::Clone for IntegerRadix {
            #[inline]
            fn clone(&self) -> IntegerRadix {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::default::Default for IntegerRadix {
            #[inline]
            fn default() -> IntegerRadix {
                Self::Dec
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for IntegerRadix {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for IntegerRadix {
            #[inline]
            fn eq(&self, other: &IntegerRadix) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for IntegerRadix {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {}
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for IntegerRadix {
            #[inline]
            fn partial_cmp(
                &self,
                other: &IntegerRadix,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for IntegerRadix {
            #[inline]
            fn cmp(&self, other: &IntegerRadix) -> ::core::cmp::Ordering {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for IntegerRadix {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_discr, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for IntegerRadix {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        IntegerRadix::Dec => "Dec",
                        IntegerRadix::Hex => "Hex",
                        IntegerRadix::Oct => "Oct",
                        IntegerRadix::Bin => "Bin",
                    },
                )
            }
        }
        impl IntegerRadix {
            pub fn description(&self) -> &'static str {
                match self {
                    Self::Dec => "integer",
                    Self::Hex => "hexadecimal",
                    Self::Oct => "octal",
                    Self::Bin => "binary",
                }
            }
            pub fn value(&self) -> u32 {
                match self {
                    Self::Dec => 10,
                    Self::Hex => 16,
                    Self::Oct => 8,
                    Self::Bin => 2,
                }
            }
            pub fn invalid_description(&self) -> &'static str {
                match self {
                    Self::Dec => "invalid integer number",
                    Self::Hex => "invalid hexadecimal number",
                    Self::Oct => "invalid octal number",
                    Self::Bin => "invalid binary number",
                }
            }
            fn validator(&self) -> fn(char) -> bool {
                match self {
                    Self::Dec => |c| c.is_ascii_digit(),
                    Self::Hex => |c| c.is_ascii_hexdigit(),
                    Self::Oct => {
                        |c| {
                            #[allow(non_exhaustive_omitted_patterns)]
                            match c {
                                '0'..='7' => true,
                                _ => false,
                            }
                        }
                    }
                    Self::Bin => {
                        |c| {
                            #[allow(non_exhaustive_omitted_patterns)]
                            match c {
                                '0'..='1' => true,
                                _ => false,
                            }
                        }
                    }
                }
            }
        }
        pub(crate) fn decode_unquoted_scalar<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            let s = raw.as_str();
            let Some(first) = s.as_bytes().first() else {
                return decode_invalid(raw, output, error);
            };
            if !first.is_ascii_digit() && s.contains(" ") {
                return decode_invalid(raw, output, error);
            }
            match first {
                b'+' | b'-' => {
                    let value = &raw.as_str()[1..];
                    decode_sign_prefix(raw, value, output, error)
                }
                b'_' => {
                    decode_datetime_or_float_or_integer(raw.as_str(), raw, output, error)
                }
                b'0' => decode_zero_prefix(raw.as_str(), false, raw, output, error),
                b'1'..=b'9' => {
                    decode_datetime_or_float_or_integer(raw.as_str(), raw, output, error)
                }
                b'.' => {
                    let kind = ScalarKind::Float;
                    let stream = raw.as_str();
                    if ensure_float(stream, raw, error) {
                        decode_float_or_integer(stream, raw, kind, output, error)
                    } else {
                        kind
                    }
                }
                b't' | b'T' => {
                    const SYMBOL: &str = "true";
                    let kind = ScalarKind::Boolean(true);
                    let expected = &[Expected::Literal(SYMBOL)];
                    decode_symbol(raw, SYMBOL, kind, expected, output, error)
                }
                b'f' | b'F' => {
                    const SYMBOL: &str = "false";
                    let kind = ScalarKind::Boolean(false);
                    let expected = &[Expected::Literal(SYMBOL)];
                    decode_symbol(raw, SYMBOL, kind, expected, output, error)
                }
                b'i' | b'I' => {
                    const SYMBOL: &str = "inf";
                    let kind = ScalarKind::Float;
                    let expected = &[Expected::Literal(SYMBOL)];
                    decode_symbol(raw, SYMBOL, kind, expected, output, error)
                }
                b'n' | b'N' => {
                    const SYMBOL: &str = "nan";
                    let kind = ScalarKind::Float;
                    let expected = &[Expected::Literal(SYMBOL)];
                    decode_symbol(raw, SYMBOL, kind, expected, output, error)
                }
                _ => decode_invalid(raw, output, error),
            }
        }
        fn decode_sign_prefix<'i>(
            raw: Raw<'i>,
            value: &'i str,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            let mut value = value;
            let first = loop {
                let Some(first) = value.as_bytes().first() else {
                    return decode_invalid(raw, output, error);
                };
                if !#[allow(non_exhaustive_omitted_patterns)]
                match first {
                    b'+' | b'-' => true,
                    _ => false,
                } {
                    break first;
                }
                let start = value.offset_from(&raw.as_str());
                let end = start + 1;
                error
                    .report_error(
                        ParseError::new("redundant numeric sign")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[])
                            .with_unexpected(Span::new_unchecked(start, end)),
                    );
                value = &value[1..];
            };
            match first {
                b'_' => decode_datetime_or_float_or_integer(value, raw, output, error),
                b'0' => decode_zero_prefix(value, true, raw, output, error),
                b'1'..=b'9' => {
                    decode_datetime_or_float_or_integer(value, raw, output, error)
                }
                b'.' => {
                    let kind = ScalarKind::Float;
                    let stream = raw.as_str();
                    if ensure_float(stream, raw, error) {
                        decode_float_or_integer(stream, raw, kind, output, error)
                    } else {
                        kind
                    }
                }
                b'i' | b'I' => {
                    const SYMBOL: &str = "inf";
                    let kind = ScalarKind::Float;
                    if value != SYMBOL {
                        let expected = &[Expected::Literal(SYMBOL)];
                        let start = value.offset_from(&raw.as_str());
                        let end = start + value.len();
                        error
                            .report_error(
                                ParseError::new(kind.invalid_description())
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(expected)
                                    .with_unexpected(Span::new_unchecked(start, end)),
                            );
                        decode_as(raw, SYMBOL, kind, output, error)
                    } else {
                        decode_as_is(raw, kind, output, error)
                    }
                }
                b'n' | b'N' => {
                    const SYMBOL: &str = "nan";
                    let kind = ScalarKind::Float;
                    if value != SYMBOL {
                        let expected = &[Expected::Literal(SYMBOL)];
                        let start = value.offset_from(&raw.as_str());
                        let end = start + value.len();
                        error
                            .report_error(
                                ParseError::new(kind.invalid_description())
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(expected)
                                    .with_unexpected(Span::new_unchecked(start, end)),
                            );
                        decode_as(raw, SYMBOL, kind, output, error)
                    } else {
                        decode_as_is(raw, kind, output, error)
                    }
                }
                _ => decode_invalid(raw, output, error),
            }
        }
        fn decode_zero_prefix<'i>(
            value: &'i str,
            signed: bool,
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            if true {
                match (&value.as_bytes()[0], &b'0') {
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
            if value.len() == 1 {
                let kind = ScalarKind::Integer(IntegerRadix::Dec);
                decode_float_or_integer(raw.as_str(), raw, kind, output, error)
            } else {
                let radix = value.as_bytes()[1];
                match radix {
                    b'x' | b'X' => {
                        if value.contains(" ") {
                            return decode_invalid(raw, output, error);
                        }
                        if signed {
                            error
                                .report_error(
                                    ParseError::new("integers with a radix cannot be signed")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[])
                                        .with_unexpected(Span::new_unchecked(0, 1)),
                                );
                        }
                        if radix == b'X' {
                            let start = value.offset_from(&raw.as_str());
                            let end = start + 2;
                            error
                                .report_error(
                                    ParseError::new("radix must be lowercase")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[Expected::Literal("0x")])
                                        .with_unexpected(Span::new_unchecked(start, end)),
                                );
                        }
                        let radix = IntegerRadix::Hex;
                        let kind = ScalarKind::Integer(radix);
                        let stream = &value[2..];
                        if ensure_radixed_value(stream, raw, radix, error) {
                            decode_float_or_integer(stream, raw, kind, output, error)
                        } else {
                            kind
                        }
                    }
                    b'o' | b'O' => {
                        if value.contains(" ") {
                            return decode_invalid(raw, output, error);
                        }
                        if signed {
                            error
                                .report_error(
                                    ParseError::new("integers with a radix cannot be signed")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[])
                                        .with_unexpected(Span::new_unchecked(0, 1)),
                                );
                        }
                        if radix == b'O' {
                            let start = value.offset_from(&raw.as_str());
                            let end = start + 2;
                            error
                                .report_error(
                                    ParseError::new("radix must be lowercase")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[Expected::Literal("0o")])
                                        .with_unexpected(Span::new_unchecked(start, end)),
                                );
                        }
                        let radix = IntegerRadix::Oct;
                        let kind = ScalarKind::Integer(radix);
                        let stream = &value[2..];
                        if ensure_radixed_value(stream, raw, radix, error) {
                            decode_float_or_integer(stream, raw, kind, output, error)
                        } else {
                            kind
                        }
                    }
                    b'b' | b'B' => {
                        if value.contains(" ") {
                            return decode_invalid(raw, output, error);
                        }
                        if signed {
                            error
                                .report_error(
                                    ParseError::new("integers with a radix cannot be signed")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[])
                                        .with_unexpected(Span::new_unchecked(0, 1)),
                                );
                        }
                        if radix == b'B' {
                            let start = value.offset_from(&raw.as_str());
                            let end = start + 2;
                            error
                                .report_error(
                                    ParseError::new("radix must be lowercase")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[Expected::Literal("0b")])
                                        .with_unexpected(Span::new_unchecked(start, end)),
                                );
                        }
                        let radix = IntegerRadix::Bin;
                        let kind = ScalarKind::Integer(radix);
                        let stream = &value[2..];
                        if ensure_radixed_value(stream, raw, radix, error) {
                            decode_float_or_integer(stream, raw, kind, output, error)
                        } else {
                            kind
                        }
                    }
                    b'd' | b'D' => {
                        if value.contains(" ") {
                            return decode_invalid(raw, output, error);
                        }
                        if signed {
                            error
                                .report_error(
                                    ParseError::new("integers with a radix cannot be signed")
                                        .with_context(Span::new_unchecked(0, raw.len()))
                                        .with_expected(&[])
                                        .with_unexpected(Span::new_unchecked(0, 1)),
                                );
                        }
                        let radix = IntegerRadix::Dec;
                        let kind = ScalarKind::Integer(radix);
                        let stream = &value[2..];
                        error
                            .report_error(
                                ParseError::new("redundant integer number prefix")
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[])
                                    .with_unexpected(Span::new_unchecked(0, 2)),
                            );
                        if ensure_radixed_value(stream, raw, radix, error) {
                            decode_float_or_integer(stream, raw, kind, output, error)
                        } else {
                            kind
                        }
                    }
                    _ => decode_datetime_or_float_or_integer(value, raw, output, error),
                }
            }
        }
        fn decode_datetime_or_float_or_integer<'i>(
            value: &'i str,
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            let Some(digit_end) = value
                .as_bytes()
                .offset_for(|b| !(b'0'..=b'9').contains_token(b)) else {
                let kind = ScalarKind::Integer(IntegerRadix::Dec);
                let stream = raw.as_str();
                if ensure_no_leading_zero(value, raw, error) {
                    return decode_float_or_integer(stream, raw, kind, output, error);
                } else {
                    return kind;
                }
            };
            let rest = &value[digit_end..];
            if rest.starts_with("-") || rest.starts_with(":") {
                decode_as_is(raw, ScalarKind::DateTime, output, error)
            } else if rest.contains(" ") {
                decode_invalid(raw, output, error)
            } else if is_float(rest) {
                let kind = ScalarKind::Float;
                let stream = raw.as_str();
                if ensure_float(value, raw, error) {
                    decode_float_or_integer(stream, raw, kind, output, error)
                } else {
                    kind
                }
            } else if rest.starts_with("_") {
                let kind = ScalarKind::Integer(IntegerRadix::Dec);
                let stream = raw.as_str();
                if ensure_no_leading_zero(value, raw, error) {
                    decode_float_or_integer(stream, raw, kind, output, error)
                } else {
                    kind
                }
            } else {
                decode_invalid(raw, output, error)
            }
        }
        /// ```abnf
        /// ;; Float
        ///
        /// float = float-int-part ( exp / frac [ exp ] )
        ///
        /// float-int-part = dec-int
        /// frac = decimal-point zero-prefixable-int
        /// decimal-point = %x2E               ; .
        /// zero-prefixable-int = DIGIT *( DIGIT / underscore DIGIT )
        ///
        /// exp = "e" float-exp-part
        /// float-exp-part = [ minus / plus ] zero-prefixable-int
        /// ```
        #[must_use]
        fn ensure_float<'i>(
            mut value: &'i str,
            raw: Raw<'i>,
            error: &mut dyn ErrorSink,
        ) -> bool {
            let mut is_valid = true;
            is_valid
                &= ensure_dec_uint(&mut value, raw, false, "invalid mantissa", error);
            if value.starts_with(".") {
                let _ = value.next_token();
                is_valid
                    &= ensure_dec_uint(&mut value, raw, true, "invalid fraction", error);
            }
            if value.starts_with(['e', 'E']) {
                let _ = value.next_token();
                if value.starts_with(['+', '-']) {
                    let _ = value.next_token();
                }
                is_valid
                    &= ensure_dec_uint(&mut value, raw, true, "invalid exponent", error);
            }
            if !value.is_empty() {
                let start = value.offset_from(&raw.as_str());
                let end = raw.len();
                error
                    .report_error(
                        ParseError::new(ScalarKind::Float.invalid_description())
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[])
                            .with_unexpected(Span::new_unchecked(start, end)),
                    );
                is_valid = false;
            }
            is_valid
        }
        #[must_use]
        fn ensure_dec_uint<'i>(
            value: &mut &'i str,
            raw: Raw<'i>,
            zero_prefix: bool,
            invalid_description: &'static str,
            error: &mut dyn ErrorSink,
        ) -> bool {
            let mut is_valid = true;
            let start = *value;
            let mut digit_count = 0;
            while let Some(current) = value.chars().next() {
                if current.is_ascii_digit() {
                    digit_count += 1;
                } else if current == '_' {} else {
                    break;
                }
                let _ = value.next_token();
            }
            match digit_count {
                0 => {
                    let start = start.offset_from(&raw.as_str());
                    let end = start;
                    error
                        .report_error(
                            ParseError::new(invalid_description)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(&[Expected::Description("digits")])
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                    is_valid = false;
                }
                1 => {}
                _ if start.starts_with("0") && !zero_prefix => {
                    let start = start.offset_from(&raw.as_str());
                    let end = start + 1;
                    error
                        .report_error(
                            ParseError::new("unexpected leading zero")
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(&[])
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                    is_valid = false;
                }
                _ => {}
            }
            is_valid
        }
        #[must_use]
        fn ensure_no_leading_zero<'i>(
            value: &'i str,
            raw: Raw<'i>,
            error: &mut dyn ErrorSink,
        ) -> bool {
            let mut is_valid = true;
            if value.starts_with("0") {
                let start = value.offset_from(&raw.as_str());
                let end = start + 1;
                error
                    .report_error(
                        ParseError::new("unexpected leading zero")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[])
                            .with_unexpected(Span::new_unchecked(start, end)),
                    );
                is_valid = false;
            }
            is_valid
        }
        #[must_use]
        fn ensure_radixed_value(
            value: &str,
            raw: Raw<'_>,
            radix: IntegerRadix,
            error: &mut dyn ErrorSink,
        ) -> bool {
            let mut is_valid = true;
            let invalid = ['+', '-'];
            let value = if let Some(value) = value.strip_prefix(invalid) {
                let pos = raw.as_str().find(invalid).unwrap();
                error
                    .report_error(
                        ParseError::new("unexpected sign")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[])
                            .with_unexpected(Span::new_unchecked(pos, pos + 1)),
                    );
                is_valid = false;
                value
            } else {
                value
            };
            let valid = radix.validator();
            for (index, c) in value.char_indices() {
                if !valid(c) && c != '_' {
                    let pos = value.offset_from(&raw.as_str()) + index;
                    error
                        .report_error(
                            ParseError::new(radix.invalid_description())
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_unexpected(Span::new_unchecked(pos, pos)),
                        );
                    is_valid = false;
                }
            }
            is_valid
        }
        fn decode_float_or_integer<'i>(
            mut stream: &'i str,
            raw: Raw<'i>,
            kind: ScalarKind,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            output.clear();
            let underscore = "_";
            let stream_start = stream.offset_from(&raw.as_str());
            while !stream.is_empty() {
                let sep_pos = stream.find_slice(underscore);
                let sep_start = sep_pos
                    .clone()
                    .map(|r| r.start)
                    .unwrap_or_else(|| stream.len());
                let part_start = stream.offset_from(&raw.as_str());
                let part_end = part_start + sep_start;
                let part = stream.next_slice(sep_start);
                if sep_pos.is_some() {
                    let _ = stream.next_slice(underscore.len());
                    let mut is_invalid_sep = false;
                    if let Some(last_pos) = sep_start.checked_sub(1) {
                        let last_byte = raw.as_bytes()[part_start + last_pos];
                        if !is_any_digit(last_byte, kind) {
                            is_invalid_sep = true;
                        }
                    } else if part_start == stream_start {
                        is_invalid_sep = true;
                    }
                    if let Some(next_byte) = stream.as_bytes().first() {
                        if !is_any_digit(*next_byte, kind) {
                            is_invalid_sep = true;
                        }
                    } else if stream.is_empty() {
                        is_invalid_sep = true;
                    }
                    if is_invalid_sep {
                        let start = part_end;
                        let end = start + underscore.len();
                        error
                            .report_error(
                                ParseError::new("`_` may only go between digits")
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[])
                                    .with_unexpected(
                                        Span::new_unchecked(end - underscore.len(), end),
                                    ),
                            );
                    }
                }
                if !part.is_empty() && !output.push_str(part) {
                    error
                        .report_error(
                            ParseError::new(ALLOCATION_ERROR)
                                .with_unexpected(Span::new_unchecked(part_start, part_end)),
                        );
                }
            }
            kind
        }
        fn is_any_digit(b: u8, kind: ScalarKind) -> bool {
            if kind == ScalarKind::Float {
                is_dec_integer_digit(b)
            } else {
                is_any_integer_digit(b)
            }
        }
        fn is_any_integer_digit(b: u8) -> bool {
            (b'0'..=b'9', b'a'..=b'f', b'A'..=b'F').contains_token(b)
        }
        fn is_dec_integer_digit(b: u8) -> bool {
            (b'0'..=b'9').contains_token(b)
        }
        fn is_float(raw: &str) -> bool {
            raw.as_bytes().find_slice((b'.', b'e', b'E')).is_some()
        }
        fn decode_as_is<'i>(
            raw: Raw<'i>,
            kind: ScalarKind,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            let kind = decode_as(raw, raw.as_str(), kind, output, error);
            kind
        }
        fn decode_as<'i>(
            raw: Raw<'i>,
            symbol: &'i str,
            kind: ScalarKind,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            output.clear();
            if !output.push_str(symbol) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
            kind
        }
        fn decode_symbol<'i>(
            raw: Raw<'i>,
            symbol: &'static str,
            kind: ScalarKind,
            expected: &'static [Expected],
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            if raw.as_str() != symbol {
                if raw.as_str().contains(" ") {
                    return decode_invalid(raw, output, error);
                } else {
                    error
                        .report_error(
                            ParseError::new(kind.invalid_description())
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(expected)
                                .with_unexpected(Span::new_unchecked(0, raw.len())),
                        );
                }
            }
            decode_as(raw, symbol, kind, output, error)
        }
        fn decode_invalid<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) -> ScalarKind {
            if raw.as_str().ends_with("'''") {
                error
                    .report_error(
                        ParseError::new("missing opening quote")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal(r#"'''"#)])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
            } else if raw.as_str().ends_with(r#"""""#) {
                error
                    .report_error(
                        ParseError::new("missing opening quote")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(
                                &[Expected::Description("multi-line basic string")],
                            )
                            .with_expected(&[Expected::Literal(r#"""""#)])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
            } else if raw.as_str().ends_with("'") {
                error
                    .report_error(
                        ParseError::new("missing opening quote")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal(r#"'"#)])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
            } else if raw.as_str().ends_with(r#"""#) {
                error
                    .report_error(
                        ParseError::new("missing opening quote")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal(r#"""#)])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
            } else {
                error
                    .report_error(
                        ParseError::new("string values must be quoted")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Description("literal string")])
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
            output.clear();
            if !output.push_str(raw.as_str()) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
            ScalarKind::String
        }
    }
    pub(crate) mod string {
        use core::ops::RangeInclusive;
        use winnow::stream::ContainsToken as _;
        use winnow::stream::Offset as _;
        use winnow::stream::Stream as _;
        use crate::ErrorSink;
        use crate::Expected;
        use crate::ParseError;
        use crate::Raw;
        use crate::Span;
        use crate::decoder::StringBuilder;
        use crate::lexer::APOSTROPHE;
        use crate::lexer::ML_BASIC_STRING_DELIM;
        use crate::lexer::ML_LITERAL_STRING_DELIM;
        use crate::lexer::QUOTATION_MARK;
        use crate::lexer::WSCHAR;
        const ALLOCATION_ERROR: &str = "could not allocate for string";
        /// Parse literal string
        ///
        /// ```abnf
        /// ;; Literal String
        ///
        /// literal-string = apostrophe *literal-char apostrophe
        ///
        /// apostrophe = %x27 ; ' apostrophe
        ///
        /// literal-char = %x09 / %x20-26 / %x28-7E / non-ascii
        /// ```
        pub(crate) fn decode_literal_string<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            const INVALID_STRING: &str = "invalid literal string";
            output.clear();
            let s = raw.as_str();
            let s = if let Some(stripped) = s.strip_prefix(APOSTROPHE as char) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("'")])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
                s
            };
            let s = if let Some(stripped) = s.strip_suffix(APOSTROPHE as char) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("'")])
                            .with_unexpected(Span::new_unchecked(raw.len(), raw.len())),
                    );
                s
            };
            for (i, b) in s.as_bytes().iter().enumerate() {
                if !LITERAL_CHAR.contains_token(b) {
                    let offset = (&s.as_bytes()[i..]).offset_from(&raw.as_bytes());
                    error
                        .report_error(
                            ParseError::new(INVALID_STRING)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(
                                    &[
                                        Expected::Description("non-single-quote visible characters"),
                                    ],
                                )
                                .with_unexpected(Span::new_unchecked(offset, offset)),
                        );
                }
            }
            if !output.push_str(s) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
        }
        /// ```abnf
        /// literal-char = %x09 / %x20-26 / %x28-7E / non-ascii
        /// ```
        const LITERAL_CHAR: (
            u8,
            RangeInclusive<u8>,
            RangeInclusive<u8>,
            RangeInclusive<u8>,
        ) = (0x9, 0x20..=0x26, 0x28..=0x7E, NON_ASCII);
        /// ```abnf
        /// non-ascii = %x80-D7FF / %xE000-10FFFF
        /// ```
        /// - ASCII is 0xxxxxxx
        /// - First byte for UTF-8 is 11xxxxxx
        /// - Subsequent UTF-8 bytes are 10xxxxxx
        const NON_ASCII: RangeInclusive<u8> = 0x80..=0xff;
        /// Parse multi-line literal string
        ///
        /// ```abnf
        /// ;; Multiline Literal String
        ///
        /// ml-literal-string = ml-literal-string-delim [ newline ] ml-literal-body
        ///                     ml-literal-string-delim
        /// ml-literal-string-delim = 3apostrophe
        /// ml-literal-body = *mll-content *( mll-quotes 1*mll-content ) [ mll-quotes ]
        ///
        /// mll-content = literal-char / newline
        /// mll-quotes = 1*2apostrophe
        /// ```
        pub(crate) fn decode_ml_literal_string<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            const INVALID_STRING: &str = "invalid multi-line literal string";
            output.clear();
            let s = raw.as_str();
            let s = if let Some(stripped) = s.strip_prefix(ML_LITERAL_STRING_DELIM) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("'")])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
                s
            };
            let s = strip_start_newline(s);
            let s = if let Some(stripped) = s.strip_suffix(ML_LITERAL_STRING_DELIM) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("'")])
                            .with_unexpected(Span::new_unchecked(raw.len(), raw.len())),
                    );
                s.trim_end_matches('\'')
            };
            for (i, b) in s.as_bytes().iter().enumerate() {
                if *b == b'\'' || *b == b'\n' {} else if *b == b'\r' {
                    if s.as_bytes().get(i + 1) != Some(&b'\n') {
                        let offset = (&s.as_bytes()[i + 1..])
                            .offset_from(&raw.as_bytes());
                        error
                            .report_error(
                                ParseError::new(
                                        "carriage return must be followed by newline",
                                    )
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[Expected::Literal("\n")])
                                    .with_unexpected(Span::new_unchecked(offset, offset)),
                            );
                    }
                } else if !LITERAL_CHAR.contains_token(b) {
                    let offset = (&s.as_bytes()[i..]).offset_from(&raw.as_bytes());
                    error
                        .report_error(
                            ParseError::new(INVALID_STRING)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(
                                    &[Expected::Description("non-single-quote characters")],
                                )
                                .with_unexpected(Span::new_unchecked(offset, offset)),
                        );
                }
            }
            if !output.push_str(s) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
        }
        /// Parse basic string
        ///
        /// ```abnf
        /// ;; Basic String
        ///
        /// basic-string = quotation-mark *basic-char quotation-mark
        ///
        /// basic-char = basic-unescaped / escaped
        ///
        /// escaped = escape escape-seq-char
        /// ```
        pub(crate) fn decode_basic_string<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            const INVALID_STRING: &str = "invalid basic string";
            output.clear();
            let s = raw.as_str();
            let s = if let Some(stripped) = s.strip_prefix(QUOTATION_MARK as char) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("\"")])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
                s
            };
            let mut s = if let Some(stripped) = s.strip_suffix(QUOTATION_MARK as char) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("\"")])
                            .with_unexpected(Span::new_unchecked(raw.len(), raw.len())),
                    );
                s
            };
            let segment = basic_unescaped(&mut s);
            if !output.push_str(segment) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
            while !s.is_empty() {
                if s.starts_with("\\") {
                    let _ = s.next_token();
                    let c = escape_seq_char(&mut s, raw, error);
                    if !output.push_char(c) {
                        error
                            .report_error(
                                ParseError::new(ALLOCATION_ERROR)
                                    .with_unexpected(Span::new_unchecked(0, raw.len())),
                            );
                    }
                } else {
                    let invalid = basic_invalid(&mut s);
                    let start = invalid.offset_from(&raw.as_str());
                    let end = start + invalid.len();
                    error
                        .report_error(
                            ParseError::new(INVALID_STRING)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(
                                    &[
                                        Expected::Description(
                                            "non-double-quote visible characters",
                                        ),
                                        Expected::Literal("\\"),
                                    ],
                                )
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                    let _ = output.push_str(invalid);
                }
                let segment = basic_unescaped(&mut s);
                if !output.push_str(segment) {
                    let start = segment.offset_from(&raw.as_str());
                    let end = start + segment.len();
                    error
                        .report_error(
                            ParseError::new(ALLOCATION_ERROR)
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                }
            }
        }
        /// ```abnf
        /// basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
        /// ```
        fn basic_unescaped<'i>(stream: &mut &'i str) -> &'i str {
            let offset = stream
                .as_bytes()
                .offset_for(|b| !BASIC_UNESCAPED.contains_token(b))
                .unwrap_or(stream.len());
            stream.next_slice(offset)
        }
        fn basic_invalid<'i>(stream: &mut &'i str) -> &'i str {
            let offset = stream
                .as_bytes()
                .offset_for(|b| (BASIC_UNESCAPED, ESCAPE).contains_token(b))
                .unwrap_or(stream.len());
            stream.next_slice(offset)
        }
        /// ```abnf
        /// basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
        /// ```
        #[allow(clippy::type_complexity)]
        const BASIC_UNESCAPED: (
            (u8, u8),
            u8,
            RangeInclusive<u8>,
            RangeInclusive<u8>,
            RangeInclusive<u8>,
        ) = (WSCHAR, 0x21, 0x23..=0x5B, 0x5D..=0x7E, NON_ASCII);
        /// ```abnf
        /// escape = %x5C                    ; \
        /// ```
        const ESCAPE: u8 = b'\\';
        /// ```abnf
        /// escape-seq-char =  %x22         ; "    quotation mark  U+0022
        /// escape-seq-char =/ %x5C         ; \    reverse solidus U+005C
        /// escape-seq-char =/ %x62         ; b    backspace       U+0008
        /// escape-seq-char =/ %x65         ; e    escape          U+001B
        /// escape-seq-char =/ %x66         ; f    form feed       U+000C
        /// escape-seq-char =/ %x6E         ; n    line feed       U+000A
        /// escape-seq-char =/ %x72         ; r    carriage return U+000D
        /// escape-seq-char =/ %x74         ; t    tab             U+0009
        /// escape-seq-char =/ %x78 2HEXDIG ; xHH                  U+00HH
        /// escape-seq-char =/ %x75 4HEXDIG ; uHHHH                U+HHHH
        /// escape-seq-char =/ %x55 8HEXDIG ; UHHHHHHHH            U+HHHHHHHH
        /// ```
        fn escape_seq_char(
            stream: &mut &str,
            raw: Raw<'_>,
            error: &mut dyn ErrorSink,
        ) -> char {
            const EXPECTED_ESCAPES: &[Expected] = &[
                Expected::Literal("b"),
                Expected::Literal("e"),
                Expected::Literal("f"),
                Expected::Literal("n"),
                Expected::Literal("r"),
                Expected::Literal("\\"),
                Expected::Literal("\""),
                Expected::Literal("x"),
                Expected::Literal("u"),
                Expected::Literal("U"),
            ];
            let start = stream.checkpoint();
            let Some(id) = stream.next_token() else {
                let offset = stream.offset_from(&raw.as_str());
                error
                    .report_error(
                        ParseError::new("missing escaped value")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(EXPECTED_ESCAPES)
                            .with_unexpected(Span::new_unchecked(offset, offset)),
                    );
                return '\\';
            };
            match id {
                'b' => '\u{8}',
                'e' => '\u{1b}',
                'f' => '\u{c}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'x' => hexescape(stream, 2, raw, error),
                'u' => hexescape(stream, 4, raw, error),
                'U' => hexescape(stream, 8, raw, error),
                '\\' => '\\',
                '"' => '"',
                _ => {
                    stream.reset(&start);
                    let offset = stream.offset_from(&raw.as_str());
                    error
                        .report_error(
                            ParseError::new("missing escaped value")
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(EXPECTED_ESCAPES)
                                .with_unexpected(Span::new_unchecked(offset, offset)),
                        );
                    '\\'
                }
            }
        }
        fn hexescape(
            stream: &mut &str,
            num_digits: usize,
            raw: Raw<'_>,
            error: &mut dyn ErrorSink,
        ) -> char {
            let offset = stream
                .as_bytes()
                .offset_for(|b| !HEXDIG.contains_token(b))
                .unwrap_or_else(|| stream.eof_offset())
                .min(num_digits);
            let value = stream.next_slice(offset);
            if value.len() != num_digits {
                let offset = stream.offset_from(&raw.as_str());
                error
                    .report_error(
                        ParseError::new("too few unicode value digits")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(
                                &[Expected::Description("unicode hexadecimal value")],
                            )
                            .with_unexpected(Span::new_unchecked(offset, offset)),
                    );
                return '�';
            }
            let Some(value) = u32::from_str_radix(value, 16)
                .ok()
                .and_then(char::from_u32) else {
                let offset = value.offset_from(&raw.as_str());
                error
                    .report_error(
                        ParseError::new("invalid value")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(
                                &[Expected::Description("unicode hexadecimal value")],
                            )
                            .with_unexpected(Span::new_unchecked(offset, offset)),
                    );
                return '�';
            };
            value
        }
        /// ```abnf
        /// HEXDIG = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
        /// ```
        const HEXDIG: (RangeInclusive<u8>, RangeInclusive<u8>, RangeInclusive<u8>) = (
            DIGIT,
            b'A'..=b'F',
            b'a'..=b'f',
        );
        /// ```abnf
        /// DIGIT = %x30-39 ; 0-9
        /// ```
        const DIGIT: RangeInclusive<u8> = b'0'..=b'9';
        fn strip_start_newline(s: &str) -> &str {
            s.strip_prefix('\n').or_else(|| s.strip_prefix("\r\n")).unwrap_or(s)
        }
        /// Parse multi-line basic string
        ///
        /// ```abnf
        /// ;; Multiline Basic String
        ///
        /// ml-basic-string = ml-basic-string-delim [ newline ] ml-basic-body
        ///                   ml-basic-string-delim
        /// ml-basic-string-delim = 3quotation-mark
        /// ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
        ///
        /// mlb-content = basic-char / newline / mlb-escaped-nl
        /// mlb-quotes = 1*2quotation-mark
        /// ```
        pub(crate) fn decode_ml_basic_string<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            const INVALID_STRING: &str = "invalid multi-line basic string";
            let s = raw.as_str();
            let s = if let Some(stripped) = s.strip_prefix(ML_BASIC_STRING_DELIM) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("\"")])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
                s
            };
            let s = strip_start_newline(s);
            let mut s = if let Some(stripped) = s.strip_suffix(ML_BASIC_STRING_DELIM) {
                stripped
            } else {
                error
                    .report_error(
                        ParseError::new(INVALID_STRING)
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("\"")])
                            .with_unexpected(Span::new_unchecked(raw.len(), raw.len())),
                    );
                s
            };
            let segment = mlb_unescaped(&mut s);
            if !output.push_str(segment) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
            while !s.is_empty() {
                if s.starts_with("\\") {
                    let _ = s.next_token();
                    if s
                        .as_bytes()
                        .first()
                        .map(|b| (WSCHAR, b'\r', b'\n').contains_token(b))
                        .unwrap_or(false)
                    {
                        mlb_escaped_nl(&mut s, raw, error);
                    } else {
                        let c = escape_seq_char(&mut s, raw, error);
                        if !output.push_char(c) {
                            error
                                .report_error(
                                    ParseError::new(ALLOCATION_ERROR)
                                        .with_unexpected(Span::new_unchecked(0, raw.len())),
                                );
                        }
                    }
                } else if s.starts_with("\r") {
                    let offset = if s.starts_with("\r\n") {
                        "\r\n".len()
                    } else {
                        let start = s.offset_from(&raw.as_str()) + 1;
                        error
                            .report_error(
                                ParseError::new(
                                        "carriage return must be followed by newline",
                                    )
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[Expected::Literal("\n")])
                                    .with_unexpected(Span::new_unchecked(start, start)),
                            );
                        "\r".len()
                    };
                    let newline = s.next_slice(offset);
                    if !output.push_str(newline) {
                        let start = newline.offset_from(&raw.as_str());
                        let end = start + newline.len();
                        error
                            .report_error(
                                ParseError::new(ALLOCATION_ERROR)
                                    .with_unexpected(Span::new_unchecked(start, end)),
                            );
                    }
                } else {
                    let invalid = mlb_invalid(&mut s);
                    let start = invalid.offset_from(&raw.as_str());
                    let end = start + invalid.len();
                    error
                        .report_error(
                            ParseError::new(INVALID_STRING)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(
                                    &[
                                        Expected::Literal("\\"),
                                        Expected::Description("characters"),
                                    ],
                                )
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                    let _ = output.push_str(invalid);
                }
                let segment = mlb_unescaped(&mut s);
                if !output.push_str(segment) {
                    let start = segment.offset_from(&raw.as_str());
                    let end = start + segment.len();
                    error
                        .report_error(
                            ParseError::new(ALLOCATION_ERROR)
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                }
            }
        }
        /// ```abnf
        /// mlb-escaped-nl = escape ws newline *( wschar / newline )
        /// ```
        fn mlb_escaped_nl(stream: &mut &str, raw: Raw<'_>, error: &mut dyn ErrorSink) {
            const INVALID_STRING: &str = "invalid multi-line basic string";
            let ws_offset = stream
                .as_bytes()
                .offset_for(|b| !WSCHAR.contains_token(b))
                .unwrap_or(stream.len());
            stream.next_slice(ws_offset);
            let start = stream.checkpoint();
            match stream.next_token() {
                Some('\n') => {}
                Some('\r') => {
                    if stream.as_bytes().first() == Some(&b'\n') {
                        let _ = stream.next_token();
                    } else {
                        let start = stream.offset_from(&raw.as_str());
                        let end = start;
                        error
                            .report_error(
                                ParseError::new(
                                        "carriage return must be followed by newline",
                                    )
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[Expected::Literal("\n")])
                                    .with_unexpected(Span::new_unchecked(start, end)),
                            );
                    }
                }
                _ => {
                    stream.reset(&start);
                    let start = stream.offset_from(&raw.as_str());
                    let end = start;
                    error
                        .report_error(
                            ParseError::new(INVALID_STRING)
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(&[Expected::Literal("\n")])
                                .with_unexpected(Span::new_unchecked(start, end)),
                        );
                }
            }
            loop {
                let start_offset = stream.offset_from(&raw.as_str());
                let offset = stream
                    .as_bytes()
                    .offset_for(|b| !(WSCHAR, b'\n').contains_token(b))
                    .unwrap_or(stream.len());
                stream.next_slice(offset);
                if stream.starts_with("\r") {
                    let offset = if stream.starts_with("\r\n") {
                        "\r\n".len()
                    } else {
                        let start = stream.offset_from(&raw.as_str()) + 1;
                        error
                            .report_error(
                                ParseError::new(
                                        "carriage return must be followed by newline",
                                    )
                                    .with_context(Span::new_unchecked(0, raw.len()))
                                    .with_expected(&[Expected::Literal("\n")])
                                    .with_unexpected(Span::new_unchecked(start, start)),
                            );
                        "\r".len()
                    };
                    let _ = stream.next_slice(offset);
                }
                let end_offset = stream.offset_from(&raw.as_str());
                if start_offset == end_offset {
                    break;
                }
            }
        }
        /// `mlb-unescaped` extended with `mlb-quotes` and `LF`
        ///
        /// This is a specialization of [`basic_unescaped`] to help with multi-line basic strings
        ///
        /// **warning:** `newline` is not validated
        ///
        /// ```abnf
        /// ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
        ///
        /// mlb-content = basic-cha / newline / mlb-escaped-nl
        /// mlb-quotes = 1*2quotation-mark
        /// ```
        fn mlb_unescaped<'i>(stream: &mut &'i str) -> &'i str {
            let offset = stream
                .as_bytes()
                .offset_for(|b| !(BASIC_UNESCAPED, b'"', b'\n').contains_token(b))
                .unwrap_or(stream.len());
            stream.next_slice(offset)
        }
        fn mlb_invalid<'i>(stream: &mut &'i str) -> &'i str {
            let offset = stream
                .as_bytes()
                .offset_for(|b| {
                    (BASIC_UNESCAPED, b'"', b'\n', ESCAPE, '\r').contains_token(b)
                })
                .unwrap_or(stream.len());
            stream.next_slice(offset)
        }
        /// Parse unquoted key
        ///
        /// ```abnf
        /// unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
        /// ```
        pub(crate) fn decode_unquoted_key<'i>(
            raw: Raw<'i>,
            output: &mut dyn StringBuilder<'i>,
            error: &mut dyn ErrorSink,
        ) {
            let s = raw.as_str();
            if s.is_empty() {
                error
                    .report_error(
                        ParseError::new("unquoted keys cannot be empty")
                            .with_context(Span::new_unchecked(0, s.len()))
                            .with_expected(
                                &[
                                    Expected::Description("letters"),
                                    Expected::Description("numbers"),
                                    Expected::Literal("-"),
                                    Expected::Literal("_"),
                                ],
                            )
                            .with_unexpected(Span::new_unchecked(0, s.len())),
                    );
            }
            let mut span = None;
            for (i, _b) in s
                .as_bytes()
                .iter()
                .enumerate()
                .filter(|(_, b)| !UNQUOTED_CHAR.contains_token(*b))
            {
                if let Some((start, end)) = span {
                    if i == end {
                        span = Some((start, i + 1));
                    } else {
                        error
                            .report_error(
                                ParseError::new("invalid unquoted key")
                                    .with_context(Span::new_unchecked(0, s.len()))
                                    .with_expected(
                                        &[
                                            Expected::Description("letters"),
                                            Expected::Description("numbers"),
                                            Expected::Literal("-"),
                                            Expected::Literal("_"),
                                        ],
                                    )
                                    .with_unexpected(Span::new_unchecked(start, end)),
                            );
                        span = Some((i, i + 1));
                    }
                } else {
                    span = Some((i, i + 1));
                }
            }
            if let Some((start, end)) = span {
                error
                    .report_error(
                        ParseError::new("invalid unquoted key")
                            .with_context(Span::new_unchecked(0, s.len()))
                            .with_expected(
                                &[
                                    Expected::Description("letters"),
                                    Expected::Description("numbers"),
                                    Expected::Literal("-"),
                                    Expected::Literal("_"),
                                ],
                            )
                            .with_unexpected(Span::new_unchecked(start, end)),
                    );
            }
            if !output.push_str(s) {
                error
                    .report_error(
                        ParseError::new(ALLOCATION_ERROR)
                            .with_unexpected(Span::new_unchecked(0, raw.len())),
                    );
            }
        }
        /// ```abnf
        /// unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
        /// ```
        const UNQUOTED_CHAR: (
            RangeInclusive<u8>,
            RangeInclusive<u8>,
            RangeInclusive<u8>,
            u8,
            u8,
        ) = (b'A'..=b'Z', b'a'..=b'z', b'0'..=b'9', b'-', b'_');
    }
    pub(crate) mod ws {
        use core::ops::RangeInclusive;
        use winnow::stream::ContainsToken as _;
        use crate::ErrorSink;
        use crate::Expected;
        use crate::ParseError;
        use crate::Raw;
        use crate::Span;
        use crate::lexer::COMMENT_START_SYMBOL;
        /// Parse comment
        ///
        /// ```abnf
        /// ;; Comment
        ///
        /// comment-start-symbol = %x23 ; #
        /// non-ascii = %x80-D7FF / %xE000-10FFFF
        /// non-eol = %x09 / %x20-7E / non-ascii
        ///
        /// comment = comment-start-symbol *non-eol
        /// ```
        pub(crate) fn decode_comment(raw: Raw<'_>, error: &mut dyn ErrorSink) {
            let s = raw.as_bytes();
            if s.first() != Some(&COMMENT_START_SYMBOL) {
                error
                    .report_error(
                        ParseError::new("missing comment start")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("#")])
                            .with_unexpected(Span::new_unchecked(0, 0)),
                    );
            }
            for (i, b) in s.iter().copied().enumerate() {
                if !NON_EOL.contains_token(b) {
                    error
                        .report_error(
                            ParseError::new("invalid comment character")
                                .with_context(Span::new_unchecked(0, raw.len()))
                                .with_expected(
                                    &[Expected::Description("printable characters")],
                                )
                                .with_unexpected(Span::new_unchecked(i, i)),
                        );
                }
            }
        }
        pub(crate) const NON_ASCII: RangeInclusive<u8> = 0x80..=0xff;
        pub(crate) const NON_EOL: (u8, RangeInclusive<u8>, RangeInclusive<u8>) = (
            0x09,
            0x20..=0x7E,
            NON_ASCII,
        );
        /// Parse newline
        ///
        /// ```abnf
        ///;; Newline
        ///
        /// newline =  %x0A     ; LF
        /// newline =/ %x0D.0A  ; CRLF
        /// ```
        pub(crate) fn decode_newline(raw: Raw<'_>, error: &mut dyn ErrorSink) {
            let s = raw.as_str();
            if s == "\r" {
                error
                    .report_error(
                        ParseError::new("carriage return must be followed by newline")
                            .with_context(Span::new_unchecked(0, raw.len()))
                            .with_expected(&[Expected::Literal("\n")])
                            .with_unexpected(Span::new_unchecked(raw.len(), raw.len())),
                    );
            }
        }
    }
    pub use scalar::IntegerRadix;
    pub use scalar::ScalarKind;
    #[repr(u8)]
    pub enum Encoding {
        LiteralString = crate::lexer::APOSTROPHE,
        BasicString = crate::lexer::QUOTATION_MARK,
        MlLiteralString = 1,
        MlBasicString,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Encoding {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Encoding {}
    #[automatically_derived]
    impl ::core::clone::Clone for Encoding {
        #[inline]
        fn clone(&self) -> Encoding {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Encoding {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Encoding {
        #[inline]
        fn eq(&self, other: &Encoding) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Encoding {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    #[automatically_derived]
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
    impl ::core::cmp::Ord for Encoding {
        #[inline]
        fn cmp(&self, other: &Encoding) -> ::core::cmp::Ordering {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr)
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for Encoding {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            ::core::hash::Hash::hash(&__self_discr, state)
        }
    }
    #[automatically_derived]
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
    impl Encoding {
        pub const fn description(&self) -> &'static str {
            match self {
                Self::LiteralString => {
                    crate::lexer::TokenKind::LiteralString.description()
                }
                Self::BasicString => crate::lexer::TokenKind::BasicString.description(),
                Self::MlLiteralString => {
                    crate::lexer::TokenKind::MlLiteralString.description()
                }
                Self::MlBasicString => {
                    crate::lexer::TokenKind::MlBasicString.description()
                }
            }
        }
    }
    pub trait StringBuilder<'s> {
        fn clear(&mut self);
        #[must_use]
        fn push_str(&mut self, append: &'s str) -> bool;
        #[must_use]
        fn push_char(&mut self, append: char) -> bool;
    }
    impl<'s> StringBuilder<'s> for () {
        fn clear(&mut self) {}
        fn push_str(&mut self, _append: &'s str) -> bool {
            true
        }
        fn push_char(&mut self, _append: char) -> bool {
            true
        }
    }
    impl<'s> StringBuilder<'s> for &'s str {
        fn clear(&mut self) {
            *self = &self[0..0];
        }
        fn push_str(&mut self, append: &'s str) -> bool {
            if self.is_empty() {
                *self = append;
                true
            } else {
                false
            }
        }
        fn push_char(&mut self, _append: char) -> bool {
            false
        }
    }
    impl<'s> StringBuilder<'s> for Cow<'s, str> {
        fn clear(&mut self) {
            match self {
                Cow::Borrowed(s) => {
                    s.clear();
                }
                Cow::Owned(s) => s.clear(),
            }
        }
        fn push_str(&mut self, append: &'s str) -> bool {
            match self {
                Cow::Borrowed(s) => {
                    if !s.push_str(append) {
                        self.to_mut().push_str(append);
                    }
                }
                Cow::Owned(s) => s.push_str(append),
            }
            true
        }
        fn push_char(&mut self, append: char) -> bool {
            self.to_mut().push(append);
            true
        }
    }
    impl<'s> StringBuilder<'s> for String {
        fn clear(&mut self) {
            self.clear();
        }
        fn push_str(&mut self, append: &'s str) -> bool {
            self.push_str(append);
            true
        }
        fn push_char(&mut self, append: char) -> bool {
            self.push(append);
            true
        }
    }
}
pub mod lexer {
    //! Lex TOML tokens
    //!
    //! To get started, see [`Source::lex`][crate::Source::lex]
    mod token {
        //! Lexed TOML tokens
        use super::APOSTROPHE;
        use super::COMMENT_START_SYMBOL;
        use super::QUOTATION_MARK;
        use super::Span;
        use super::WSCHAR;
        use crate::decoder::Encoding;
        /// An unvalidated TOML Token
        pub struct Token {
            pub(super) kind: TokenKind,
            pub(super) span: Span,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Token {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Token {}
        #[automatically_derived]
        impl ::core::clone::Clone for Token {
            #[inline]
            fn clone(&self) -> Token {
                let _: ::core::clone::AssertParamIsClone<TokenKind>;
                let _: ::core::clone::AssertParamIsClone<Span>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Token {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Token {
            #[inline]
            fn eq(&self, other: &Token) -> bool {
                self.kind == other.kind && self.span == other.span
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Token {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<TokenKind>;
                let _: ::core::cmp::AssertParamIsEq<Span>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Token {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Token,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                match ::core::cmp::PartialOrd::partial_cmp(&self.kind, &other.kind) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        ::core::cmp::PartialOrd::partial_cmp(&self.span, &other.span)
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Token {
            #[inline]
            fn cmp(&self, other: &Token) -> ::core::cmp::Ordering {
                match ::core::cmp::Ord::cmp(&self.kind, &other.kind) {
                    ::core::cmp::Ordering::Equal => {
                        ::core::cmp::Ord::cmp(&self.span, &other.span)
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Token {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.kind, state);
                ::core::hash::Hash::hash(&self.span, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Token {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Token",
                    "kind",
                    &self.kind,
                    "span",
                    &&self.span,
                )
            }
        }
        impl Token {
            pub(super) fn new(kind: TokenKind, span: Span) -> Self {
                Self { kind, span }
            }
            #[inline(always)]
            pub fn kind(&self) -> TokenKind {
                self.kind
            }
            #[inline(always)]
            pub fn span(&self) -> Span {
                self.span
            }
        }
        #[repr(u8)]
        pub enum TokenKind {
            /// Either for dotted-key or float
            Dot = b'.',
            /// Key-value separator
            Equals = b'=',
            /// Value separator
            Comma = b',',
            /// Either array or standard-table start
            LeftSquareBracket = b'[',
            /// Either array or standard-table end
            RightSquareBracket = b']',
            /// Inline table start
            LeftCurlyBracket = b'{',
            /// Inline table end
            RightCurlyBracket = b'}',
            Whitespace = WSCHAR.0,
            Comment = COMMENT_START_SYMBOL,
            Newline = b'\n',
            LiteralString = APOSTROPHE,
            BasicString = QUOTATION_MARK,
            MlLiteralString = 1,
            MlBasicString,
            /// Anything else
            Atom,
            Eof,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for TokenKind {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for TokenKind {}
        #[automatically_derived]
        impl ::core::clone::Clone for TokenKind {
            #[inline]
            fn clone(&self) -> TokenKind {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for TokenKind {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for TokenKind {
            #[inline]
            fn eq(&self, other: &TokenKind) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for TokenKind {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {}
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for TokenKind {
            #[inline]
            fn partial_cmp(
                &self,
                other: &TokenKind,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for TokenKind {
            #[inline]
            fn cmp(&self, other: &TokenKind) -> ::core::cmp::Ordering {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for TokenKind {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_discr, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for TokenKind {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        TokenKind::Dot => "Dot",
                        TokenKind::Equals => "Equals",
                        TokenKind::Comma => "Comma",
                        TokenKind::LeftSquareBracket => "LeftSquareBracket",
                        TokenKind::RightSquareBracket => "RightSquareBracket",
                        TokenKind::LeftCurlyBracket => "LeftCurlyBracket",
                        TokenKind::RightCurlyBracket => "RightCurlyBracket",
                        TokenKind::Whitespace => "Whitespace",
                        TokenKind::Comment => "Comment",
                        TokenKind::Newline => "Newline",
                        TokenKind::LiteralString => "LiteralString",
                        TokenKind::BasicString => "BasicString",
                        TokenKind::MlLiteralString => "MlLiteralString",
                        TokenKind::MlBasicString => "MlBasicString",
                        TokenKind::Atom => "Atom",
                        TokenKind::Eof => "Eof",
                    },
                )
            }
        }
        impl TokenKind {
            pub const fn description(&self) -> &'static str {
                match self {
                    Self::Dot => "`.`",
                    Self::Equals => "`=`",
                    Self::Comma => "`,`",
                    Self::LeftSquareBracket => "`[`",
                    Self::RightSquareBracket => "`]`",
                    Self::LeftCurlyBracket => "`{`",
                    Self::RightCurlyBracket => "`}`",
                    Self::Whitespace => "whitespace",
                    Self::Comment => "comment",
                    Self::Newline => "newline",
                    Self::LiteralString => "literal string",
                    Self::BasicString => "basic string",
                    Self::MlLiteralString => "multi-line literal string",
                    Self::MlBasicString => "multi-line basic string",
                    Self::Atom => "token",
                    Self::Eof => "end-of-input",
                }
            }
            pub fn encoding(&self) -> Option<Encoding> {
                match self {
                    Self::LiteralString => Some(Encoding::LiteralString),
                    Self::BasicString => Some(Encoding::BasicString),
                    Self::MlLiteralString => Some(Encoding::MlLiteralString),
                    Self::MlBasicString => Some(Encoding::MlBasicString),
                    Self::Atom
                    | Self::LeftSquareBracket
                    | Self::RightSquareBracket
                    | Self::Dot
                    | Self::Equals
                    | Self::Comma
                    | Self::RightCurlyBracket
                    | Self::LeftCurlyBracket
                    | Self::Whitespace
                    | Self::Newline
                    | Self::Comment
                    | Self::Eof => None,
                }
            }
        }
    }
    use alloc::vec::Vec;
    use winnow::stream::AsBStr as _;
    use winnow::stream::ContainsToken as _;
    use winnow::stream::FindSlice as _;
    use winnow::stream::Location;
    use winnow::stream::Stream as _;
    use crate::Span;
    pub use token::Token;
    pub use token::TokenKind;
    /// Lex TOML [tokens][Token]
    ///
    /// To get started, see [`Source::lex`][crate::Source::lex]
    pub struct Lexer<'i> {
        stream: Stream<'i>,
        eof: bool,
    }
    impl<'i> Lexer<'i> {
        pub(crate) fn new(input: &'i str) -> Self {
            let mut stream = Stream::new(input);
            if input.as_bytes().starts_with(BOM) {
                let offset = BOM.len();
                stream.next_slice(offset);
            }
            Lexer { stream, eof: false }
        }
        pub fn into_vec(self) -> Vec<Token> {
            #![allow(unused_qualifications)]
            let capacity = core::cmp::min(
                self.stream.len(),
                usize::MAX / core::mem::size_of::<Token>(),
            );
            let mut vec = Vec::with_capacity(capacity);
            vec.extend(self);
            vec
        }
    }
    impl Iterator for Lexer<'_> {
        type Item = Token;
        fn next(&mut self) -> Option<Self::Item> {
            let Some(peek_byte) = self.stream.as_bstr().first() else {
                if self.eof {
                    return None;
                } else {
                    self.eof = true;
                    let start = self.stream.current_token_start();
                    let span = Span::new_unchecked(start, start);
                    return Some(Token::new(TokenKind::Eof, span));
                }
            };
            Some(process_token(*peek_byte, &mut self.stream))
        }
    }
    const BOM: &[u8] = b"\xEF\xBB\xBF";
    pub(crate) type Stream<'i> = winnow::stream::LocatingSlice<&'i str>;
    fn process_token(peek_byte: u8, stream: &mut Stream<'_>) -> Token {
        let token = match peek_byte {
            b'.' => lex_ascii_char(stream, TokenKind::Dot),
            b'=' => lex_ascii_char(stream, TokenKind::Equals),
            b',' => lex_ascii_char(stream, TokenKind::Comma),
            b'[' => lex_ascii_char(stream, TokenKind::LeftSquareBracket),
            b']' => lex_ascii_char(stream, TokenKind::RightSquareBracket),
            b'{' => lex_ascii_char(stream, TokenKind::LeftCurlyBracket),
            b'}' => lex_ascii_char(stream, TokenKind::RightCurlyBracket),
            b' ' => lex_whitespace(stream),
            b'\t' => lex_whitespace(stream),
            b'#' => lex_comment(stream),
            b'\r' => lex_crlf(stream),
            b'\n' => lex_ascii_char(stream, TokenKind::Newline),
            b'\'' => {
                if stream.starts_with(ML_LITERAL_STRING_DELIM) {
                    lex_ml_literal_string(stream)
                } else {
                    lex_literal_string(stream)
                }
            }
            b'"' => {
                if stream.starts_with(ML_BASIC_STRING_DELIM) {
                    lex_ml_basic_string(stream)
                } else {
                    lex_basic_string(stream)
                }
            }
            _ => lex_atom(stream),
        };
        token
    }
    /// Process an ASCII character token
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream` must be non-empty
    /// - `stream[0]` must be ASCII
    fn lex_ascii_char(stream: &mut Stream<'_>, kind: TokenKind) -> Token {
        if true {
            if !!stream.is_empty() {
                ::core::panicking::panic("assertion failed: !stream.is_empty()")
            }
        }
        let start = stream.current_token_start();
        let offset = 1;
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(kind, span)
    }
    /// Process Whitespace
    ///
    /// ```abnf
    /// ;; Whitespace
    ///
    /// ws = *wschar
    /// wschar =  %x20  ; Space
    /// wschar =/ %x09  ; Horizontal tab
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream` must be non-empty
    fn lex_whitespace(stream: &mut Stream<'_>) -> Token {
        if true {
            if !!stream.is_empty() {
                ::core::panicking::panic("assertion failed: !stream.is_empty()")
            }
        }
        let start = stream.current_token_start();
        let offset = stream
            .as_bstr()
            .offset_for(|b| !WSCHAR.contains_token(b))
            .unwrap_or(stream.eof_offset());
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::Whitespace, span)
    }
    /// ```abnf
    /// wschar =  %x20  ; Space
    /// wschar =/ %x09  ; Horizontal tab
    /// ```
    pub(crate) const WSCHAR: (u8, u8) = (b' ', b'\t');
    /// Process Comment
    ///
    /// ```abnf
    /// ;; Comment
    ///
    /// comment-start-symbol = %x23 ; #
    /// non-ascii = %x80-D7FF / %xE000-10FFFF
    /// non-eol = %x09 / %x20-7E / non-ascii
    ///
    /// comment = comment-start-symbol *non-eol
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream[0] == b'#'`
    fn lex_comment(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let offset = stream
            .as_bytes()
            .find_slice((b'\r', b'\n'))
            .map(|s| s.start)
            .unwrap_or_else(|| stream.eof_offset());
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::Comment, span)
    }
    /// ```abnf
    /// comment-start-symbol = %x23 ; #
    /// ```
    pub(crate) const COMMENT_START_SYMBOL: u8 = b'#';
    /// Process Newline
    ///
    /// ```abnf
    /// ;; Newline
    ///
    /// newline =  %x0A     ; LF
    /// newline =/ %x0D.0A  ; CRLF
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream[0] == b'\r'`
    fn lex_crlf(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let mut offset = '\r'.len_utf8();
        let has_lf = stream.as_bstr().get(1) == Some(&b'\n');
        if has_lf {
            offset += '\n'.len_utf8();
        }
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::Newline, span)
    }
    /// Process literal string
    ///
    /// ```abnf
    /// ;; Literal String
    ///
    /// literal-string = apostrophe *literal-char apostrophe
    ///
    /// apostrophe = %x27 ; ' apostrophe
    ///
    /// literal-char = %x09 / %x20-26 / %x28-7E / non-ascii
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream[0] == b'\''`
    fn lex_literal_string(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let offset = 1;
        stream.next_slice(offset);
        let offset = match stream.as_bstr().find_slice((APOSTROPHE, b'\n')) {
            Some(span) => {
                if stream.as_bstr()[span.start] == APOSTROPHE {
                    span.end
                } else {
                    span.start
                }
            }
            None => stream.eof_offset(),
        };
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::LiteralString, span)
    }
    /// ```abnf
    /// apostrophe = %x27 ; ' apostrophe
    /// ```
    pub(crate) const APOSTROPHE: u8 = b'\'';
    /// Process multi-line literal string
    ///
    /// ```abnf
    /// ;; Multiline Literal String
    ///
    /// ml-literal-string = ml-literal-string-delim [ newline ] ml-literal-body
    ///                     ml-literal-string-delim
    /// ml-literal-string-delim = 3apostrophe
    /// ml-literal-body = *mll-content *( mll-quotes 1*mll-content ) [ mll-quotes ]
    ///
    /// mll-content = literal-char / newline
    /// mll-quotes = 1*2apostrophe
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream.starts_with(ML_LITERAL_STRING_DELIM)`
    fn lex_ml_literal_string(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let offset = ML_LITERAL_STRING_DELIM.len();
        stream.next_slice(offset);
        let offset = match stream.as_bstr().find_slice(ML_LITERAL_STRING_DELIM) {
            Some(span) => span.end,
            None => stream.eof_offset(),
        };
        stream.next_slice(offset);
        if stream.as_bstr().peek_token() == Some(APOSTROPHE) {
            let offset = 1;
            stream.next_slice(offset);
            if stream.as_bstr().peek_token() == Some(APOSTROPHE) {
                let offset = 1;
                stream.next_slice(offset);
            }
        }
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::MlLiteralString, span)
    }
    /// ```abnf
    /// ml-literal-string-delim = 3apostrophe
    /// ```
    pub(crate) const ML_LITERAL_STRING_DELIM: &str = "'''";
    /// Process basic string
    ///
    /// ```abnf
    /// ;; Basic String
    ///
    /// basic-string = quotation-mark *basic-char quotation-mark
    ///
    /// quotation-mark = %x22            ; "
    ///
    /// basic-char = basic-unescaped / escaped
    /// basic-unescaped = wschar / %x21 / %x23-5B / %x5D-7E / non-ascii
    /// escaped = escape escape-seq-char
    ///
    /// escape = %x5C                   ; \
    /// escape-seq-char =  %x22         ; "    quotation mark  U+0022
    /// escape-seq-char =/ %x5C         ; \    reverse solidus U+005C
    /// escape-seq-char =/ %x62         ; b    backspace       U+0008
    /// escape-seq-char =/ %x65         ; e    escape          U+001B
    /// escape-seq-char =/ %x66         ; f    form feed       U+000C
    /// escape-seq-char =/ %x6E         ; n    line feed       U+000A
    /// escape-seq-char =/ %x72         ; r    carriage return U+000D
    /// escape-seq-char =/ %x74         ; t    tab             U+0009
    /// escape-seq-char =/ %x78 2HEXDIG ; xHH                  U+00HH
    /// escape-seq-char =/ %x75 4HEXDIG ; uHHHH                U+HHHH
    /// escape-seq-char =/ %x55 8HEXDIG ; UHHHHHHHH            U+HHHHHHHH
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream[0] == b'"'`
    fn lex_basic_string(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let offset = 1;
        stream.next_slice(offset);
        loop {
            match stream.as_bstr().find_slice((QUOTATION_MARK, ESCAPE, b'\n')) {
                Some(span) => {
                    let found = stream.as_bstr()[span.start];
                    if found == QUOTATION_MARK {
                        let offset = span.end;
                        stream.next_slice(offset);
                        break;
                    } else if found == ESCAPE {
                        let offset = span.end;
                        stream.next_slice(offset);
                        let peek = stream.as_bstr().peek_token();
                        match peek {
                            Some(ESCAPE) | Some(QUOTATION_MARK) => {
                                let offset = 1;
                                stream.next_slice(offset);
                            }
                            _ => {}
                        }
                        continue;
                    } else if found == b'\n' {
                        let offset = span.start;
                        stream.next_slice(offset);
                        break;
                    } else {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!("found `{0}`", found),
                                ),
                            );
                        };
                    }
                }
                None => {
                    stream.finish();
                    break;
                }
            }
        }
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::BasicString, span)
    }
    /// ```abnf
    /// quotation-mark = %x22            ; "
    /// ```
    pub(crate) const QUOTATION_MARK: u8 = b'"';
    /// ```abnf
    /// escape = %x5C                   ; \
    /// ```
    pub(crate) const ESCAPE: u8 = b'\\';
    /// Process multi-line basic string
    ///
    /// ```abnf
    /// ;; Multiline Basic String
    ///
    /// ml-basic-string = ml-basic-string-delim [ newline ] ml-basic-body
    ///                   ml-basic-string-delim
    /// ml-basic-string-delim = 3quotation-mark
    /// ml-basic-body = *mlb-content *( mlb-quotes 1*mlb-content ) [ mlb-quotes ]
    ///
    /// mlb-content = basic-char / newline / mlb-escaped-nl
    /// mlb-quotes = 1*2quotation-mark
    /// mlb-escaped-nl = escape ws newline *( wschar / newline )
    /// ```
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream.starts_with(ML_BASIC_STRING_DELIM)`
    fn lex_ml_basic_string(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        let offset = ML_BASIC_STRING_DELIM.len();
        stream.next_slice(offset);
        loop {
            match stream.as_bstr().find_slice((ML_BASIC_STRING_DELIM, "\\")) {
                Some(span) => {
                    let found = stream.as_bstr()[span.start];
                    if found == QUOTATION_MARK {
                        let offset = span.end;
                        stream.next_slice(offset);
                        break;
                    } else if found == ESCAPE {
                        let offset = span.end;
                        stream.next_slice(offset);
                        let peek = stream.as_bstr().peek_token();
                        match peek {
                            Some(ESCAPE) | Some(QUOTATION_MARK) => {
                                let offset = 1;
                                stream.next_slice(offset);
                            }
                            _ => {}
                        }
                        continue;
                    } else {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!("found `{0}`", found),
                                ),
                            );
                        };
                    }
                }
                None => {
                    stream.finish();
                    break;
                }
            }
        }
        if stream.as_bstr().peek_token() == Some(QUOTATION_MARK) {
            let offset = 1;
            stream.next_slice(offset);
            if stream.as_bstr().peek_token() == Some(QUOTATION_MARK) {
                let offset = 1;
                stream.next_slice(offset);
            }
        }
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::MlBasicString, span)
    }
    /// ```abnf
    /// ml-basic-string-delim = 3quotation-mark
    /// ```
    pub(crate) const ML_BASIC_STRING_DELIM: &str = "\"\"\"";
    /// Process Atom
    ///
    /// This is everything else
    ///
    /// # Safety
    ///
    /// - `stream` must be UTF-8
    /// - `stream` must be non-empty
    fn lex_atom(stream: &mut Stream<'_>) -> Token {
        let start = stream.current_token_start();
        const TOKEN_START: &[u8] = b".=,[]{} \t#\r\n";
        let offset = stream
            .as_bstr()
            .offset_for(|b| TOKEN_START.contains_token(b))
            .unwrap_or_else(|| stream.eof_offset());
        stream.next_slice(offset);
        let end = stream.previous_token_end();
        let span = Span::new_unchecked(start, end);
        Token::new(TokenKind::Atom, span)
    }
}
pub mod parser {
    //! A TOML push [parser][parse_document]
    //!
    //! This takes TOML [tokens][crate::lexer::Token] and [emits][EventReceiver] [events][Event].
    mod document {
        use winnow::stream::Offset as _;
        use winnow::stream::Stream as _;
        use winnow::stream::TokenSlice;
        use super::EventReceiver;
        use crate::ErrorSink;
        use crate::Expected;
        use crate::ParseError;
        use crate::decoder::Encoding;
        use crate::lexer::Token;
        use crate::lexer::TokenKind;
        /// Parse lexed tokens into [`Event`][super::Event]s
        pub fn parse_document(
            tokens: &[Token],
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut tokens = TokenSlice::new(tokens);
            document(&mut tokens, receiver, error);
            eof(&mut tokens, receiver, error);
        }
        /// Parse lexed tokens into [`Event`][super::Event]s
        pub fn parse_key(
            tokens: &[Token],
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut tokens = TokenSlice::new(tokens);
            key(&mut tokens, "invalid key", receiver, error);
            eof(&mut tokens, receiver, error);
        }
        /// Parse lexed tokens into [`Event`][super::Event]s
        pub fn parse_simple_key(
            tokens: &[Token],
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut tokens = TokenSlice::new(tokens);
            simple_key(&mut tokens, "invalid key", receiver, error);
            eof(&mut tokens, receiver, error);
        }
        /// Parse lexed tokens into [`Event`][super::Event]s
        pub fn parse_value(
            tokens: &[Token],
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut tokens = TokenSlice::new(tokens);
            value(&mut tokens, receiver, error);
            eof(&mut tokens, receiver, error);
        }
        type Stream<'i> = TokenSlice<'i, Token>;
        /// Parse a TOML Document
        ///
        /// Only the order of [`Event`][super::Event]s is validated and not [`Event`][super::Event] content nor semantics like duplicate
        /// keys.
        ///
        /// ```abnf
        /// toml = expression *( newline expression )
        ///
        /// expression =  ws [ comment ]
        /// expression =/ ws keyval ws [ comment ]
        /// expression =/ ws table ws [ comment ]
        ///
        /// ;; Key-Value pairs
        ///
        /// keyval = key keyval-sep val
        /// key = simple-key / dotted-key
        /// val = string / boolean / array / inline-table / date-time / float / integer
        ///
        /// simple-key = quoted-key / unquoted-key
        ///
        /// ;; Quoted and dotted key
        ///
        /// quoted-key = basic-string / literal-string
        /// dotted-key = simple-key 1*( dot-sep simple-key )
        ///
        /// dot-sep   = ws %x2E ws  ; . Period
        /// keyval-sep = ws %x3D ws ; =
        ///
        /// ;; Array
        ///
        /// array = array-open [ array-values ] ws-comment-newline array-close
        ///
        /// array-open =  %x5B ; [
        /// array-close = %x5D ; ]
        ///
        /// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
        /// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
        ///
        /// array-sep = %x2C  ; , Comma
        ///
        /// ;; Table
        ///
        /// table = std-table / array-table
        ///
        /// ;; Standard Table
        ///
        /// std-table = std-table-open key std-table-close
        ///
        /// ;; Inline Table
        ///
        /// inline-table = inline-table-open [ inline-table-keyvals ] ws-comment-newline inline-table-close
        ///
        /// inline-table-keyvals =  ws-comment-newline keyval ws-comment-newline inline-table-sep inline-table-keyvals
        /// inline-table-keyvals =/ ws-comment-newline keyval ws-comment-newline [ inline-table-sep ]
        ///
        /// ;; Array Table
        ///
        /// array-table = array-table-open key array-table-close
        /// ```
        fn document(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            while let Some(current_token) = tokens.next_token() {
                match current_token.kind() {
                    TokenKind::LeftSquareBracket => {
                        on_table(tokens, current_token, receiver, error)
                    }
                    TokenKind::RightSquareBracket => {
                        on_missing_std_table(tokens, current_token, receiver, error);
                    }
                    TokenKind::LiteralString => {
                        on_expression_key(
                            tokens,
                            current_token,
                            Some(Encoding::LiteralString),
                            receiver,
                            error,
                        )
                    }
                    TokenKind::BasicString => {
                        on_expression_key(
                            tokens,
                            current_token,
                            Some(Encoding::BasicString),
                            receiver,
                            error,
                        )
                    }
                    TokenKind::MlLiteralString => {
                        on_expression_key(
                            tokens,
                            current_token,
                            Some(Encoding::MlLiteralString),
                            receiver,
                            error,
                        )
                    }
                    TokenKind::MlBasicString => {
                        on_expression_key(
                            tokens,
                            current_token,
                            Some(Encoding::MlBasicString),
                            receiver,
                            error,
                        )
                    }
                    TokenKind::Atom => {
                        on_expression_key(tokens, current_token, None, receiver, error)
                    }
                    TokenKind::Equals => {
                        let fake_key = current_token.span().before();
                        let encoding = None;
                        receiver.simple_key(fake_key, encoding, error);
                        on_expression_key_val_sep(
                            tokens,
                            current_token,
                            receiver,
                            error,
                        );
                    }
                    TokenKind::Dot => {
                        on_expression_dot(tokens, current_token, receiver, error);
                    }
                    TokenKind::Comma
                    | TokenKind::RightCurlyBracket
                    | TokenKind::LeftCurlyBracket => {
                        on_missing_expression_key(
                            tokens,
                            current_token,
                            receiver,
                            error,
                        );
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error)
                    }
                    TokenKind::Newline => receiver.newline(current_token.span(), error),
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error)
                    }
                    TokenKind::Eof => {
                        break;
                    }
                }
            }
        }
        /// Start a table from the open token
        ///
        /// This eats to EOL
        ///
        /// ```abnf
        /// ;; Table
        ///
        /// table = std-table / array-table
        ///
        /// ;; Standard Table
        ///
        /// std-table = std-table-open key std-table-close
        ///
        /// ;; Array Table
        ///
        /// array-table = array-table-open key array-table-close
        /// ```
        fn on_table(
            tokens: &mut Stream<'_>,
            open_token: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let is_array_table = if let Some(second_open_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::LeftSquareBracket => true,
                        _ => false,
                    }
                },
            ) {
                let span = open_token.span().append(second_open_token.span());
                receiver.array_table_open(span, error);
                true
            } else {
                let span = open_token.span();
                receiver.std_table_open(span, error);
                false
            };
            opt_whitespace(tokens, receiver, error);
            let valid_key = key(tokens, "invalid table", receiver, error);
            opt_whitespace(tokens, receiver, error);
            let mut success = false;
            if let Some(close_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::RightSquareBracket => true,
                        _ => false,
                    }
                },
            ) {
                if is_array_table {
                    if let Some(second_close_token) = next_token_if(
                        tokens,
                        |k| {
                            #[allow(non_exhaustive_omitted_patterns)]
                            match k {
                                TokenKind::RightSquareBracket => true,
                                _ => false,
                            }
                        },
                    ) {
                        let span = close_token.span().append(second_close_token.span());
                        receiver.array_table_close(span, error);
                        success = true;
                    } else {
                        let context = open_token.span().append(close_token.span());
                        error
                            .report_error(
                                ParseError::new("unclosed array table")
                                    .with_context(context)
                                    .with_expected(&[Expected::Literal("]")])
                                    .with_unexpected(close_token.span().after()),
                            );
                    }
                } else {
                    receiver.std_table_close(close_token.span(), error);
                    success = true;
                }
            } else if valid_key {
                let last_key_token = tokens
                    .previous_tokens()
                    .find(|t| t.kind() != TokenKind::Whitespace)
                    .unwrap_or(open_token);
                let context = open_token.span().append(last_key_token.span());
                if is_array_table {
                    error
                        .report_error(
                            ParseError::new("unclosed array table")
                                .with_context(context)
                                .with_expected(&[Expected::Literal("]]")])
                                .with_unexpected(last_key_token.span().after()),
                        );
                } else {
                    error
                        .report_error(
                            ParseError::new("unclosed table")
                                .with_context(context)
                                .with_expected(&[Expected::Literal("]")])
                                .with_unexpected(last_key_token.span().after()),
                        );
                }
            }
            if success {
                ws_comment_newline(tokens, receiver, error);
            } else {
                ignore_to_newline(tokens, receiver, error);
            }
        }
        /// Parse a TOML key
        ///
        /// ```abnf
        /// ;; Key-Value pairs
        ///
        /// key = simple-key / dotted-key
        ///
        /// simple-key = quoted-key / unquoted-key
        ///
        /// ;; Quoted and dotted key
        ///
        /// quoted-key = basic-string / literal-string
        /// dotted-key = simple-key 1*( dot-sep simple-key )
        ///
        /// dot-sep   = ws %x2E ws  ; . Period
        /// ```
        fn key(
            tokens: &mut Stream<'_>,
            invalid_description: &'static str,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) -> bool {
            while let Some(current_token) = tokens.next_token() {
                let encoding = match current_token.kind() {
                    TokenKind::RightSquareBracket
                    | TokenKind::Comment
                    | TokenKind::Equals
                    | TokenKind::Comma
                    | TokenKind::LeftSquareBracket
                    | TokenKind::LeftCurlyBracket
                    | TokenKind::RightCurlyBracket
                    | TokenKind::Newline
                    | TokenKind::Eof => {
                        let fake_key = current_token.span().before();
                        let encoding = None;
                        receiver.simple_key(fake_key, encoding, error);
                        seek(tokens, -1);
                        return false;
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                        continue;
                    }
                    TokenKind::Dot => {
                        let fake_key = current_token.span().before();
                        let encoding = None;
                        receiver.simple_key(fake_key, encoding, error);
                        receiver.key_sep(current_token.span(), error);
                        continue;
                    }
                    TokenKind::LiteralString => Some(Encoding::LiteralString),
                    TokenKind::BasicString => Some(Encoding::BasicString),
                    TokenKind::MlLiteralString => Some(Encoding::MlLiteralString),
                    TokenKind::MlBasicString => Some(Encoding::MlBasicString),
                    TokenKind::Atom => None,
                };
                receiver.simple_key(current_token.span(), encoding, error);
                return opt_dot_keys(tokens, receiver, error);
            }
            let previous_span = tokens
                .previous_tokens()
                .find(|t| {
                    !#[allow(non_exhaustive_omitted_patterns)]
                    match t.kind() {
                        TokenKind::Whitespace
                        | TokenKind::Comment
                        | TokenKind::Newline
                        | TokenKind::Eof => true,
                        _ => false,
                    }
                })
                .map(|t| t.span())
                .unwrap_or_default();
            error
                .report_error(
                    ParseError::new(invalid_description)
                        .with_context(previous_span)
                        .with_expected(&[Expected::Description("key")])
                        .with_unexpected(previous_span.after()),
                );
            false
        }
        /// Start an expression from a key compatible token  type
        ///
        /// ```abnf
        /// expression =  ws [ comment ]
        /// expression =/ ws keyval ws [ comment ]
        /// expression =/ ws table ws [ comment ]
        ///
        /// ;; Key-Value pairs
        ///
        /// keyval = key keyval-sep val
        /// ```
        fn on_expression_key<'i>(
            tokens: &mut Stream<'i>,
            key_token: &'i Token,
            encoding: Option<Encoding>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            receiver.simple_key(key_token.span(), encoding, error);
            opt_dot_keys(tokens, receiver, error);
            opt_whitespace(tokens, receiver, error);
            let Some(eq_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::Equals => true,
                        _ => false,
                    }
                },
            ) else {
                if let Some(peek_token) = tokens.first() {
                    let span = peek_token.span().before();
                    error
                        .report_error(
                            ParseError::new("key with no value")
                                .with_context(span)
                                .with_expected(&[Expected::Literal("=")])
                                .with_unexpected(span),
                        );
                }
                ignore_to_newline(tokens, receiver, error);
                return;
            };
            on_expression_key_val_sep(tokens, eq_token, receiver, error);
        }
        fn on_expression_dot<'i>(
            tokens: &mut Stream<'i>,
            dot_token: &'i Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            receiver.simple_key(dot_token.span().before(), None, error);
            seek(tokens, -1);
            opt_dot_keys(tokens, receiver, error);
            opt_whitespace(tokens, receiver, error);
            let Some(eq_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::Equals => true,
                        _ => false,
                    }
                },
            ) else {
                if let Some(peek_token) = tokens.first() {
                    let span = peek_token.span().before();
                    error
                        .report_error(
                            ParseError::new("missing value for key")
                                .with_context(span)
                                .with_expected(&[Expected::Literal("=")])
                                .with_unexpected(span),
                        );
                }
                ignore_to_newline(tokens, receiver, error);
                return;
            };
            on_expression_key_val_sep(tokens, eq_token, receiver, error);
        }
        fn on_expression_key_val_sep<'i>(
            tokens: &mut Stream<'i>,
            eq_token: &'i Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            receiver.key_val_sep(eq_token.span(), error);
            opt_whitespace(tokens, receiver, error);
            value(tokens, receiver, error);
            ws_comment_newline(tokens, receiver, error);
        }
        /// Parse a TOML simple key
        ///
        /// ```abnf
        /// ;; Key-Value pairs
        ///
        /// simple-key = quoted-key / unquoted-key
        ///
        /// ;; Quoted and dotted key
        ///
        /// quoted-key = basic-string / literal-string
        /// ```
        fn simple_key(
            tokens: &mut Stream<'_>,
            invalid_description: &'static str,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let Some(current_token) = tokens.next_token() else {
                let previous_span = tokens
                    .previous_tokens()
                    .find(|t| {
                        !#[allow(non_exhaustive_omitted_patterns)]
                        match t.kind() {
                            TokenKind::Whitespace
                            | TokenKind::Comment
                            | TokenKind::Newline
                            | TokenKind::Eof => true,
                            _ => false,
                        }
                    })
                    .map(|t| t.span())
                    .unwrap_or_default();
                error
                    .report_error(
                        ParseError::new(invalid_description)
                            .with_context(previous_span)
                            .with_expected(&[Expected::Description("key")])
                            .with_unexpected(previous_span.after()),
                    );
                return;
            };
            const EXPECTED_KEYS: [Expected; 3] = [
                Expected::Description(Encoding::LiteralString.description()),
                Expected::Description(Encoding::BasicString.description()),
                Expected::Description(UNQUOTED_STRING),
            ];
            let kind = match current_token.kind() {
                TokenKind::Dot
                | TokenKind::RightSquareBracket
                | TokenKind::Comment
                | TokenKind::Equals
                | TokenKind::Comma
                | TokenKind::LeftSquareBracket
                | TokenKind::LeftCurlyBracket
                | TokenKind::RightCurlyBracket
                | TokenKind::Newline
                | TokenKind::Eof
                | TokenKind::Whitespace => {
                    on_missing_key(
                        tokens,
                        current_token,
                        invalid_description,
                        receiver,
                        error,
                    );
                    return;
                }
                TokenKind::LiteralString => Some(Encoding::LiteralString),
                TokenKind::BasicString => Some(Encoding::BasicString),
                TokenKind::MlLiteralString => {
                    error
                        .report_error(
                            ParseError::new(invalid_description)
                                .with_context(current_token.span())
                                .with_expected(&EXPECTED_KEYS)
                                .with_unexpected(current_token.span()),
                        );
                    Some(Encoding::MlLiteralString)
                }
                TokenKind::MlBasicString => {
                    error
                        .report_error(
                            ParseError::new(invalid_description)
                                .with_context(current_token.span())
                                .with_expected(&EXPECTED_KEYS)
                                .with_unexpected(current_token.span()),
                        );
                    Some(Encoding::MlBasicString)
                }
                TokenKind::Atom => None,
            };
            receiver.simple_key(current_token.span(), kind, error);
        }
        /// Start a key from the first key compatible token type
        ///
        /// Returns the last key on success
        ///
        /// This will swallow the trailing [`TokenKind::Whitespace`]
        ///
        /// ```abnf
        /// key = simple-key / dotted-key
        ///
        /// simple-key = quoted-key / unquoted-key
        ///
        /// ;; Quoted and dotted key
        ///
        /// quoted-key = basic-string / literal-string
        /// dotted-key = simple-key 1*( dot-sep simple-key )
        ///
        /// dot-sep   = ws %x2E ws  ; . Period
        /// ```
        fn opt_dot_keys(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) -> bool {
            opt_whitespace(tokens, receiver, error);
            let mut success = true;
            'dot: while let Some(dot_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::Dot => true,
                        _ => false,
                    }
                },
            ) {
                receiver.key_sep(dot_token.span(), error);
                while let Some(current_token) = tokens.next_token() {
                    let kind = match current_token.kind() {
                        TokenKind::Equals
                        | TokenKind::Comma
                        | TokenKind::LeftSquareBracket
                        | TokenKind::RightSquareBracket
                        | TokenKind::LeftCurlyBracket
                        | TokenKind::RightCurlyBracket
                        | TokenKind::Comment
                        | TokenKind::Newline
                        | TokenKind::Eof => {
                            let fake_key = current_token.span().before();
                            let encoding = None;
                            receiver.simple_key(fake_key, encoding, error);
                            seek(tokens, -1);
                            success = false;
                            break 'dot;
                        }
                        TokenKind::Whitespace => {
                            receiver.whitespace(current_token.span(), error);
                            continue;
                        }
                        TokenKind::Dot => {
                            let fake_key = current_token.span().before();
                            let encoding = None;
                            receiver.simple_key(fake_key, encoding, error);
                            receiver.key_sep(current_token.span(), error);
                            continue;
                        }
                        TokenKind::LiteralString => Some(Encoding::LiteralString),
                        TokenKind::BasicString => Some(Encoding::BasicString),
                        TokenKind::MlLiteralString => Some(Encoding::MlLiteralString),
                        TokenKind::MlBasicString => Some(Encoding::MlBasicString),
                        TokenKind::Atom => None,
                    };
                    receiver.simple_key(current_token.span(), kind, error);
                    opt_whitespace(tokens, receiver, error);
                    continue 'dot;
                }
                let fake_key = dot_token.span().after();
                let encoding = None;
                receiver.simple_key(fake_key, encoding, error);
            }
            success
        }
        /// Parse a value
        ///
        /// ```abnf
        /// val = string / boolean / array / inline-table / date-time / float / integer
        /// ```
        fn value(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let current_token = loop {
                let Some(current_token) = tokens.next_token() else {
                    let previous_span = tokens
                        .previous_tokens()
                        .find(|t| {
                            !#[allow(non_exhaustive_omitted_patterns)]
                            match t.kind() {
                                TokenKind::Whitespace
                                | TokenKind::Comment
                                | TokenKind::Newline
                                | TokenKind::Eof => true,
                                _ => false,
                            }
                        })
                        .map(|t| t.span())
                        .unwrap_or_default();
                    error
                        .report_error(
                            ParseError::new("missing value")
                                .with_context(previous_span)
                                .with_expected(&[Expected::Description("value")])
                                .with_unexpected(previous_span.after()),
                        );
                    return;
                };
                if current_token.kind() != TokenKind::Equals {
                    break current_token;
                }
                error
                    .report_error(
                        ParseError::new("extra `=`")
                            .with_context(current_token.span())
                            .with_expected(&[])
                            .with_unexpected(current_token.span()),
                    );
                receiver.error(current_token.span(), error);
            };
            match current_token.kind() {
                TokenKind::Comment
                | TokenKind::Comma
                | TokenKind::Newline
                | TokenKind::Eof
                | TokenKind::Whitespace => {
                    let fake_key = current_token.span().before();
                    let encoding = None;
                    receiver.scalar(fake_key, encoding, error);
                    seek(tokens, -1);
                }
                TokenKind::Equals => {
                    ::core::panicking::panic("internal error: entered unreachable code")
                }
                TokenKind::LeftCurlyBracket => {
                    on_inline_table_open(tokens, current_token, receiver, error);
                }
                TokenKind::RightCurlyBracket => {
                    error
                        .report_error(
                            ParseError::new("missing inline table opening")
                                .with_context(current_token.span())
                                .with_expected(&[Expected::Literal("{")])
                                .with_unexpected(current_token.span().before()),
                        );
                    let _ = receiver
                        .inline_table_open(current_token.span().before(), error);
                    receiver.inline_table_close(current_token.span(), error);
                }
                TokenKind::LeftSquareBracket => {
                    on_array_open(tokens, current_token, receiver, error);
                }
                TokenKind::RightSquareBracket => {
                    error
                        .report_error(
                            ParseError::new("missing array opening")
                                .with_context(current_token.span())
                                .with_expected(&[Expected::Literal("[")])
                                .with_unexpected(current_token.span().before()),
                        );
                    let _ = receiver.array_open(current_token.span().before(), error);
                    receiver.array_close(current_token.span(), error);
                }
                TokenKind::LiteralString
                | TokenKind::BasicString
                | TokenKind::MlLiteralString
                | TokenKind::MlBasicString
                | TokenKind::Dot
                | TokenKind::Atom => {
                    on_scalar(tokens, current_token, receiver, error);
                }
            }
        }
        /// Parse a scalar value
        ///
        /// ```abnf
        /// val = string / boolean / array / inline-table / date-time / float / integer
        /// ```
        fn on_scalar(
            tokens: &mut Stream<'_>,
            scalar: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut span = scalar.span();
            let encoding = match scalar.kind() {
                TokenKind::Comment
                | TokenKind::Comma
                | TokenKind::Newline
                | TokenKind::Eof
                | TokenKind::Whitespace
                | TokenKind::Equals
                | TokenKind::LeftCurlyBracket
                | TokenKind::RightCurlyBracket
                | TokenKind::LeftSquareBracket
                | TokenKind::RightSquareBracket => {
                    ::core::panicking::panic("internal error: entered unreachable code")
                }
                TokenKind::LiteralString => Some(Encoding::LiteralString),
                TokenKind::BasicString => Some(Encoding::BasicString),
                TokenKind::MlLiteralString => Some(Encoding::MlLiteralString),
                TokenKind::MlBasicString => Some(Encoding::MlBasicString),
                TokenKind::Dot | TokenKind::Atom => {
                    while let Some(next_token) = tokens.first() {
                        match next_token.kind() {
                            TokenKind::Comment
                            | TokenKind::Comma
                            | TokenKind::Newline
                            | TokenKind::Eof
                            | TokenKind::Equals
                            | TokenKind::LeftCurlyBracket
                            | TokenKind::RightCurlyBracket
                            | TokenKind::LeftSquareBracket
                            | TokenKind::RightSquareBracket
                            | TokenKind::LiteralString
                            | TokenKind::BasicString
                            | TokenKind::MlLiteralString
                            | TokenKind::MlBasicString => {
                                break;
                            }
                            TokenKind::Whitespace => {
                                if let Some(second) = tokens.get(1) {
                                    if second.kind() == TokenKind::Atom {
                                        span = span.append(second.span());
                                        let _ = tokens.next_slice(2);
                                        continue;
                                    }
                                }
                                break;
                            }
                            TokenKind::Dot | TokenKind::Atom => {
                                span = span.append(next_token.span());
                                let _ = tokens.next_token();
                            }
                        }
                    }
                    None
                }
            };
            receiver.scalar(span, encoding, error);
        }
        /// Parse an array
        ///
        /// ```abnf
        /// ;; Array
        ///
        /// array = array-open [ array-values ] ws-comment-newline array-close
        ///
        /// array-values =  ws-comment-newline val ws-comment-newline array-sep array-values
        /// array-values =/ ws-comment-newline val ws-comment-newline [ array-sep ]
        /// ```
        fn on_array_open(
            tokens: &mut Stream<'_>,
            array_open: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            if !receiver.array_open(array_open.span(), error) {
                ignore_to_value_close(
                    tokens,
                    TokenKind::RightSquareBracket,
                    receiver,
                    error,
                );
                return;
            }
            enum State {
                NeedsValue,
                NeedsComma,
            }
            let mut state = State::NeedsValue;
            while let Some(current_token) = tokens.next_token() {
                match current_token.kind() {
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error);
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                    }
                    TokenKind::Newline => {
                        receiver.newline(current_token.span(), error);
                    }
                    TokenKind::Eof => {
                        break;
                    }
                    TokenKind::Comma => {
                        match state {
                            State::NeedsValue => {
                                error
                                    .report_error(
                                        ParseError::new("extra comma in array")
                                            .with_context(array_open.span())
                                            .with_expected(&[Expected::Description("value")])
                                            .with_unexpected(current_token.span()),
                                    );
                                receiver.error(current_token.span(), error);
                            }
                            State::NeedsComma => {
                                receiver.value_sep(current_token.span(), error);
                                state = State::NeedsValue;
                            }
                        }
                    }
                    TokenKind::Equals => {
                        error
                            .report_error(
                                ParseError::new("unexpected `=` in array")
                                    .with_context(array_open.span())
                                    .with_expected(
                                        &[Expected::Description("value"), Expected::Literal("]")],
                                    )
                                    .with_unexpected(current_token.span()),
                            );
                        receiver.error(current_token.span(), error);
                    }
                    TokenKind::LeftCurlyBracket => {
                        if !#[allow(non_exhaustive_omitted_patterns)]
                        match state {
                            State::NeedsValue => true,
                            _ => false,
                        } {
                            error
                                .report_error(
                                    ParseError::new("missing comma between array elements")
                                        .with_context(array_open.span())
                                        .with_expected(&[Expected::Literal(",")])
                                        .with_unexpected(current_token.span().before()),
                                );
                            receiver.value_sep(current_token.span().before(), error);
                        }
                        on_inline_table_open(tokens, current_token, receiver, error);
                        state = State::NeedsComma;
                    }
                    TokenKind::RightCurlyBracket => {
                        if !#[allow(non_exhaustive_omitted_patterns)]
                        match state {
                            State::NeedsValue => true,
                            _ => false,
                        } {
                            error
                                .report_error(
                                    ParseError::new("missing comma between array elements")
                                        .with_context(array_open.span())
                                        .with_expected(&[Expected::Literal(",")])
                                        .with_unexpected(current_token.span().before()),
                                );
                            receiver.value_sep(current_token.span().before(), error);
                        }
                        error
                            .report_error(
                                ParseError::new("missing inline table opening")
                                    .with_context(current_token.span())
                                    .with_expected(&[Expected::Literal("{")])
                                    .with_unexpected(current_token.span().before()),
                            );
                        let _ = receiver
                            .inline_table_open(current_token.span().before(), error);
                        receiver.inline_table_close(current_token.span(), error);
                        state = State::NeedsComma;
                    }
                    TokenKind::LeftSquareBracket => {
                        if !#[allow(non_exhaustive_omitted_patterns)]
                        match state {
                            State::NeedsValue => true,
                            _ => false,
                        } {
                            error
                                .report_error(
                                    ParseError::new("missing comma between array elements")
                                        .with_context(array_open.span())
                                        .with_expected(&[Expected::Literal(",")])
                                        .with_unexpected(current_token.span().before()),
                                );
                            receiver.value_sep(current_token.span().before(), error);
                        }
                        on_array_open(tokens, current_token, receiver, error);
                        state = State::NeedsComma;
                    }
                    TokenKind::RightSquareBracket => {
                        receiver.array_close(current_token.span(), error);
                        return;
                    }
                    TokenKind::LiteralString
                    | TokenKind::BasicString
                    | TokenKind::MlLiteralString
                    | TokenKind::MlBasicString
                    | TokenKind::Dot
                    | TokenKind::Atom => {
                        if !#[allow(non_exhaustive_omitted_patterns)]
                        match state {
                            State::NeedsValue => true,
                            _ => false,
                        } {
                            error
                                .report_error(
                                    ParseError::new("missing comma between array elements")
                                        .with_context(array_open.span())
                                        .with_expected(&[Expected::Literal(",")])
                                        .with_unexpected(current_token.span().before()),
                                );
                            receiver.value_sep(current_token.span().before(), error);
                        }
                        on_scalar(tokens, current_token, receiver, error);
                        state = State::NeedsComma;
                    }
                }
            }
            let previous_span = tokens
                .previous_tokens()
                .find(|t| {
                    !#[allow(non_exhaustive_omitted_patterns)]
                    match t.kind() {
                        TokenKind::Whitespace
                        | TokenKind::Comment
                        | TokenKind::Newline
                        | TokenKind::Eof => true,
                        _ => false,
                    }
                })
                .map(|t| t.span())
                .unwrap_or_default();
            error
                .report_error(
                    ParseError::new("unclosed array")
                        .with_context(array_open.span())
                        .with_expected(&[Expected::Literal("]")])
                        .with_unexpected(previous_span.after()),
                );
            receiver.array_close(previous_span.after(), error);
        }
        /// Parse an inline table
        ///
        /// ```abnf
        /// ;; Inline Table
        ///
        /// inline-table = inline-table-open [ inline-table-keyvals ] ws-comment-newline inline-table-close
        ///
        /// inline-table-keyvals =  ws-comment-newline keyval ws-comment-newline inline-table-sep inline-table-keyvals
        /// inline-table-keyvals =/ ws-comment-newline keyval ws-comment-newline [ inline-table-sep ]
        /// ```
        fn on_inline_table_open(
            tokens: &mut Stream<'_>,
            inline_table_open: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            if !receiver.inline_table_open(inline_table_open.span(), error) {
                ignore_to_value_close(
                    tokens,
                    TokenKind::RightCurlyBracket,
                    receiver,
                    error,
                );
                return;
            }
            #[allow(clippy::enum_variant_names)]
            enum State {
                NeedsKey,
                NeedsEquals,
                NeedsValue,
                NeedsComma,
            }
            #[automatically_derived]
            #[allow(clippy::enum_variant_names)]
            impl ::core::fmt::Debug for State {
                #[inline]
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(
                        f,
                        match self {
                            State::NeedsKey => "NeedsKey",
                            State::NeedsEquals => "NeedsEquals",
                            State::NeedsValue => "NeedsValue",
                            State::NeedsComma => "NeedsComma",
                        },
                    )
                }
            }
            impl State {
                fn expected(&self) -> &'static [Expected] {
                    match self {
                        Self::NeedsKey => &[Expected::Description("key")],
                        Self::NeedsEquals => &[Expected::Literal("=")],
                        Self::NeedsValue => &[Expected::Description("value")],
                        Self::NeedsComma => &[Expected::Literal(",")],
                    }
                }
            }
            let mut state = State::NeedsKey;
            while let Some(current_token) = tokens.next_token() {
                match current_token.kind() {
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error);
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                    }
                    TokenKind::Newline => {
                        receiver.newline(current_token.span(), error);
                    }
                    TokenKind::Eof => {
                        break;
                    }
                    TokenKind::Comma => {
                        match state {
                            State::NeedsKey | State::NeedsEquals | State::NeedsValue => {
                                error
                                    .report_error(
                                        ParseError::new("extra comma in inline table")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                receiver.error(current_token.span(), error);
                            }
                            State::NeedsComma => {
                                receiver.value_sep(current_token.span(), error);
                                state = State::NeedsKey;
                            }
                        }
                    }
                    TokenKind::Equals => {
                        match state {
                            State::NeedsKey => {
                                let fake_key = current_token.span().before();
                                let encoding = None;
                                receiver.simple_key(fake_key, encoding, error);
                                receiver.key_val_sep(current_token.span(), error);
                                state = State::NeedsValue;
                            }
                            State::NeedsEquals => {
                                receiver.key_val_sep(current_token.span(), error);
                                state = State::NeedsValue;
                            }
                            State::NeedsValue | State::NeedsComma => {
                                error
                                    .report_error(
                                        ParseError::new("extra assignment between key-value pairs")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                receiver.error(current_token.span(), error);
                            }
                        }
                    }
                    TokenKind::LeftCurlyBracket => {
                        match state {
                            State::NeedsKey | State::NeedsComma => {
                                error
                                    .report_error(
                                        ParseError::new("missing key for inline table element")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                receiver.error(current_token.span(), error);
                                ignore_to_value_close(
                                    tokens,
                                    TokenKind::RightCurlyBracket,
                                    receiver,
                                    error,
                                );
                            }
                            State::NeedsEquals => {
                                error
                                    .report_error(
                                        ParseError::new(
                                                "missing assignment between key-value pairs",
                                            )
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                on_inline_table_open(
                                    tokens,
                                    current_token,
                                    receiver,
                                    error,
                                );
                                state = State::NeedsComma;
                            }
                            State::NeedsValue => {
                                on_inline_table_open(
                                    tokens,
                                    current_token,
                                    receiver,
                                    error,
                                );
                                state = State::NeedsComma;
                            }
                        }
                    }
                    TokenKind::RightCurlyBracket => {
                        match state {
                            State::NeedsKey => {}
                            State::NeedsEquals => {
                                receiver.key_val_sep(current_token.span().before(), error);
                                receiver
                                    .scalar(
                                        current_token.span().before(),
                                        Some(Encoding::LiteralString),
                                        error,
                                    );
                            }
                            State::NeedsValue => {
                                receiver
                                    .scalar(
                                        current_token.span().before(),
                                        Some(Encoding::LiteralString),
                                        error,
                                    );
                            }
                            State::NeedsComma => {}
                        }
                        receiver.inline_table_close(current_token.span(), error);
                        return;
                    }
                    TokenKind::LeftSquareBracket => {
                        match state {
                            State::NeedsKey | State::NeedsComma => {
                                error
                                    .report_error(
                                        ParseError::new("missing key for inline table element")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                receiver.error(current_token.span(), error);
                                ignore_to_value_close(
                                    tokens,
                                    TokenKind::RightSquareBracket,
                                    receiver,
                                    error,
                                );
                            }
                            State::NeedsEquals => {
                                error
                                    .report_error(
                                        ParseError::new(
                                                "missing assignment between key-value pairs",
                                            )
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                on_array_open(tokens, current_token, receiver, error);
                                state = State::NeedsComma;
                            }
                            State::NeedsValue => {
                                on_array_open(tokens, current_token, receiver, error);
                                state = State::NeedsComma;
                            }
                        }
                    }
                    TokenKind::RightSquareBracket => {
                        match state {
                            State::NeedsKey | State::NeedsEquals | State::NeedsComma => {
                                error
                                    .report_error(
                                        ParseError::new("invalid inline table element")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                receiver.error(current_token.span(), error);
                            }
                            State::NeedsValue => {
                                error
                                    .report_error(
                                        ParseError::new("missing array opening")
                                            .with_context(current_token.span())
                                            .with_expected(&[Expected::Literal("[")])
                                            .with_unexpected(current_token.span().before()),
                                    );
                                let _ = receiver
                                    .array_open(current_token.span().before(), error);
                                receiver.array_close(current_token.span(), error);
                                state = State::NeedsComma;
                            }
                        }
                    }
                    TokenKind::LiteralString
                    | TokenKind::BasicString
                    | TokenKind::MlLiteralString
                    | TokenKind::MlBasicString
                    | TokenKind::Dot
                    | TokenKind::Atom => {
                        match state {
                            State::NeedsKey => {
                                if current_token.kind() == TokenKind::Dot {
                                    receiver
                                        .simple_key(
                                            current_token.span().before(),
                                            current_token.kind().encoding(),
                                            error,
                                        );
                                    seek(tokens, -1);
                                    opt_dot_keys(tokens, receiver, error);
                                    state = State::NeedsEquals;
                                } else {
                                    receiver
                                        .simple_key(
                                            current_token.span(),
                                            current_token.kind().encoding(),
                                            error,
                                        );
                                    opt_dot_keys(tokens, receiver, error);
                                    state = State::NeedsEquals;
                                }
                            }
                            State::NeedsEquals => {
                                error
                                    .report_error(
                                        ParseError::new(
                                                "missing assignment between key-value pairs",
                                            )
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                on_scalar(tokens, current_token, receiver, error);
                                state = State::NeedsComma;
                            }
                            State::NeedsValue => {
                                on_scalar(tokens, current_token, receiver, error);
                                state = State::NeedsComma;
                            }
                            State::NeedsComma => {
                                error
                                    .report_error(
                                        ParseError::new("missing comma between key-value pairs")
                                            .with_context(inline_table_open.span())
                                            .with_expected(state.expected())
                                            .with_unexpected(current_token.span().before()),
                                    );
                                if current_token.kind() == TokenKind::Dot {
                                    receiver
                                        .simple_key(
                                            current_token.span().before(),
                                            current_token.kind().encoding(),
                                            error,
                                        );
                                    seek(tokens, -1);
                                    opt_dot_keys(tokens, receiver, error);
                                    state = State::NeedsEquals;
                                } else {
                                    receiver
                                        .simple_key(
                                            current_token.span(),
                                            current_token.kind().encoding(),
                                            error,
                                        );
                                    opt_dot_keys(tokens, receiver, error);
                                    state = State::NeedsEquals;
                                }
                            }
                        }
                    }
                }
            }
            let previous_span = tokens
                .previous_tokens()
                .find(|t| {
                    !#[allow(non_exhaustive_omitted_patterns)]
                    match t.kind() {
                        TokenKind::Whitespace
                        | TokenKind::Comment
                        | TokenKind::Newline
                        | TokenKind::Eof => true,
                        _ => false,
                    }
                })
                .map(|t| t.span())
                .unwrap_or_default();
            match state {
                State::NeedsKey => {}
                State::NeedsEquals => {
                    receiver.key_val_sep(previous_span.after(), error);
                    receiver
                        .scalar(
                            previous_span.after(),
                            Some(Encoding::LiteralString),
                            error,
                        );
                }
                State::NeedsValue => {
                    receiver
                        .scalar(
                            previous_span.after(),
                            Some(Encoding::LiteralString),
                            error,
                        );
                }
                State::NeedsComma => {}
            }
            error
                .report_error(
                    ParseError::new("unclosed inline table")
                        .with_context(inline_table_open.span())
                        .with_expected(&[Expected::Literal("}")])
                        .with_unexpected(previous_span.after()),
                );
            receiver.inline_table_close(previous_span.after(), error);
        }
        /// Parse whitespace, if present
        ///
        /// ```abnf
        /// ws = *wschar
        /// ```
        fn opt_whitespace(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            if let Some(ws_token) = next_token_if(
                tokens,
                |k| {
                    #[allow(non_exhaustive_omitted_patterns)]
                    match k {
                        TokenKind::Whitespace => true,
                        _ => false,
                    }
                },
            ) {
                receiver.whitespace(ws_token.span(), error);
            }
        }
        /// Parse EOL decor, if present
        ///
        /// ```abnf
        /// toml = expression *( newline expression )
        ///
        /// expression =  ws [ on_comment ]
        /// expression =/ ws keyval ws [ on_comment ]
        /// expression =/ ws table ws [ on_comment ]
        ///
        /// ;; Whitespace
        ///
        /// ws = *wschar
        /// wschar =  %x20  ; Space
        /// wschar =/ %x09  ; Horizontal tab
        ///
        /// ;; Newline
        ///
        /// newline =  %x0A     ; LF
        /// newline =/ %x0D.0A  ; CRLF
        ///
        /// ;; Comment
        ///
        /// comment = comment-start-symbol *non-eol
        /// ```
        fn ws_comment_newline(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut first = None;
            while let Some(current_token) = tokens.next_token() {
                let first = first.get_or_insert(current_token.span());
                match current_token.kind() {
                    TokenKind::Dot
                    | TokenKind::Equals
                    | TokenKind::Comma
                    | TokenKind::LeftSquareBracket
                    | TokenKind::RightSquareBracket
                    | TokenKind::LeftCurlyBracket
                    | TokenKind::RightCurlyBracket
                    | TokenKind::LiteralString
                    | TokenKind::BasicString
                    | TokenKind::MlLiteralString
                    | TokenKind::MlBasicString
                    | TokenKind::Atom => {
                        let context = first.append(current_token.span());
                        error
                            .report_error(
                                ParseError::new("unexpected key or value")
                                    .with_context(context)
                                    .with_expected(
                                        &[Expected::Literal("\n"), Expected::Literal("#")],
                                    )
                                    .with_unexpected(current_token.span().before()),
                            );
                        receiver.error(current_token.span(), error);
                        ignore_to_newline(tokens, receiver, error);
                        break;
                    }
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error);
                        break;
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                        continue;
                    }
                    TokenKind::Newline => {
                        receiver.newline(current_token.span(), error);
                        break;
                    }
                    TokenKind::Eof => {
                        break;
                    }
                }
            }
        }
        /// Start EOL from [`TokenKind::Comment`]
        fn on_comment(
            tokens: &mut Stream<'_>,
            comment_token: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            receiver.comment(comment_token.span(), error);
            let Some(current_token) = tokens.next_token() else {
                return;
            };
            match current_token.kind() {
                TokenKind::Dot
                | TokenKind::Equals
                | TokenKind::Comma
                | TokenKind::LeftSquareBracket
                | TokenKind::RightSquareBracket
                | TokenKind::LeftCurlyBracket
                | TokenKind::RightCurlyBracket
                | TokenKind::Whitespace
                | TokenKind::Comment
                | TokenKind::LiteralString
                | TokenKind::BasicString
                | TokenKind::MlLiteralString
                | TokenKind::MlBasicString
                | TokenKind::Atom => {
                    let context = comment_token.span().append(current_token.span());
                    error
                        .report_error(
                            ParseError::new(
                                    "unexpected content between comment and newline",
                                )
                                .with_context(context)
                                .with_expected(&[Expected::Literal("\n")])
                                .with_unexpected(current_token.span().before()),
                        );
                    receiver.error(current_token.span(), error);
                    ignore_to_newline(tokens, receiver, error);
                }
                TokenKind::Newline => {
                    receiver.newline(current_token.span(), error);
                }
                TokenKind::Eof => {}
            }
        }
        fn eof(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let Some(current_token) = tokens.next_token() else {
                return;
            };
            match current_token.kind() {
                TokenKind::Dot
                | TokenKind::Equals
                | TokenKind::Comma
                | TokenKind::LeftSquareBracket
                | TokenKind::RightSquareBracket
                | TokenKind::LeftCurlyBracket
                | TokenKind::RightCurlyBracket
                | TokenKind::LiteralString
                | TokenKind::BasicString
                | TokenKind::MlLiteralString
                | TokenKind::MlBasicString
                | TokenKind::Atom
                | TokenKind::Comment
                | TokenKind::Whitespace
                | TokenKind::Newline => {
                    error
                        .report_error(
                            ParseError::new("unexpected content")
                                .with_context(current_token.span())
                                .with_expected(&[])
                                .with_unexpected(current_token.span().before()),
                        );
                    receiver.error(current_token.span(), error);
                    while let Some(current_token) = tokens.next_token() {
                        if current_token.kind() == TokenKind::Eof {
                            continue;
                        }
                        receiver.error(current_token.span(), error);
                    }
                }
                TokenKind::Eof => {}
            }
        }
        #[cold]
        fn ignore_to_newline(
            tokens: &mut Stream<'_>,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            while let Some(current_token) = tokens.next_token() {
                match current_token.kind() {
                    TokenKind::Dot
                    | TokenKind::Equals
                    | TokenKind::Comma
                    | TokenKind::LeftSquareBracket
                    | TokenKind::RightSquareBracket
                    | TokenKind::LeftCurlyBracket
                    | TokenKind::RightCurlyBracket
                    | TokenKind::LiteralString
                    | TokenKind::BasicString
                    | TokenKind::MlLiteralString
                    | TokenKind::MlBasicString
                    | TokenKind::Atom => {
                        receiver.error(current_token.span(), error);
                    }
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error);
                        break;
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                    }
                    TokenKind::Newline => {
                        receiver.newline(current_token.span(), error);
                        break;
                    }
                    TokenKind::Eof => {
                        break;
                    }
                }
            }
        }
        /// Don't bother recovering until the matching [`TokenKind`]
        ///
        /// Attempts to ignore nested `[]`, `{}`.
        #[cold]
        fn ignore_to_value_close(
            tokens: &mut Stream<'_>,
            closing_kind: TokenKind,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            let mut array_count: usize = 0;
            let mut inline_table_count: usize = 0;
            while let Some(current_token) = tokens.next_token() {
                match current_token.kind() {
                    TokenKind::Dot
                    | TokenKind::Equals
                    | TokenKind::Comma
                    | TokenKind::LiteralString
                    | TokenKind::BasicString
                    | TokenKind::MlLiteralString
                    | TokenKind::MlBasicString
                    | TokenKind::Atom => {
                        receiver.error(current_token.span(), error);
                    }
                    TokenKind::Comment => {
                        on_comment(tokens, current_token, receiver, error);
                    }
                    TokenKind::Whitespace => {
                        receiver.whitespace(current_token.span(), error);
                    }
                    TokenKind::Newline => {
                        receiver.newline(current_token.span(), error);
                    }
                    TokenKind::LeftSquareBracket => {
                        receiver.error(current_token.span(), error);
                        array_count += 1;
                    }
                    TokenKind::RightSquareBracket => {
                        if array_count == 0 && current_token.kind() == closing_kind {
                            receiver.array_close(current_token.span(), error);
                            break;
                        } else {
                            receiver.error(current_token.span(), error);
                            array_count = array_count.saturating_sub(1);
                        }
                    }
                    TokenKind::LeftCurlyBracket => {
                        receiver.error(current_token.span(), error);
                        inline_table_count += 1;
                    }
                    TokenKind::RightCurlyBracket => {
                        if inline_table_count == 0
                            && current_token.kind() == closing_kind
                        {
                            receiver.inline_table_close(current_token.span(), error);
                            break;
                        } else {
                            receiver.error(current_token.span(), error);
                            inline_table_count = inline_table_count.saturating_sub(1);
                        }
                    }
                    TokenKind::Eof => {
                        break;
                    }
                }
            }
        }
        #[cold]
        fn on_missing_key(
            tokens: &mut Stream<'_>,
            token: &Token,
            invalid_description: &'static str,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            error
                .report_error(
                    ParseError::new(invalid_description)
                        .with_context(token.span())
                        .with_expected(&[Expected::Description("key")])
                        .with_unexpected(token.span().before()),
                );
            if token.kind() == TokenKind::Eof
            {} else if token.kind() == TokenKind::Newline {
                receiver.newline(token.span(), error);
            } else if token.kind() == TokenKind::Comment {
                on_comment(tokens, token, receiver, error);
            } else {
                receiver.error(token.span(), error);
            }
        }
        #[cold]
        fn on_missing_expression_key(
            tokens: &mut Stream<'_>,
            token: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            error
                .report_error(
                    ParseError::new("invalid key-value pair")
                        .with_context(token.span())
                        .with_expected(&[Expected::Description("key")])
                        .with_unexpected(token.span().before()),
                );
            receiver.error(token.span(), error);
            ignore_to_newline(tokens, receiver, error);
        }
        #[cold]
        fn on_missing_std_table(
            tokens: &mut Stream<'_>,
            token: &Token,
            receiver: &mut dyn EventReceiver,
            error: &mut dyn ErrorSink,
        ) {
            error
                .report_error(
                    ParseError::new("missing table open")
                        .with_context(token.span())
                        .with_expected(&[Expected::Literal("[")])
                        .with_unexpected(token.span().before()),
                );
            receiver.error(token.span(), error);
            ignore_to_newline(tokens, receiver, error);
        }
        fn next_token_if<'i, F: Fn(TokenKind) -> bool>(
            tokens: &mut Stream<'i>,
            pred: F,
        ) -> Option<&'i Token> {
            match tokens.first() {
                Some(next) if pred(next.kind()) => tokens.next_token(),
                _ => None,
            }
        }
        fn seek(stream: &mut Stream<'_>, offset: isize) {
            let current = stream.checkpoint();
            stream.reset_to_start();
            let start = stream.checkpoint();
            let old_offset = current.offset_from(&start);
            let new_offset = (old_offset as isize).saturating_add(offset) as usize;
            if new_offset < stream.eof_offset() {
                stream.next_slice(new_offset);
            } else {
                stream.finish();
            }
        }
        const UNQUOTED_STRING: &str = "unquoted string";
    }
    mod event {
        use crate::ErrorSink;
        use crate::ParseError;
        use crate::Source;
        use crate::Span;
        use crate::decoder::Encoding;
        pub trait EventReceiver {
            fn std_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn std_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn array_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            /// Returns if entering the inline table is allowed
            #[must_use]
            fn inline_table_open(
                &mut self,
                _span: Span,
                _error: &mut dyn ErrorSink,
            ) -> bool {
                true
            }
            fn inline_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            /// Returns if entering the array is allowed
            #[must_use]
            fn array_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
                true
            }
            fn array_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn simple_key(
                &mut self,
                _span: Span,
                _kind: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {}
            fn key_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn key_val_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn scalar(
                &mut self,
                _span: Span,
                _kind: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {}
            fn value_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn whitespace(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn comment(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn newline(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
            fn error(&mut self, _span: Span, _error: &mut dyn ErrorSink) {}
        }
        impl<F> EventReceiver for F
        where
            F: FnMut(Event),
        {
            fn std_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::StdTableOpen,
                    encoding: None,
                    span,
                });
            }
            fn std_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::StdTableClose,
                    encoding: None,
                    span,
                });
            }
            fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::ArrayTableOpen,
                    encoding: None,
                    span,
                });
            }
            fn array_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::ArrayTableClose,
                    encoding: None,
                    span,
                });
            }
            fn inline_table_open(
                &mut self,
                span: Span,
                _error: &mut dyn ErrorSink,
            ) -> bool {
                (self)(Event {
                    kind: EventKind::InlineTableOpen,
                    encoding: None,
                    span,
                });
                true
            }
            fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::InlineTableClose,
                    encoding: None,
                    span,
                });
            }
            fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
                (self)(Event {
                    kind: EventKind::ArrayOpen,
                    encoding: None,
                    span,
                });
                true
            }
            fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::ArrayClose,
                    encoding: None,
                    span,
                });
            }
            fn simple_key(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {
                (self)(Event {
                    kind: EventKind::SimpleKey,
                    encoding,
                    span,
                });
            }
            fn key_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::KeySep,
                    encoding: None,
                    span,
                });
            }
            fn key_val_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::KeyValSep,
                    encoding: None,
                    span,
                });
            }
            fn scalar(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {
                (self)(Event {
                    kind: EventKind::Scalar,
                    encoding,
                    span,
                });
            }
            fn value_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::ValueSep,
                    encoding: None,
                    span,
                });
            }
            fn whitespace(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::Whitespace,
                    encoding: None,
                    span,
                });
            }
            fn comment(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::Comment,
                    encoding: None,
                    span,
                });
            }
            fn newline(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::Newline,
                    encoding: None,
                    span,
                });
            }
            fn error(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                (self)(Event {
                    kind: EventKind::Error,
                    encoding: None,
                    span,
                });
            }
        }
        #[allow(unused_qualifications)]
        impl EventReceiver for alloc::vec::Vec<Event> {
            fn std_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::StdTableOpen,
                    encoding: None,
                    span,
                });
            }
            fn std_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::StdTableClose,
                    encoding: None,
                    span,
                });
            }
            fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::ArrayTableOpen,
                    encoding: None,
                    span,
                });
            }
            fn array_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::ArrayTableClose,
                    encoding: None,
                    span,
                });
            }
            fn inline_table_open(
                &mut self,
                span: Span,
                _error: &mut dyn ErrorSink,
            ) -> bool {
                self.push(Event {
                    kind: EventKind::InlineTableOpen,
                    encoding: None,
                    span,
                });
                true
            }
            fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::InlineTableClose,
                    encoding: None,
                    span,
                });
            }
            fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
                self.push(Event {
                    kind: EventKind::ArrayOpen,
                    encoding: None,
                    span,
                });
                true
            }
            fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::ArrayClose,
                    encoding: None,
                    span,
                });
            }
            fn simple_key(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {
                self.push(Event {
                    kind: EventKind::SimpleKey,
                    encoding,
                    span,
                });
            }
            fn key_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::KeySep,
                    encoding: None,
                    span,
                });
            }
            fn key_val_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::KeyValSep,
                    encoding: None,
                    span,
                });
            }
            fn scalar(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                _error: &mut dyn ErrorSink,
            ) {
                self.push(Event {
                    kind: EventKind::Scalar,
                    encoding,
                    span,
                });
            }
            fn value_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::ValueSep,
                    encoding: None,
                    span,
                });
            }
            fn whitespace(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::Whitespace,
                    encoding: None,
                    span,
                });
            }
            fn comment(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::Comment,
                    encoding: None,
                    span,
                });
            }
            fn newline(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::Newline,
                    encoding: None,
                    span,
                });
            }
            fn error(&mut self, span: Span, _error: &mut dyn ErrorSink) {
                self.push(Event {
                    kind: EventKind::Error,
                    encoding: None,
                    span,
                });
            }
        }
        impl EventReceiver for () {}
        /// Centralize validation for all whitespace-like content
        pub struct ValidateWhitespace<'r, 's> {
            receiver: &'r mut dyn EventReceiver,
            source: Source<'s>,
        }
        impl<'r, 's> ValidateWhitespace<'r, 's> {
            pub fn new(receiver: &'r mut dyn EventReceiver, source: Source<'s>) -> Self {
                Self { receiver, source }
            }
        }
        impl EventReceiver for ValidateWhitespace<'_, '_> {
            fn std_table_open(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.std_table_open(span, error);
            }
            fn std_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.std_table_close(span, error);
            }
            fn array_table_open(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.array_table_open(span, error);
            }
            fn array_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.array_table_close(span, error);
            }
            fn inline_table_open(
                &mut self,
                span: Span,
                error: &mut dyn ErrorSink,
            ) -> bool {
                self.receiver.inline_table_open(span, error)
            }
            fn inline_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.inline_table_close(span, error);
            }
            fn array_open(&mut self, span: Span, error: &mut dyn ErrorSink) -> bool {
                self.receiver.array_open(span, error)
            }
            fn array_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.array_close(span, error);
            }
            fn simple_key(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                error: &mut dyn ErrorSink,
            ) {
                self.receiver.simple_key(span, encoding, error);
            }
            fn key_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.key_sep(span, error);
            }
            fn key_val_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.key_val_sep(span, error);
            }
            fn scalar(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                error: &mut dyn ErrorSink,
            ) {
                self.receiver.scalar(span, encoding, error);
            }
            fn value_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.value_sep(span, error);
            }
            fn whitespace(&mut self, span: Span, error: &mut dyn ErrorSink) {
                let raw = self.source.get(span).expect("token spans are valid");
                raw.decode_whitespace(error);
                self.receiver.whitespace(span, error);
            }
            fn comment(&mut self, span: Span, error: &mut dyn ErrorSink) {
                let raw = self.source.get(span).expect("token spans are valid");
                raw.decode_comment(error);
                self.receiver.comment(span, error);
            }
            fn newline(&mut self, span: Span, error: &mut dyn ErrorSink) {
                let raw = self.source.get(span).expect("token spans are valid");
                raw.decode_newline(error);
                self.receiver.newline(span, error);
            }
            fn error(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.error(span, error);
            }
        }
        pub struct RecursionGuard<'r> {
            receiver: &'r mut dyn EventReceiver,
            max_depth: u32,
            depth: i64,
        }
        impl<'r> RecursionGuard<'r> {
            pub fn new(receiver: &'r mut dyn EventReceiver, max_depth: u32) -> Self {
                Self {
                    receiver,
                    max_depth,
                    depth: 0,
                }
            }
            fn within_depth(&self) -> bool {
                self.depth <= self.max_depth as i64
            }
        }
        impl EventReceiver for RecursionGuard<'_> {
            fn std_table_open(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.std_table_open(span, error);
            }
            fn std_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.std_table_close(span, error);
            }
            fn array_table_open(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.array_table_open(span, error);
            }
            fn array_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.array_table_close(span, error);
            }
            fn inline_table_open(
                &mut self,
                span: Span,
                error: &mut dyn ErrorSink,
            ) -> bool {
                let allowed = self.receiver.inline_table_open(span, error);
                self.depth += 1;
                let within_depth = self.within_depth();
                if allowed && !within_depth {
                    error
                        .report_error(
                            ParseError::new(
                                    "cannot recurse further; max recursion depth met",
                                )
                                .with_unexpected(span),
                        );
                }
                allowed && within_depth
            }
            fn inline_table_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.depth -= 1;
                self.receiver.inline_table_close(span, error);
            }
            fn array_open(&mut self, span: Span, error: &mut dyn ErrorSink) -> bool {
                let allowed = self.receiver.array_open(span, error);
                self.depth += 1;
                let within_depth = self.within_depth();
                if allowed && !within_depth {
                    error
                        .report_error(
                            ParseError::new(
                                    "cannot recurse further; max recursion depth met",
                                )
                                .with_unexpected(span),
                        );
                }
                allowed && within_depth
            }
            fn array_close(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.depth -= 1;
                self.receiver.array_close(span, error);
            }
            fn simple_key(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                error: &mut dyn ErrorSink,
            ) {
                self.receiver.simple_key(span, encoding, error);
            }
            fn key_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.key_sep(span, error);
            }
            fn key_val_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.key_val_sep(span, error);
            }
            fn scalar(
                &mut self,
                span: Span,
                encoding: Option<Encoding>,
                error: &mut dyn ErrorSink,
            ) {
                self.receiver.scalar(span, encoding, error);
            }
            fn value_sep(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.value_sep(span, error);
            }
            fn whitespace(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.whitespace(span, error);
            }
            fn comment(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.comment(span, error);
            }
            fn newline(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.newline(span, error);
            }
            fn error(&mut self, span: Span, error: &mut dyn ErrorSink) {
                self.receiver.error(span, error);
            }
        }
        pub struct Event {
            kind: EventKind,
            encoding: Option<Encoding>,
            span: Span,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Event {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Event {}
        #[automatically_derived]
        impl ::core::clone::Clone for Event {
            #[inline]
            fn clone(&self) -> Event {
                let _: ::core::clone::AssertParamIsClone<EventKind>;
                let _: ::core::clone::AssertParamIsClone<Option<Encoding>>;
                let _: ::core::clone::AssertParamIsClone<Span>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Event {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Event {
            #[inline]
            fn eq(&self, other: &Event) -> bool {
                self.kind == other.kind && self.encoding == other.encoding
                    && self.span == other.span
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Event {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<EventKind>;
                let _: ::core::cmp::AssertParamIsEq<Option<Encoding>>;
                let _: ::core::cmp::AssertParamIsEq<Span>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Event {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Event,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                match ::core::cmp::PartialOrd::partial_cmp(&self.kind, &other.kind) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        match ::core::cmp::PartialOrd::partial_cmp(
                            &self.encoding,
                            &other.encoding,
                        ) {
                            ::core::option::Option::Some(
                                ::core::cmp::Ordering::Equal,
                            ) => {
                                ::core::cmp::PartialOrd::partial_cmp(
                                    &self.span,
                                    &other.span,
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
        impl ::core::cmp::Ord for Event {
            #[inline]
            fn cmp(&self, other: &Event) -> ::core::cmp::Ordering {
                match ::core::cmp::Ord::cmp(&self.kind, &other.kind) {
                    ::core::cmp::Ordering::Equal => {
                        match ::core::cmp::Ord::cmp(&self.encoding, &other.encoding) {
                            ::core::cmp::Ordering::Equal => {
                                ::core::cmp::Ord::cmp(&self.span, &other.span)
                            }
                            cmp => cmp,
                        }
                    }
                    cmp => cmp,
                }
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Event {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.kind, state);
                ::core::hash::Hash::hash(&self.encoding, state);
                ::core::hash::Hash::hash(&self.span, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Event {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field3_finish(
                    f,
                    "Event",
                    "kind",
                    &self.kind,
                    "encoding",
                    &self.encoding,
                    "span",
                    &&self.span,
                )
            }
        }
        impl Event {
            pub fn new_unchecked(
                kind: EventKind,
                encoding: Option<Encoding>,
                span: Span,
            ) -> Self {
                Self { kind, encoding, span }
            }
            #[inline(always)]
            pub fn kind(&self) -> EventKind {
                self.kind
            }
            #[inline(always)]
            pub fn encoding(&self) -> Option<Encoding> {
                self.encoding
            }
            #[inline(always)]
            pub fn span(&self) -> Span {
                self.span
            }
        }
        pub enum EventKind {
            StdTableOpen,
            StdTableClose,
            ArrayTableOpen,
            ArrayTableClose,
            InlineTableOpen,
            InlineTableClose,
            ArrayOpen,
            ArrayClose,
            SimpleKey,
            KeySep,
            KeyValSep,
            Scalar,
            ValueSep,
            Whitespace,
            Comment,
            Newline,
            Error,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for EventKind {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for EventKind {}
        #[automatically_derived]
        impl ::core::clone::Clone for EventKind {
            #[inline]
            fn clone(&self) -> EventKind {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for EventKind {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for EventKind {
            #[inline]
            fn eq(&self, other: &EventKind) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for EventKind {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) {}
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for EventKind {
            #[inline]
            fn partial_cmp(
                &self,
                other: &EventKind,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for EventKind {
            #[inline]
            fn cmp(&self, other: &EventKind) -> ::core::cmp::Ordering {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_discr, &__arg1_discr)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for EventKind {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_discr, state)
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for EventKind {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        EventKind::StdTableOpen => "StdTableOpen",
                        EventKind::StdTableClose => "StdTableClose",
                        EventKind::ArrayTableOpen => "ArrayTableOpen",
                        EventKind::ArrayTableClose => "ArrayTableClose",
                        EventKind::InlineTableOpen => "InlineTableOpen",
                        EventKind::InlineTableClose => "InlineTableClose",
                        EventKind::ArrayOpen => "ArrayOpen",
                        EventKind::ArrayClose => "ArrayClose",
                        EventKind::SimpleKey => "SimpleKey",
                        EventKind::KeySep => "KeySep",
                        EventKind::KeyValSep => "KeyValSep",
                        EventKind::Scalar => "Scalar",
                        EventKind::ValueSep => "ValueSep",
                        EventKind::Whitespace => "Whitespace",
                        EventKind::Comment => "Comment",
                        EventKind::Newline => "Newline",
                        EventKind::Error => "Error",
                    },
                )
            }
        }
        impl EventKind {
            pub const fn description(&self) -> &'static str {
                match self {
                    Self::StdTableOpen => "std-table open",
                    Self::StdTableClose => "std-table close",
                    Self::ArrayTableOpen => "array-table open",
                    Self::ArrayTableClose => "array-table close",
                    Self::InlineTableOpen => "inline-table open",
                    Self::InlineTableClose => "inline-table close",
                    Self::ArrayOpen => "array open",
                    Self::ArrayClose => "array close",
                    Self::SimpleKey => "key",
                    Self::KeySep => "key separator",
                    Self::KeyValSep => "key-value separator",
                    Self::Scalar => "value",
                    Self::ValueSep => "value separator",
                    Self::Whitespace => "whitespace",
                    Self::Comment => "comment",
                    Self::Newline => "newline",
                    Self::Error => "error",
                }
            }
        }
    }
    pub use document::parse_document;
    pub use document::parse_key;
    pub use document::parse_simple_key;
    pub use document::parse_value;
    pub use event::Event;
    pub use event::EventKind;
    pub use event::EventReceiver;
    pub use event::RecursionGuard;
    pub use event::ValidateWhitespace;
}
pub use error::ErrorSink;
pub use error::Expected;
pub use error::ParseError;
pub use source::Raw;
pub use source::Source;
pub use source::SourceIndex;
pub use source::Span;
