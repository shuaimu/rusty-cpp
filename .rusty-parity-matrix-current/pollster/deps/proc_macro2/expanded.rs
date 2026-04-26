#![feature(prelude_import)]
//! [![github]](https://github.com/dtolnay/proc-macro2)&ensp;[![crates-io]](https://crates.io/crates/proc-macro2)&ensp;[![docs-rs]](crate)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! A wrapper around the procedural macro API of the compiler's [`proc_macro`]
//! crate. This library serves two purposes:
//!
//! - **Bring proc-macro-like functionality to other contexts like build.rs and
//!   main.rs.** Types from `proc_macro` are entirely specific to procedural
//!   macros and cannot ever exist in code outside of a procedural macro.
//!   Meanwhile `proc_macro2` types may exist anywhere including non-macro code.
//!   By developing foundational libraries like [syn] and [quote] against
//!   `proc_macro2` rather than `proc_macro`, the procedural macro ecosystem
//!   becomes easily applicable to many other use cases and we avoid
//!   reimplementing non-macro equivalents of those libraries.
//!
//! - **Make procedural macros unit testable.** As a consequence of being
//!   specific to procedural macros, nothing that uses `proc_macro` can be
//!   executed from a unit test. In order for helper libraries or components of
//!   a macro to be testable in isolation, they must be implemented using
//!   `proc_macro2`.
//!
//! [syn]: https://github.com/dtolnay/syn
//! [quote]: https://github.com/dtolnay/quote
//!
//! # Usage
//!
//! The skeleton of a typical procedural macro typically looks like this:
//!
//! ```
//! extern crate proc_macro;
//!
//! # const IGNORE: &str = stringify! {
//! #[proc_macro_derive(MyDerive)]
//! # };
//! # #[cfg(wrap_proc_macro)]
//! pub fn my_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//!     let input = proc_macro2::TokenStream::from(input);
//!
//!     let output: proc_macro2::TokenStream = {
//!         /* transform input */
//!         # input
//!     };
//!
//!     proc_macro::TokenStream::from(output)
//! }
//! ```
//!
//! If parsing with [Syn], you'll use [`parse_macro_input!`] instead to
//! propagate parse errors correctly back to the compiler when parsing fails.
//!
//! [`parse_macro_input!`]: https://docs.rs/syn/2.0/syn/macro.parse_macro_input.html
//!
//! # Unstable features
//!
//! The default feature set of proc-macro2 tracks the most recent stable
//! compiler API. Functionality in `proc_macro` that is not yet stable is not
//! exposed by proc-macro2 by default.
//!
//! To opt into the additional APIs available in the most recent nightly
//! compiler, the `procmacro2_semver_exempt` config flag must be passed to
//! rustc. We will polyfill those nightly-only APIs back to Rust 1.68.0. As
//! these are unstable APIs that track the nightly compiler, minor versions of
//! proc-macro2 may make breaking changes to them at any time.
//!
//! ```sh
//! RUSTFLAGS='--cfg procmacro2_semver_exempt' cargo build
//! ```
//!
//! Note that this must not only be done for your crate, but for any crate that
//! depends on your crate. This infectious nature is intentional, as it serves
//! as a reminder that you are outside of the normal semver guarantees.
//!
//! Semver exempt methods are marked as such in the proc-macro2 documentation.
//!
//! # Thread-Safety
//!
//! Most types in this crate are `!Sync` because the underlying compiler
//! types make use of thread-local memory, meaning they cannot be accessed from
//! a different thread.
#![no_std]
#![doc(html_root_url = "https://docs.rs/proc-macro2/1.0.106")]
#![feature(proc_macro_span)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::checked_conversions,
    clippy::doc_markdown,
    clippy::elidable_lifetime_names,
    clippy::incompatible_msrv,
    clippy::items_after_statements,
    clippy::iter_without_into_iter,
    clippy::let_underscore_untyped,
    clippy::manual_assert,
    clippy::manual_range_contains,
    clippy::missing_panics_doc,
    clippy::missing_safety_doc,
    clippy::must_use_candidate,
    clippy::needless_doctest_main,
    clippy::needless_lifetimes,
    clippy::new_without_default,
    clippy::return_self_not_must_use,
    clippy::shadow_unrelated,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::vec_init_then_push
)]
#![allow(unknown_lints, mismatched_lifetime_syntaxes)]
extern crate core;
#[prelude_import]
use core::prelude::rust_2021::*;
extern crate alloc;
extern crate std;
extern crate proc_macro;
mod marker {
    use alloc::rc::Rc;
    use core::marker::PhantomData;
    use core::panic::{RefUnwindSafe, UnwindSafe};
    pub(crate) struct ProcMacroAutoTraits(PhantomData<Rc<()>>);
    #[automatically_derived]
    impl ::core::marker::Copy for ProcMacroAutoTraits {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for ProcMacroAutoTraits {}
    #[automatically_derived]
    impl ::core::clone::Clone for ProcMacroAutoTraits {
        #[inline]
        fn clone(&self) -> ProcMacroAutoTraits {
            let _: ::core::clone::AssertParamIsClone<PhantomData<Rc<()>>>;
            *self
        }
    }
    pub(crate) const MARKER: ProcMacroAutoTraits = ProcMacroAutoTraits(PhantomData);
    impl UnwindSafe for ProcMacroAutoTraits {}
    impl RefUnwindSafe for ProcMacroAutoTraits {}
}
mod parse {
    use crate::fallback::{
        self, is_ident_continue, is_ident_start, Group, Ident, LexError, Literal, Span,
        TokenStream, TokenStreamBuilder,
    };
    use crate::{Delimiter, Punct, Spacing, TokenTree};
    use alloc::borrow::ToOwned as _;
    use alloc::string::ToString as _;
    use alloc::vec::Vec;
    use core::char;
    use core::str::{Bytes, CharIndices, Chars};
    pub(crate) struct Cursor<'a> {
        pub(crate) rest: &'a str,
    }
    #[automatically_derived]
    impl<'a> ::core::marker::Copy for Cursor<'a> {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl<'a> ::core::clone::TrivialClone for Cursor<'a> {}
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for Cursor<'a> {
        #[inline]
        fn clone(&self) -> Cursor<'a> {
            let _: ::core::clone::AssertParamIsClone<&'a str>;
            *self
        }
    }
    #[automatically_derived]
    impl<'a> ::core::cmp::Eq for Cursor<'a> {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<&'a str>;
        }
    }
    #[automatically_derived]
    impl<'a> ::core::marker::StructuralPartialEq for Cursor<'a> {}
    #[automatically_derived]
    impl<'a> ::core::cmp::PartialEq for Cursor<'a> {
        #[inline]
        fn eq(&self, other: &Cursor<'a>) -> bool {
            self.rest == other.rest
        }
    }
    impl<'a> Cursor<'a> {
        pub(crate) fn advance(&self, bytes: usize) -> Cursor<'a> {
            let (_front, rest) = self.rest.split_at(bytes);
            Cursor { rest }
        }
        pub(crate) fn starts_with(&self, s: &str) -> bool {
            self.rest.starts_with(s)
        }
        pub(crate) fn starts_with_char(&self, ch: char) -> bool {
            self.rest.starts_with(ch)
        }
        pub(crate) fn starts_with_fn<Pattern>(&self, f: Pattern) -> bool
        where
            Pattern: FnMut(char) -> bool,
        {
            self.rest.starts_with(f)
        }
        pub(crate) fn is_empty(&self) -> bool {
            self.rest.is_empty()
        }
        fn len(&self) -> usize {
            self.rest.len()
        }
        fn as_bytes(&self) -> &'a [u8] {
            self.rest.as_bytes()
        }
        fn bytes(&self) -> Bytes<'a> {
            self.rest.bytes()
        }
        fn chars(&self) -> Chars<'a> {
            self.rest.chars()
        }
        fn char_indices(&self) -> CharIndices<'a> {
            self.rest.char_indices()
        }
        fn parse(&self, tag: &str) -> Result<Cursor<'a>, Reject> {
            if self.starts_with(tag) { Ok(self.advance(tag.len())) } else { Err(Reject) }
        }
    }
    pub(crate) struct Reject;
    type PResult<'a, O> = Result<(Cursor<'a>, O), Reject>;
    fn skip_whitespace(input: Cursor) -> Cursor {
        let mut s = input;
        while !s.is_empty() {
            let byte = s.as_bytes()[0];
            if byte == b'/' {
                if s.starts_with("//")
                    && (!s.starts_with("///") || s.starts_with("////"))
                    && !s.starts_with("//!")
                {
                    let (cursor, _) = take_until_newline_or_eof(s);
                    s = cursor;
                    continue;
                } else if s.starts_with("/**/") {
                    s = s.advance(4);
                    continue;
                } else if s.starts_with("/*")
                    && (!s.starts_with("/**") || s.starts_with("/***"))
                    && !s.starts_with("/*!")
                {
                    match block_comment(s) {
                        Ok((rest, _)) => {
                            s = rest;
                            continue;
                        }
                        Err(Reject) => return s,
                    }
                }
            }
            match byte {
                b' ' | 0x09..=0x0d => {
                    s = s.advance(1);
                    continue;
                }
                b if b.is_ascii() => {}
                _ => {
                    let ch = s.chars().next().unwrap();
                    if is_whitespace(ch) {
                        s = s.advance(ch.len_utf8());
                        continue;
                    }
                }
            }
            return s;
        }
        s
    }
    fn block_comment(input: Cursor) -> PResult<&str> {
        if !input.starts_with("/*") {
            return Err(Reject);
        }
        let mut depth = 0usize;
        let bytes = input.as_bytes();
        let mut i = 0usize;
        let upper = bytes.len() - 1;
        while i < upper {
            if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                depth += 1;
                i += 1;
            } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                depth -= 1;
                if depth == 0 {
                    return Ok((input.advance(i + 2), &input.rest[..i + 2]));
                }
                i += 1;
            }
            i += 1;
        }
        Err(Reject)
    }
    fn is_whitespace(ch: char) -> bool {
        ch.is_whitespace() || ch == '\u{200e}' || ch == '\u{200f}'
    }
    fn word_break(input: Cursor) -> Result<Cursor, Reject> {
        match input.chars().next() {
            Some(ch) if is_ident_continue(ch) => Err(Reject),
            Some(_) | None => Ok(input),
        }
    }
    const ERROR: &str = "(/*ERROR*/)";
    pub(crate) fn token_stream(mut input: Cursor) -> Result<TokenStream, LexError> {
        let mut tokens = TokenStreamBuilder::new();
        let mut stack = Vec::new();
        loop {
            input = skip_whitespace(input);
            if let Ok((rest, ())) = doc_comment(input, &mut tokens) {
                input = rest;
                continue;
            }
            let Some(first) = input.bytes().next() else {
                return match stack.last() {
                    None => Ok(tokens.build()),
                    Some(_frame) => Err(LexError { span: Span {} }),
                };
            };
            if let Some(open_delimiter) = match first {
                b'(' if !input.starts_with(ERROR) => Some(Delimiter::Parenthesis),
                b'[' => Some(Delimiter::Bracket),
                b'{' => Some(Delimiter::Brace),
                _ => None,
            } {
                input = input.advance(1);
                let frame = (open_delimiter, tokens);
                stack.push(frame);
                tokens = TokenStreamBuilder::new();
            } else if let Some(close_delimiter) = match first {
                b')' => Some(Delimiter::Parenthesis),
                b']' => Some(Delimiter::Bracket),
                b'}' => Some(Delimiter::Brace),
                _ => None,
            } {
                let Some(frame) = stack.pop() else {
                    return Err(lex_error(input));
                };
                let (open_delimiter, outer) = frame;
                if open_delimiter != close_delimiter {
                    return Err(lex_error(input));
                }
                input = input.advance(1);
                let mut g = Group::new(open_delimiter, tokens.build());
                g.set_span(Span {});
                tokens = outer;
                tokens
                    .push_token_from_parser(
                        TokenTree::Group(crate::Group::_new_fallback(g)),
                    );
            } else {
                let (rest, mut tt) = match leaf_token(input) {
                    Ok((rest, tt)) => (rest, tt),
                    Err(Reject) => return Err(lex_error(input)),
                };
                tt.set_span(crate::Span::_new_fallback(Span {}));
                tokens.push_token_from_parser(tt);
                input = rest;
            }
        }
    }
    fn lex_error(cursor: Cursor) -> LexError {
        let _ = cursor;
        LexError { span: Span {} }
    }
    fn leaf_token(input: Cursor) -> PResult<TokenTree> {
        if let Ok((input, l)) = literal(input) {
            Ok((input, TokenTree::Literal(crate::Literal::_new_fallback(l))))
        } else if let Ok((input, p)) = punct(input) {
            Ok((input, TokenTree::Punct(p)))
        } else if let Ok((input, i)) = ident(input) {
            Ok((input, TokenTree::Ident(i)))
        } else if input.starts_with(ERROR) {
            let rest = input.advance(ERROR.len());
            let repr = crate::Literal::_new_fallback(Literal::_new(ERROR.to_owned()));
            Ok((rest, TokenTree::Literal(repr)))
        } else {
            Err(Reject)
        }
    }
    fn ident(input: Cursor) -> PResult<crate::Ident> {
        if ["r\"", "r#\"", "r##", "b\"", "b\'", "br\"", "br#", "c\"", "cr\"", "cr#"]
            .iter()
            .any(|prefix| input.starts_with(prefix))
        {
            Err(Reject)
        } else {
            ident_any(input)
        }
    }
    fn ident_any(input: Cursor) -> PResult<crate::Ident> {
        let raw = input.starts_with("r#");
        let rest = input.advance((raw as usize) << 1);
        let (rest, sym) = ident_not_raw(rest)?;
        if !raw {
            let ident = crate::Ident::_new_fallback(
                Ident::new_unchecked(sym, fallback::Span::call_site()),
            );
            return Ok((rest, ident));
        }
        match sym {
            "_" | "super" | "self" | "Self" | "crate" => return Err(Reject),
            _ => {}
        }
        let ident = crate::Ident::_new_fallback(
            Ident::new_raw_unchecked(sym, fallback::Span::call_site()),
        );
        Ok((rest, ident))
    }
    fn ident_not_raw(input: Cursor) -> PResult<&str> {
        let mut chars = input.char_indices();
        match chars.next() {
            Some((_, ch)) if is_ident_start(ch) => {}
            _ => return Err(Reject),
        }
        let mut end = input.len();
        for (i, ch) in chars {
            if !is_ident_continue(ch) {
                end = i;
                break;
            }
        }
        Ok((input.advance(end), &input.rest[..end]))
    }
    pub(crate) fn literal(input: Cursor) -> PResult<Literal> {
        let rest = literal_nocapture(input)?;
        let end = input.len() - rest.len();
        Ok((rest, Literal::_new(input.rest[..end].to_string())))
    }
    fn literal_nocapture(input: Cursor) -> Result<Cursor, Reject> {
        if let Ok(ok) = string(input) {
            Ok(ok)
        } else if let Ok(ok) = byte_string(input) {
            Ok(ok)
        } else if let Ok(ok) = c_string(input) {
            Ok(ok)
        } else if let Ok(ok) = byte(input) {
            Ok(ok)
        } else if let Ok(ok) = character(input) {
            Ok(ok)
        } else if let Ok(ok) = float(input) {
            Ok(ok)
        } else if let Ok(ok) = int(input) {
            Ok(ok)
        } else {
            Err(Reject)
        }
    }
    fn literal_suffix(input: Cursor) -> Cursor {
        match ident_not_raw(input) {
            Ok((input, _)) => input,
            Err(Reject) => input,
        }
    }
    fn string(input: Cursor) -> Result<Cursor, Reject> {
        if let Ok(input) = input.parse("\"") {
            cooked_string(input)
        } else if let Ok(input) = input.parse("r") {
            raw_string(input)
        } else {
            Err(Reject)
        }
    }
    fn cooked_string(mut input: Cursor) -> Result<Cursor, Reject> {
        let mut chars = input.char_indices();
        while let Some((i, ch)) = chars.next() {
            match ch {
                '"' => {
                    let input = input.advance(i + 1);
                    return Ok(literal_suffix(input));
                }
                '\r' => {
                    match chars.next() {
                        Some((_, '\n')) => {}
                        _ => break,
                    }
                }
                '\\' => {
                    match chars.next() {
                        Some((_, 'x')) => {
                            backslash_x_char(&mut chars)?;
                        }
                        Some((_, 'n' | 'r' | 't' | '\\' | '\'' | '"' | '0')) => {}
                        Some((_, 'u')) => {
                            backslash_u(&mut chars)?;
                        }
                        Some((newline, ch @ ('\n' | '\r'))) => {
                            input = input.advance(newline + 1);
                            trailing_backslash(&mut input, ch as u8)?;
                            chars = input.char_indices();
                        }
                        _ => break,
                    }
                }
                _ch => {}
            }
        }
        Err(Reject)
    }
    fn raw_string(input: Cursor) -> Result<Cursor, Reject> {
        let (input, delimiter) = delimiter_of_raw_string(input)?;
        let mut bytes = input.bytes().enumerate();
        while let Some((i, byte)) = bytes.next() {
            match byte {
                b'"' if input.rest[i + 1..].starts_with(delimiter) => {
                    let rest = input.advance(i + 1 + delimiter.len());
                    return Ok(literal_suffix(rest));
                }
                b'\r' => {
                    match bytes.next() {
                        Some((_, b'\n')) => {}
                        _ => break,
                    }
                }
                _ => {}
            }
        }
        Err(Reject)
    }
    fn byte_string(input: Cursor) -> Result<Cursor, Reject> {
        if let Ok(input) = input.parse("b\"") {
            cooked_byte_string(input)
        } else if let Ok(input) = input.parse("br") {
            raw_byte_string(input)
        } else {
            Err(Reject)
        }
    }
    fn cooked_byte_string(mut input: Cursor) -> Result<Cursor, Reject> {
        let mut bytes = input.bytes().enumerate();
        while let Some((offset, b)) = bytes.next() {
            match b {
                b'"' => {
                    let input = input.advance(offset + 1);
                    return Ok(literal_suffix(input));
                }
                b'\r' => {
                    match bytes.next() {
                        Some((_, b'\n')) => {}
                        _ => break,
                    }
                }
                b'\\' => {
                    match bytes.next() {
                        Some((_, b'x')) => {
                            backslash_x_byte(&mut bytes)?;
                        }
                        Some((_, b'n' | b'r' | b't' | b'\\' | b'0' | b'\'' | b'"')) => {}
                        Some((newline, b @ (b'\n' | b'\r'))) => {
                            input = input.advance(newline + 1);
                            trailing_backslash(&mut input, b)?;
                            bytes = input.bytes().enumerate();
                        }
                        _ => break,
                    }
                }
                b if b.is_ascii() => {}
                _ => break,
            }
        }
        Err(Reject)
    }
    fn delimiter_of_raw_string(input: Cursor) -> PResult<&str> {
        for (i, byte) in input.bytes().enumerate() {
            match byte {
                b'"' => {
                    if i > 255 {
                        return Err(Reject);
                    }
                    return Ok((input.advance(i + 1), &input.rest[..i]));
                }
                b'#' => {}
                _ => break,
            }
        }
        Err(Reject)
    }
    fn raw_byte_string(input: Cursor) -> Result<Cursor, Reject> {
        let (input, delimiter) = delimiter_of_raw_string(input)?;
        let mut bytes = input.bytes().enumerate();
        while let Some((i, byte)) = bytes.next() {
            match byte {
                b'"' if input.rest[i + 1..].starts_with(delimiter) => {
                    let rest = input.advance(i + 1 + delimiter.len());
                    return Ok(literal_suffix(rest));
                }
                b'\r' => {
                    match bytes.next() {
                        Some((_, b'\n')) => {}
                        _ => break,
                    }
                }
                other => {
                    if !other.is_ascii() {
                        break;
                    }
                }
            }
        }
        Err(Reject)
    }
    fn c_string(input: Cursor) -> Result<Cursor, Reject> {
        if let Ok(input) = input.parse("c\"") {
            cooked_c_string(input)
        } else if let Ok(input) = input.parse("cr") {
            raw_c_string(input)
        } else {
            Err(Reject)
        }
    }
    fn raw_c_string(input: Cursor) -> Result<Cursor, Reject> {
        let (input, delimiter) = delimiter_of_raw_string(input)?;
        let mut bytes = input.bytes().enumerate();
        while let Some((i, byte)) = bytes.next() {
            match byte {
                b'"' if input.rest[i + 1..].starts_with(delimiter) => {
                    let rest = input.advance(i + 1 + delimiter.len());
                    return Ok(literal_suffix(rest));
                }
                b'\r' => {
                    match bytes.next() {
                        Some((_, b'\n')) => {}
                        _ => break,
                    }
                }
                b'\0' => break,
                _ => {}
            }
        }
        Err(Reject)
    }
    fn cooked_c_string(mut input: Cursor) -> Result<Cursor, Reject> {
        let mut chars = input.char_indices();
        while let Some((i, ch)) = chars.next() {
            match ch {
                '"' => {
                    let input = input.advance(i + 1);
                    return Ok(literal_suffix(input));
                }
                '\r' => {
                    match chars.next() {
                        Some((_, '\n')) => {}
                        _ => break,
                    }
                }
                '\\' => {
                    match chars.next() {
                        Some((_, 'x')) => {
                            backslash_x_nonzero(&mut chars)?;
                        }
                        Some((_, 'n' | 'r' | 't' | '\\' | '\'' | '"')) => {}
                        Some((_, 'u')) => {
                            if backslash_u(&mut chars)? == '\0' {
                                break;
                            }
                        }
                        Some((newline, ch @ ('\n' | '\r'))) => {
                            input = input.advance(newline + 1);
                            trailing_backslash(&mut input, ch as u8)?;
                            chars = input.char_indices();
                        }
                        _ => break,
                    }
                }
                '\0' => break,
                _ch => {}
            }
        }
        Err(Reject)
    }
    fn byte(input: Cursor) -> Result<Cursor, Reject> {
        let input = input.parse("b'")?;
        let mut bytes = input.bytes().enumerate();
        let ok = match bytes.next().map(|(_, b)| b) {
            Some(b'\\') => {
                match bytes.next().map(|(_, b)| b) {
                    Some(b'x') => backslash_x_byte(&mut bytes).is_ok(),
                    Some(b'n' | b'r' | b't' | b'\\' | b'0' | b'\'' | b'"') => true,
                    _ => false,
                }
            }
            b => b.is_some(),
        };
        if !ok {
            return Err(Reject);
        }
        let (offset, _) = bytes.next().ok_or(Reject)?;
        if !input.chars().as_str().is_char_boundary(offset) {
            return Err(Reject);
        }
        let input = input.advance(offset).parse("'")?;
        Ok(literal_suffix(input))
    }
    fn character(input: Cursor) -> Result<Cursor, Reject> {
        let input = input.parse("'")?;
        let mut chars = input.char_indices();
        let ok = match chars.next().map(|(_, ch)| ch) {
            Some('\\') => {
                match chars.next().map(|(_, ch)| ch) {
                    Some('x') => backslash_x_char(&mut chars).is_ok(),
                    Some('u') => backslash_u(&mut chars).is_ok(),
                    Some('n' | 'r' | 't' | '\\' | '0' | '\'' | '"') => true,
                    _ => false,
                }
            }
            ch => ch.is_some(),
        };
        if !ok {
            return Err(Reject);
        }
        let (idx, _) = chars.next().ok_or(Reject)?;
        let input = input.advance(idx).parse("'")?;
        Ok(literal_suffix(input))
    }
    fn backslash_x_char<I>(chars: &mut I) -> Result<(), Reject>
    where
        I: Iterator<Item = (usize, char)>,
    {
        match chars.next() {
            Some((_, ch)) => {
                match ch {
                    '0'..='7' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        match chars.next() {
            Some((_, ch)) => {
                match ch {
                    '0'..='9' | 'a'..='f' | 'A'..='F' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        Ok(())
    }
    fn backslash_x_byte<I>(chars: &mut I) -> Result<(), Reject>
    where
        I: Iterator<Item = (usize, u8)>,
    {
        match chars.next() {
            Some((_, ch)) => {
                match ch {
                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        match chars.next() {
            Some((_, ch)) => {
                match ch {
                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        Ok(())
    }
    fn backslash_x_nonzero<I>(chars: &mut I) -> Result<(), Reject>
    where
        I: Iterator<Item = (usize, char)>,
    {
        let first = match chars.next() {
            Some((_, ch)) => {
                match ch {
                    '0'..='9' | 'a'..='f' | 'A'..='F' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        let second = match chars.next() {
            Some((_, ch)) => {
                match ch {
                    '0'..='9' | 'a'..='f' | 'A'..='F' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        if first == '0' && second == '0' { Err(Reject) } else { Ok(()) }
    }
    fn backslash_u<I>(chars: &mut I) -> Result<char, Reject>
    where
        I: Iterator<Item = (usize, char)>,
    {
        match chars.next() {
            Some((_, ch)) => {
                match ch {
                    '{' => ch,
                    _ => return Err(Reject),
                }
            }
            None => return Err(Reject),
        };
        let mut value = 0;
        let mut len = 0;
        for (_, ch) in chars {
            let digit = match ch {
                '0'..='9' => ch as u8 - b'0',
                'a'..='f' => 10 + ch as u8 - b'a',
                'A'..='F' => 10 + ch as u8 - b'A',
                '_' if len > 0 => continue,
                '}' if len > 0 => return char::from_u32(value).ok_or(Reject),
                _ => break,
            };
            if len == 6 {
                break;
            }
            value *= 0x10;
            value += u32::from(digit);
            len += 1;
        }
        Err(Reject)
    }
    fn trailing_backslash(input: &mut Cursor, mut last: u8) -> Result<(), Reject> {
        let mut whitespace = input.bytes().enumerate();
        loop {
            if last == b'\r' && whitespace.next().map_or(true, |(_, b)| b != b'\n') {
                return Err(Reject);
            }
            match whitespace.next() {
                Some((_, b @ (b' ' | b'\t' | b'\n' | b'\r'))) => {
                    last = b;
                }
                Some((offset, _)) => {
                    *input = input.advance(offset);
                    return Ok(());
                }
                None => return Err(Reject),
            }
        }
    }
    fn float(input: Cursor) -> Result<Cursor, Reject> {
        let mut rest = float_digits(input)?;
        if let Some(ch) = rest.chars().next() {
            if is_ident_start(ch) {
                rest = ident_not_raw(rest)?.0;
            }
        }
        word_break(rest)
    }
    fn float_digits(input: Cursor) -> Result<Cursor, Reject> {
        let mut chars = input.chars().peekable();
        match chars.next() {
            Some(ch) if '0' <= ch && ch <= '9' => {}
            _ => return Err(Reject),
        }
        let mut len = 1;
        let mut has_dot = false;
        let mut has_exp = false;
        while let Some(&ch) = chars.peek() {
            match ch {
                '0'..='9' | '_' => {
                    chars.next();
                    len += 1;
                }
                '.' => {
                    if has_dot {
                        break;
                    }
                    chars.next();
                    if chars.peek().map_or(false, |&ch| ch == '.' || is_ident_start(ch))
                    {
                        return Err(Reject);
                    }
                    len += 1;
                    has_dot = true;
                }
                'e' | 'E' => {
                    chars.next();
                    len += 1;
                    has_exp = true;
                    break;
                }
                _ => break,
            }
        }
        if !(has_dot || has_exp) {
            return Err(Reject);
        }
        if has_exp {
            let token_before_exp = if has_dot {
                Ok(input.advance(len - 1))
            } else {
                Err(Reject)
            };
            let mut has_sign = false;
            let mut has_exp_value = false;
            while let Some(&ch) = chars.peek() {
                match ch {
                    '+' | '-' => {
                        if has_exp_value {
                            break;
                        }
                        if has_sign {
                            return token_before_exp;
                        }
                        chars.next();
                        len += 1;
                        has_sign = true;
                    }
                    '0'..='9' => {
                        chars.next();
                        len += 1;
                        has_exp_value = true;
                    }
                    '_' => {
                        chars.next();
                        len += 1;
                    }
                    _ => break,
                }
            }
            if !has_exp_value {
                return token_before_exp;
            }
        }
        Ok(input.advance(len))
    }
    fn int(input: Cursor) -> Result<Cursor, Reject> {
        let mut rest = digits(input)?;
        if let Some(ch) = rest.chars().next() {
            if is_ident_start(ch) {
                rest = ident_not_raw(rest)?.0;
            }
        }
        word_break(rest)
    }
    fn digits(mut input: Cursor) -> Result<Cursor, Reject> {
        let base = if input.starts_with("0x") {
            input = input.advance(2);
            16
        } else if input.starts_with("0o") {
            input = input.advance(2);
            8
        } else if input.starts_with("0b") {
            input = input.advance(2);
            2
        } else {
            10
        };
        let mut len = 0;
        let mut empty = true;
        for b in input.bytes() {
            match b {
                b'0'..=b'9' => {
                    let digit = (b - b'0') as u64;
                    if digit >= base {
                        return Err(Reject);
                    }
                }
                b'a'..=b'f' => {
                    let digit = 10 + (b - b'a') as u64;
                    if digit >= base {
                        break;
                    }
                }
                b'A'..=b'F' => {
                    let digit = 10 + (b - b'A') as u64;
                    if digit >= base {
                        break;
                    }
                }
                b'_' => {
                    if empty && base == 10 {
                        return Err(Reject);
                    }
                    len += 1;
                    continue;
                }
                _ => break,
            }
            len += 1;
            empty = false;
        }
        if empty { Err(Reject) } else { Ok(input.advance(len)) }
    }
    fn punct(input: Cursor) -> PResult<Punct> {
        let (rest, ch) = punct_char(input)?;
        if ch == '\'' {
            let (after_lifetime, _ident) = ident_any(rest)?;
            if after_lifetime.starts_with_char('\'')
                || (after_lifetime.starts_with_char('#') && !rest.starts_with("r#"))
            {
                Err(Reject)
            } else {
                Ok((rest, Punct::new('\'', Spacing::Joint)))
            }
        } else {
            let kind = match punct_char(rest) {
                Ok(_) => Spacing::Joint,
                Err(Reject) => Spacing::Alone,
            };
            Ok((rest, Punct::new(ch, kind)))
        }
    }
    fn punct_char(input: Cursor) -> PResult<char> {
        if input.starts_with("//") || input.starts_with("/*") {
            return Err(Reject);
        }
        let mut chars = input.chars();
        let Some(first) = chars.next() else {
            return Err(Reject);
        };
        let recognized = "~!@#$%^&*-=+|;:,<.>/?'";
        if recognized.contains(first) {
            Ok((input.advance(first.len_utf8()), first))
        } else {
            Err(Reject)
        }
    }
    fn doc_comment<'a>(
        input: Cursor<'a>,
        tokens: &mut TokenStreamBuilder,
    ) -> PResult<'a, ()> {
        let (rest, (comment, inner)) = doc_comment_contents(input)?;
        let fallback_span = Span {};
        let span = crate::Span::_new_fallback(fallback_span);
        let mut scan_for_bare_cr = comment;
        while let Some(cr) = scan_for_bare_cr.find('\r') {
            let rest = &scan_for_bare_cr[cr + 1..];
            if !rest.starts_with('\n') {
                return Err(Reject);
            }
            scan_for_bare_cr = rest;
        }
        let mut pound = Punct::new('#', Spacing::Alone);
        pound.set_span(span);
        tokens.push_token_from_parser(TokenTree::Punct(pound));
        if inner {
            let mut bang = Punct::new('!', Spacing::Alone);
            bang.set_span(span);
            tokens.push_token_from_parser(TokenTree::Punct(bang));
        }
        let doc_ident = crate::Ident::_new_fallback(
            Ident::new_unchecked("doc", fallback_span),
        );
        let mut equal = Punct::new('=', Spacing::Alone);
        equal.set_span(span);
        let mut literal = crate::Literal::_new_fallback(Literal::string(comment));
        literal.set_span(span);
        let mut bracketed = TokenStreamBuilder::with_capacity(3);
        bracketed.push_token_from_parser(TokenTree::Ident(doc_ident));
        bracketed.push_token_from_parser(TokenTree::Punct(equal));
        bracketed.push_token_from_parser(TokenTree::Literal(literal));
        let group = Group::new(Delimiter::Bracket, bracketed.build());
        let mut group = crate::Group::_new_fallback(group);
        group.set_span(span);
        tokens.push_token_from_parser(TokenTree::Group(group));
        Ok((rest, ()))
    }
    fn doc_comment_contents(input: Cursor) -> PResult<(&str, bool)> {
        if input.starts_with("//!") {
            let input = input.advance(3);
            let (input, s) = take_until_newline_or_eof(input);
            Ok((input, (s, true)))
        } else if input.starts_with("/*!") {
            let (input, s) = block_comment(input)?;
            Ok((input, (&s[3..s.len() - 2], true)))
        } else if input.starts_with("///") {
            let input = input.advance(3);
            if input.starts_with_char('/') {
                return Err(Reject);
            }
            let (input, s) = take_until_newline_or_eof(input);
            Ok((input, (s, false)))
        } else if input.starts_with("/**") && !input.rest[3..].starts_with('*') {
            let (input, s) = block_comment(input)?;
            Ok((input, (&s[3..s.len() - 2], false)))
        } else {
            Err(Reject)
        }
    }
    fn take_until_newline_or_eof(input: Cursor) -> (Cursor, &str) {
        let chars = input.char_indices();
        for (i, ch) in chars {
            if ch == '\n' {
                return (input.advance(i), &input.rest[..i]);
            } else if ch == '\r' && input.rest[i + 1..].starts_with('\n') {
                return (input.advance(i + 1), &input.rest[..i]);
            }
        }
        (input.advance(input.len()), input.rest)
    }
}
mod probe {
    #![allow(dead_code)]
    pub(crate) mod proc_macro_span {
        extern crate alloc;
        extern crate proc_macro;
        extern crate std;
        use alloc::string::String;
        use core::ops::{Range, RangeBounds};
        use proc_macro::{Literal, Span};
        use std::path::PathBuf;
        pub fn byte_range(this: &Span) -> Range<usize> {
            this.byte_range()
        }
        pub fn start(this: &Span) -> Span {
            this.start()
        }
        pub fn end(this: &Span) -> Span {
            this.end()
        }
        pub fn line(this: &Span) -> usize {
            this.line()
        }
        pub fn column(this: &Span) -> usize {
            this.column()
        }
        pub fn file(this: &Span) -> String {
            this.file()
        }
        pub fn local_file(this: &Span) -> Option<PathBuf> {
            this.local_file()
        }
        pub fn join(this: &Span, other: Span) -> Option<Span> {
            this.join(other)
        }
        pub fn subspan<R: RangeBounds<usize>>(this: &Literal, range: R) -> Option<Span> {
            this.subspan(range)
        }
    }
    pub(crate) mod proc_macro_span_file {
        extern crate alloc;
        extern crate proc_macro;
        extern crate std;
        use alloc::string::String;
        use proc_macro::Span;
        use std::path::PathBuf;
        pub fn file(this: &Span) -> String {
            this.file()
        }
        pub fn local_file(this: &Span) -> Option<PathBuf> {
            this.local_file()
        }
    }
    pub(crate) mod proc_macro_span_location {
        extern crate alloc;
        extern crate proc_macro;
        extern crate std;
        use proc_macro::Span;
        pub fn start(this: &Span) -> Span {
            this.start()
        }
        pub fn end(this: &Span) -> Span {
            this.end()
        }
        pub fn line(this: &Span) -> usize {
            this.line()
        }
        pub fn column(this: &Span) -> usize {
            this.column()
        }
    }
}
mod rcvec {
    use alloc::rc::Rc;
    use alloc::vec::{self, Vec};
    use core::mem;
    use core::panic::RefUnwindSafe;
    use core::slice;
    pub(crate) struct RcVec<T> {
        inner: Rc<Vec<T>>,
    }
    pub(crate) struct RcVecBuilder<T> {
        inner: Vec<T>,
    }
    pub(crate) struct RcVecMut<'a, T> {
        inner: &'a mut Vec<T>,
    }
    pub(crate) struct RcVecIntoIter<T> {
        inner: vec::IntoIter<T>,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for RcVecIntoIter<T> {
        #[inline]
        fn clone(&self) -> RcVecIntoIter<T> {
            RcVecIntoIter {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    impl<T> RcVec<T> {
        pub(crate) fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }
        pub(crate) fn len(&self) -> usize {
            self.inner.len()
        }
        pub(crate) fn iter(&self) -> slice::Iter<T> {
            self.inner.iter()
        }
        pub(crate) fn make_mut(&mut self) -> RcVecMut<T>
        where
            T: Clone,
        {
            RcVecMut {
                inner: Rc::make_mut(&mut self.inner),
            }
        }
        pub(crate) fn get_mut(&mut self) -> Option<RcVecMut<T>> {
            let inner = Rc::get_mut(&mut self.inner)?;
            Some(RcVecMut { inner })
        }
        pub(crate) fn make_owned(mut self) -> RcVecBuilder<T>
        where
            T: Clone,
        {
            let vec = if let Some(owned) = Rc::get_mut(&mut self.inner) {
                mem::take(owned)
            } else {
                Vec::clone(&self.inner)
            };
            RcVecBuilder { inner: vec }
        }
    }
    impl<T> RcVecBuilder<T> {
        pub(crate) fn new() -> Self {
            RcVecBuilder { inner: Vec::new() }
        }
        pub(crate) fn with_capacity(cap: usize) -> Self {
            RcVecBuilder {
                inner: Vec::with_capacity(cap),
            }
        }
        pub(crate) fn push(&mut self, element: T) {
            self.inner.push(element);
        }
        pub(crate) fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
            self.inner.extend(iter);
        }
        pub(crate) fn as_mut(&mut self) -> RcVecMut<T> {
            RcVecMut { inner: &mut self.inner }
        }
        pub(crate) fn build(self) -> RcVec<T> {
            RcVec {
                inner: Rc::new(self.inner),
            }
        }
    }
    impl<'a, T> RcVecMut<'a, T> {
        pub(crate) fn push(&mut self, element: T) {
            self.inner.push(element);
        }
        pub(crate) fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
            self.inner.extend(iter);
        }
        pub(crate) fn as_mut(&mut self) -> RcVecMut<T> {
            RcVecMut { inner: self.inner }
        }
        pub(crate) fn take(self) -> RcVecBuilder<T> {
            let vec = mem::take(self.inner);
            RcVecBuilder { inner: vec }
        }
    }
    impl<T> Clone for RcVec<T> {
        fn clone(&self) -> Self {
            RcVec {
                inner: Rc::clone(&self.inner),
            }
        }
    }
    impl<T> IntoIterator for RcVecBuilder<T> {
        type Item = T;
        type IntoIter = RcVecIntoIter<T>;
        fn into_iter(self) -> Self::IntoIter {
            RcVecIntoIter {
                inner: self.inner.into_iter(),
            }
        }
    }
    impl<T> Iterator for RcVecIntoIter<T> {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }
    impl<T> RefUnwindSafe for RcVec<T>
    where
        T: RefUnwindSafe,
    {}
}
mod detection {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Once;
    static WORKS: AtomicUsize = AtomicUsize::new(0);
    static INIT: Once = Once::new();
    pub(crate) fn inside_proc_macro() -> bool {
        match WORKS.load(Ordering::Relaxed) {
            1 => return false,
            2 => return true,
            _ => {}
        }
        INIT.call_once(initialize);
        inside_proc_macro()
    }
    pub(crate) fn force_fallback() {
        WORKS.store(1, Ordering::Relaxed);
    }
    pub(crate) fn unforce_fallback() {
        initialize();
    }
    fn initialize() {
        let available = proc_macro::is_available();
        WORKS.store(available as usize + 1, Ordering::Relaxed);
    }
}
#[doc(hidden)]
pub mod fallback {
    use crate::imp;
    use crate::parse::{self, Cursor};
    use crate::rcvec::{RcVec, RcVecBuilder, RcVecIntoIter, RcVecMut};
    use crate::{Delimiter, Spacing, TokenTree};
    use alloc::borrow::ToOwned as _;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::{String, ToString as _};
    use alloc::vec::Vec;
    use core::ffi::CStr;
    use core::fmt::{self, Debug, Display, Write};
    use core::mem::ManuallyDrop;
    use core::ops::RangeBounds;
    use core::ptr;
    use core::str;
    use core::str::FromStr;
    use std::panic;
    /// Force use of proc-macro2's fallback implementation of the API for now, even
    /// if the compiler's implementation is available.
    pub fn force() {
        crate::detection::force_fallback();
    }
    /// Resume using the compiler's implementation of the proc macro API if it is
    /// available.
    pub fn unforce() {
        crate::detection::unforce_fallback();
    }
    pub(crate) struct TokenStream {
        inner: RcVec<TokenTree>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TokenStream {
        #[inline]
        fn clone(&self) -> TokenStream {
            TokenStream {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    pub(crate) struct LexError {
        pub(crate) span: Span,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for LexError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "LexError",
                "span",
                &&self.span,
            )
        }
    }
    impl LexError {
        pub(crate) fn span(&self) -> Span {
            self.span
        }
        pub(crate) fn call_site() -> Self {
            LexError {
                span: Span::call_site(),
            }
        }
    }
    impl TokenStream {
        pub(crate) fn new() -> Self {
            TokenStream {
                inner: RcVecBuilder::new().build(),
            }
        }
        pub(crate) fn from_str_checked(src: &str) -> Result<Self, LexError> {
            let mut cursor = get_cursor(src);
            const BYTE_ORDER_MARK: &str = "\u{feff}";
            if cursor.starts_with(BYTE_ORDER_MARK) {
                cursor = cursor.advance(BYTE_ORDER_MARK.len());
            }
            parse::token_stream(cursor)
        }
        pub(crate) fn from_str_unchecked(src: &str) -> Self {
            Self::from_str_checked(src).unwrap()
        }
        pub(crate) fn is_empty(&self) -> bool {
            self.inner.len() == 0
        }
        fn take_inner(self) -> RcVecBuilder<TokenTree> {
            let nodrop = ManuallyDrop::new(self);
            unsafe { ptr::read(&nodrop.inner) }.make_owned()
        }
    }
    fn push_token_from_proc_macro(mut vec: RcVecMut<TokenTree>, token: TokenTree) {
        match token {
            TokenTree::Literal(
                crate::Literal { inner: crate::imp::Literal::Fallback(literal), .. },
            ) if literal.repr.starts_with('-') => {
                push_negative_literal(vec, literal);
            }
            _ => vec.push(token),
        }
        #[cold]
        fn push_negative_literal(mut vec: RcVecMut<TokenTree>, mut literal: Literal) {
            literal.repr.remove(0);
            let mut punct = crate::Punct::new('-', Spacing::Alone);
            punct.set_span(crate::Span::_new_fallback(literal.span));
            vec.push(TokenTree::Punct(punct));
            vec.push(TokenTree::Literal(crate::Literal::_new_fallback(literal)));
        }
    }
    impl Drop for TokenStream {
        fn drop(&mut self) {
            let mut stack = Vec::new();
            let mut current = match self.inner.get_mut() {
                Some(inner) => inner.take().into_iter(),
                None => return,
            };
            loop {
                while let Some(token) = current.next() {
                    let group = match token {
                        TokenTree::Group(group) => group.inner,
                        _ => continue,
                    };
                    let group = match group {
                        crate::imp::Group::Fallback(group) => group,
                        crate::imp::Group::Compiler(_) => continue,
                    };
                    let mut group = group;
                    if let Some(inner) = group.stream.inner.get_mut() {
                        stack.push(current);
                        current = inner.take().into_iter();
                    }
                }
                match stack.pop() {
                    Some(next) => current = next,
                    None => return,
                }
            }
        }
    }
    pub(crate) struct TokenStreamBuilder {
        inner: RcVecBuilder<TokenTree>,
    }
    impl TokenStreamBuilder {
        pub(crate) fn new() -> Self {
            TokenStreamBuilder {
                inner: RcVecBuilder::new(),
            }
        }
        pub(crate) fn with_capacity(cap: usize) -> Self {
            TokenStreamBuilder {
                inner: RcVecBuilder::with_capacity(cap),
            }
        }
        pub(crate) fn push_token_from_parser(&mut self, tt: TokenTree) {
            self.inner.push(tt);
        }
        pub(crate) fn build(self) -> TokenStream {
            TokenStream {
                inner: self.inner.build(),
            }
        }
    }
    fn get_cursor(src: &str) -> Cursor {
        Cursor { rest: src }
    }
    impl Display for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("cannot parse string into token stream")
        }
    }
    impl Display for TokenStream {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut joint = false;
            for (i, tt) in self.inner.iter().enumerate() {
                if i != 0 && !joint {
                    f.write_fmt(format_args!(" "))?;
                }
                joint = false;
                match tt {
                    TokenTree::Group(tt) => f.write_fmt(format_args!("{0}", tt)),
                    TokenTree::Ident(tt) => f.write_fmt(format_args!("{0}", tt)),
                    TokenTree::Punct(tt) => {
                        joint = tt.spacing() == Spacing::Joint;
                        f.write_fmt(format_args!("{0}", tt))
                    }
                    TokenTree::Literal(tt) => f.write_fmt(format_args!("{0}", tt)),
                }?;
            }
            Ok(())
        }
    }
    impl Debug for TokenStream {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("TokenStream ")?;
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl From<proc_macro::TokenStream> for TokenStream {
        fn from(inner: proc_macro::TokenStream) -> Self {
            TokenStream::from_str_unchecked(&inner.to_string())
        }
    }
    impl From<TokenStream> for proc_macro::TokenStream {
        fn from(inner: TokenStream) -> Self {
            proc_macro::TokenStream::from_str_unchecked(&inner.to_string())
        }
    }
    impl From<TokenTree> for TokenStream {
        fn from(tree: TokenTree) -> Self {
            let mut stream = RcVecBuilder::new();
            push_token_from_proc_macro(stream.as_mut(), tree);
            TokenStream {
                inner: stream.build(),
            }
        }
    }
    impl FromIterator<TokenTree> for TokenStream {
        fn from_iter<I: IntoIterator<Item = TokenTree>>(tokens: I) -> Self {
            let mut stream = TokenStream::new();
            stream.extend(tokens);
            stream
        }
    }
    impl FromIterator<TokenStream> for TokenStream {
        fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
            let mut v = RcVecBuilder::new();
            for stream in streams {
                v.extend(stream.take_inner());
            }
            TokenStream { inner: v.build() }
        }
    }
    impl Extend<TokenTree> for TokenStream {
        fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, tokens: I) {
            let mut vec = self.inner.make_mut();
            tokens
                .into_iter()
                .for_each(|token| push_token_from_proc_macro(vec.as_mut(), token));
        }
    }
    impl Extend<TokenStream> for TokenStream {
        fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
            self.inner.make_mut().extend(streams.into_iter().flatten());
        }
    }
    pub(crate) type TokenTreeIter = RcVecIntoIter<TokenTree>;
    impl IntoIterator for TokenStream {
        type Item = TokenTree;
        type IntoIter = TokenTreeIter;
        fn into_iter(self) -> TokenTreeIter {
            self.take_inner().into_iter()
        }
    }
    pub(crate) struct Span {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for Span {}
    #[automatically_derived]
    impl ::core::clone::Clone for Span {
        #[inline]
        fn clone(&self) -> Span {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Span {}
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Span {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Span {
        #[inline]
        fn eq(&self, other: &Span) -> bool {
            true
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for Span {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
    }
    impl Span {
        pub(crate) fn call_site() -> Self {
            Span {}
        }
        pub(crate) fn mixed_site() -> Self {
            Span::call_site()
        }
        pub(crate) fn resolved_at(&self, _other: Span) -> Span {
            *self
        }
        pub(crate) fn located_at(&self, other: Span) -> Span {
            other
        }
        pub(crate) fn join(&self, _other: Span) -> Option<Span> {
            Some(Span {})
        }
        pub(crate) fn source_text(&self) -> Option<String> {
            None
        }
        pub(crate) fn first_byte(self) -> Self {
            self
        }
        pub(crate) fn last_byte(self) -> Self {
            self
        }
    }
    impl Debug for Span {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(format_args!("Span"))
        }
    }
    pub(crate) fn debug_span_field_if_nontrivial(
        debug: &mut fmt::DebugStruct,
        span: Span,
    ) {
        if false {
            debug.field("span", &span);
        }
    }
    pub(crate) struct Group {
        delimiter: Delimiter,
        stream: TokenStream,
        span: Span,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Group {
        #[inline]
        fn clone(&self) -> Group {
            Group {
                delimiter: ::core::clone::Clone::clone(&self.delimiter),
                stream: ::core::clone::Clone::clone(&self.stream),
                span: ::core::clone::Clone::clone(&self.span),
            }
        }
    }
    impl Group {
        pub(crate) fn new(delimiter: Delimiter, stream: TokenStream) -> Self {
            Group {
                delimiter,
                stream,
                span: Span::call_site(),
            }
        }
        pub(crate) fn delimiter(&self) -> Delimiter {
            self.delimiter
        }
        pub(crate) fn stream(&self) -> TokenStream {
            self.stream.clone()
        }
        pub(crate) fn span(&self) -> Span {
            self.span
        }
        pub(crate) fn span_open(&self) -> Span {
            self.span.first_byte()
        }
        pub(crate) fn span_close(&self) -> Span {
            self.span.last_byte()
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            self.span = span;
        }
    }
    impl Display for Group {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let (open, close) = match self.delimiter {
                Delimiter::Parenthesis => ("(", ")"),
                Delimiter::Brace => ("{ ", "}"),
                Delimiter::Bracket => ("[", "]"),
                Delimiter::None => ("", ""),
            };
            f.write_str(open)?;
            Display::fmt(&self.stream, f)?;
            if self.delimiter == Delimiter::Brace && !self.stream.inner.is_empty() {
                f.write_str(" ")?;
            }
            f.write_str(close)?;
            Ok(())
        }
    }
    impl Debug for Group {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            let mut debug = fmt.debug_struct("Group");
            debug.field("delimiter", &self.delimiter);
            debug.field("stream", &self.stream);
            debug_span_field_if_nontrivial(&mut debug, self.span);
            debug.finish()
        }
    }
    pub(crate) struct Ident {
        sym: Box<str>,
        span: Span,
        raw: bool,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Ident {
        #[inline]
        fn clone(&self) -> Ident {
            Ident {
                sym: ::core::clone::Clone::clone(&self.sym),
                span: ::core::clone::Clone::clone(&self.span),
                raw: ::core::clone::Clone::clone(&self.raw),
            }
        }
    }
    impl Ident {
        #[track_caller]
        pub(crate) fn new_checked(string: &str, span: Span) -> Self {
            validate_ident(string);
            Ident::new_unchecked(string, span)
        }
        pub(crate) fn new_unchecked(string: &str, span: Span) -> Self {
            Ident {
                sym: Box::from(string),
                span,
                raw: false,
            }
        }
        #[track_caller]
        pub(crate) fn new_raw_checked(string: &str, span: Span) -> Self {
            validate_ident_raw(string);
            Ident::new_raw_unchecked(string, span)
        }
        pub(crate) fn new_raw_unchecked(string: &str, span: Span) -> Self {
            Ident {
                sym: Box::from(string),
                span,
                raw: true,
            }
        }
        pub(crate) fn span(&self) -> Span {
            self.span
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            self.span = span;
        }
    }
    pub(crate) fn is_ident_start(c: char) -> bool {
        c == '_' || unicode_ident::is_xid_start(c)
    }
    pub(crate) fn is_ident_continue(c: char) -> bool {
        unicode_ident::is_xid_continue(c)
    }
    #[track_caller]
    fn validate_ident(string: &str) {
        if string.is_empty() {
            {
                ::core::panicking::panic_fmt(
                    format_args!("Ident is not allowed to be empty; use Option<Ident>"),
                );
            };
        }
        if string.bytes().all(|digit| b'0' <= digit && digit <= b'9') {
            {
                ::core::panicking::panic_fmt(
                    format_args!("Ident cannot be a number; use Literal instead"),
                );
            };
        }
        fn ident_ok(string: &str) -> bool {
            let mut chars = string.chars();
            let first = chars.next().unwrap();
            if !is_ident_start(first) {
                return false;
            }
            for ch in chars {
                if !is_ident_continue(ch) {
                    return false;
                }
            }
            true
        }
        if !ident_ok(string) {
            {
                ::core::panicking::panic_fmt(
                    format_args!("{0:?} is not a valid Ident", string),
                );
            };
        }
    }
    #[track_caller]
    fn validate_ident_raw(string: &str) {
        validate_ident(string);
        match string {
            "_" | "super" | "self" | "Self" | "crate" => {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("`r#{0}` cannot be a raw identifier", string),
                    );
                };
            }
            _ => {}
        }
    }
    impl PartialEq for Ident {
        fn eq(&self, other: &Ident) -> bool {
            self.sym == other.sym && self.raw == other.raw
        }
    }
    impl<T> PartialEq<T> for Ident
    where
        T: ?Sized + AsRef<str>,
    {
        fn eq(&self, other: &T) -> bool {
            let other = other.as_ref();
            if self.raw {
                other.starts_with("r#") && *self.sym == other[2..]
            } else {
                *self.sym == *other
            }
        }
    }
    impl Display for Ident {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if self.raw {
                f.write_str("r#")?;
            }
            f.write_str(&self.sym)
        }
    }
    #[allow(clippy::missing_fields_in_debug)]
    impl Debug for Ident {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let mut debug = f.debug_tuple("Ident");
            debug.field(&format_args!("{0}", self));
            debug.finish()
        }
    }
    pub(crate) struct Literal {
        pub(crate) repr: String,
        span: Span,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Literal {
        #[inline]
        fn clone(&self) -> Literal {
            Literal {
                repr: ::core::clone::Clone::clone(&self.repr),
                span: ::core::clone::Clone::clone(&self.span),
            }
        }
    }
    impl Literal {
        pub(crate) fn _new(repr: String) -> Self {
            Literal {
                repr,
                span: Span::call_site(),
            }
        }
        pub(crate) fn from_str_checked(repr: &str) -> Result<Self, LexError> {
            let mut cursor = get_cursor(repr);
            let negative = cursor.starts_with_char('-');
            if negative {
                cursor = cursor.advance(1);
                if !cursor.starts_with_fn(|ch| ch.is_ascii_digit()) {
                    return Err(LexError::call_site());
                }
            }
            if let Ok((rest, mut literal)) = parse::literal(cursor) {
                if rest.is_empty() {
                    if negative {
                        literal.repr.insert(0, '-');
                    }
                    literal.span = Span {};
                    return Ok(literal);
                }
            }
            Err(LexError::call_site())
        }
        pub(crate) unsafe fn from_str_unchecked(repr: &str) -> Self {
            Literal::_new(repr.to_owned())
        }
        pub(crate) fn u8_suffixed(n: u8) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}u8", n))
                }),
            )
        }
        pub(crate) fn u16_suffixed(n: u16) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}u16", n))
                }),
            )
        }
        pub(crate) fn u32_suffixed(n: u32) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}u32", n))
                }),
            )
        }
        pub(crate) fn u64_suffixed(n: u64) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}u64", n))
                }),
            )
        }
        pub(crate) fn u128_suffixed(n: u128) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}u128", n))
                }),
            )
        }
        pub(crate) fn usize_suffixed(n: usize) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}usize", n))
                }),
            )
        }
        pub(crate) fn i8_suffixed(n: i8) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}i8", n))
                }),
            )
        }
        pub(crate) fn i16_suffixed(n: i16) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}i16", n))
                }),
            )
        }
        pub(crate) fn i32_suffixed(n: i32) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}i32", n))
                }),
            )
        }
        pub(crate) fn i64_suffixed(n: i64) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}i64", n))
                }),
            )
        }
        pub(crate) fn i128_suffixed(n: i128) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}i128", n))
                }),
            )
        }
        pub(crate) fn isize_suffixed(n: isize) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}isize", n))
                }),
            )
        }
        pub(crate) fn f32_suffixed(n: f32) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}f32", n))
                }),
            )
        }
        pub(crate) fn f64_suffixed(n: f64) -> Literal {
            Literal::_new(
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("{0}f64", n))
                }),
            )
        }
        pub(crate) fn u8_unsuffixed(n: u8) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn u16_unsuffixed(n: u16) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn u32_unsuffixed(n: u32) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn u64_unsuffixed(n: u64) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn u128_unsuffixed(n: u128) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn usize_unsuffixed(n: usize) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn i8_unsuffixed(n: i8) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn i16_unsuffixed(n: i16) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn i32_unsuffixed(n: i32) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn i64_unsuffixed(n: i64) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn i128_unsuffixed(n: i128) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn isize_unsuffixed(n: isize) -> Literal {
            Literal::_new(n.to_string())
        }
        pub(crate) fn f32_unsuffixed(f: f32) -> Literal {
            let mut s = f.to_string();
            if !s.contains('.') {
                s.push_str(".0");
            }
            Literal::_new(s)
        }
        pub(crate) fn f64_unsuffixed(f: f64) -> Literal {
            let mut s = f.to_string();
            if !s.contains('.') {
                s.push_str(".0");
            }
            Literal::_new(s)
        }
        pub(crate) fn string(string: &str) -> Literal {
            let mut repr = String::with_capacity(string.len() + 2);
            repr.push('"');
            escape_utf8(string, &mut repr);
            repr.push('"');
            Literal::_new(repr)
        }
        pub(crate) fn character(ch: char) -> Literal {
            let mut repr = String::new();
            repr.push('\'');
            if ch == '"' {
                repr.push(ch);
            } else {
                repr.extend(ch.escape_debug());
            }
            repr.push('\'');
            Literal::_new(repr)
        }
        pub(crate) fn byte_character(byte: u8) -> Literal {
            let mut repr = "b'".to_string();
            #[allow(clippy::match_overlapping_arm)]
            match byte {
                b'\0' => repr.push_str(r"\0"),
                b'\t' => repr.push_str(r"\t"),
                b'\n' => repr.push_str(r"\n"),
                b'\r' => repr.push_str(r"\r"),
                b'\'' => repr.push_str(r"\'"),
                b'\\' => repr.push_str(r"\\"),
                b'\x20'..=b'\x7E' => repr.push(byte as char),
                _ => {
                    let _ = repr.write_fmt(format_args!("\\x{0:02X}", byte));
                }
            }
            repr.push('\'');
            Literal::_new(repr)
        }
        pub(crate) fn byte_string(bytes: &[u8]) -> Literal {
            let mut repr = "b\"".to_string();
            let mut bytes = bytes.iter();
            while let Some(&b) = bytes.next() {
                #[allow(clippy::match_overlapping_arm)]
                match b {
                    b'\0' => {
                        repr.push_str(
                            match bytes.as_slice().first() {
                                Some(b'0'..=b'7') => r"\x00",
                                _ => r"\0",
                            },
                        )
                    }
                    b'\t' => repr.push_str(r"\t"),
                    b'\n' => repr.push_str(r"\n"),
                    b'\r' => repr.push_str(r"\r"),
                    b'"' => repr.push_str("\\\""),
                    b'\\' => repr.push_str(r"\\"),
                    b'\x20'..=b'\x7E' => repr.push(b as char),
                    _ => {
                        let _ = repr.write_fmt(format_args!("\\x{0:02X}", b));
                    }
                }
            }
            repr.push('"');
            Literal::_new(repr)
        }
        pub(crate) fn c_string(string: &CStr) -> Literal {
            let mut repr = "c\"".to_string();
            let mut bytes = string.to_bytes();
            while !bytes.is_empty() {
                let (valid, invalid) = match str::from_utf8(bytes) {
                    Ok(all_valid) => {
                        bytes = b"";
                        (all_valid, bytes)
                    }
                    Err(utf8_error) => {
                        let (valid, rest) = bytes.split_at(utf8_error.valid_up_to());
                        let valid = str::from_utf8(valid).unwrap();
                        let invalid = utf8_error
                            .error_len()
                            .map_or(rest, |error_len| &rest[..error_len]);
                        bytes = &bytes[valid.len() + invalid.len()..];
                        (valid, invalid)
                    }
                };
                escape_utf8(valid, &mut repr);
                for &byte in invalid {
                    let _ = repr.write_fmt(format_args!("\\x{0:02X}", byte));
                }
            }
            repr.push('"');
            Literal::_new(repr)
        }
        pub(crate) fn span(&self) -> Span {
            self.span
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            self.span = span;
        }
        pub(crate) fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Option<Span> {
            {
                let _ = range;
                None
            }
        }
    }
    impl Display for Literal {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            Display::fmt(&self.repr, f)
        }
    }
    impl Debug for Literal {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            let mut debug = fmt.debug_struct("Literal");
            debug.field("lit", &format_args!("{0}", self.repr));
            debug_span_field_if_nontrivial(&mut debug, self.span);
            debug.finish()
        }
    }
    fn escape_utf8(string: &str, repr: &mut String) {
        let mut chars = string.chars();
        while let Some(ch) = chars.next() {
            if ch == '\0' {
                repr.push_str(
                    if chars.as_str().starts_with(|next| '0' <= next && next <= '7') {
                        r"\x00"
                    } else {
                        r"\0"
                    },
                );
            } else if ch == '\'' {
                repr.push(ch);
            } else {
                repr.extend(ch.escape_debug());
            }
        }
    }
    pub(crate) trait FromStr2: FromStr<Err = proc_macro::LexError> {
        fn valid(src: &str) -> bool;
        fn from_str_checked(src: &str) -> Result<Self, imp::LexError> {
            if !Self::valid(src) {
                return Err(imp::LexError::CompilerPanic);
            }
            match panic::catch_unwind(|| Self::from_str(src)) {
                Ok(Ok(ok)) => Ok(ok),
                Ok(Err(lex)) => Err(imp::LexError::Compiler(lex)),
                Err(_panic) => Err(imp::LexError::CompilerPanic),
            }
        }
        fn from_str_unchecked(src: &str) -> Self {
            Self::from_str(src).unwrap()
        }
    }
    impl FromStr2 for proc_macro::TokenStream {
        fn valid(src: &str) -> bool {
            TokenStream::from_str_checked(src).is_ok()
        }
    }
    impl FromStr2 for proc_macro::Literal {
        fn valid(src: &str) -> bool {
            Literal::from_str_checked(src).is_ok()
        }
    }
}
pub mod extra {
    //! Items which do not have a correspondence to any API in the proc_macro crate,
    //! but are necessary to include in proc-macro2.
    use crate::fallback;
    use crate::imp;
    use crate::marker::{ProcMacroAutoTraits, MARKER};
    use crate::Span;
    use core::fmt::{self, Debug};
    /// An object that holds a [`Group`]'s `span_open()` and `span_close()` together
    /// in a more compact representation than holding those 2 spans individually.
    ///
    /// [`Group`]: crate::Group
    pub struct DelimSpan {
        inner: DelimSpanEnum,
        _marker: ProcMacroAutoTraits,
    }
    #[automatically_derived]
    impl ::core::marker::Copy for DelimSpan {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for DelimSpan {}
    #[automatically_derived]
    impl ::core::clone::Clone for DelimSpan {
        #[inline]
        fn clone(&self) -> DelimSpan {
            let _: ::core::clone::AssertParamIsClone<DelimSpanEnum>;
            let _: ::core::clone::AssertParamIsClone<ProcMacroAutoTraits>;
            *self
        }
    }
    enum DelimSpanEnum {
        Compiler {
            join: proc_macro::Span,
            open: proc_macro::Span,
            close: proc_macro::Span,
        },
        Fallback(fallback::Span),
    }
    #[automatically_derived]
    impl ::core::marker::Copy for DelimSpanEnum {}
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for DelimSpanEnum {}
    #[automatically_derived]
    impl ::core::clone::Clone for DelimSpanEnum {
        #[inline]
        fn clone(&self) -> DelimSpanEnum {
            let _: ::core::clone::AssertParamIsClone<proc_macro::Span>;
            let _: ::core::clone::AssertParamIsClone<proc_macro::Span>;
            let _: ::core::clone::AssertParamIsClone<proc_macro::Span>;
            let _: ::core::clone::AssertParamIsClone<fallback::Span>;
            *self
        }
    }
    impl DelimSpan {
        pub(crate) fn new(group: &imp::Group) -> Self {
            let inner = match group {
                imp::Group::Compiler(group) => {
                    DelimSpanEnum::Compiler {
                        join: group.span(),
                        open: group.span_open(),
                        close: group.span_close(),
                    }
                }
                imp::Group::Fallback(group) => DelimSpanEnum::Fallback(group.span()),
            };
            DelimSpan {
                inner,
                _marker: MARKER,
            }
        }
        /// Returns a span covering the entire delimited group.
        pub fn join(&self) -> Span {
            match &self.inner {
                DelimSpanEnum::Compiler { join, .. } => {
                    Span::_new(imp::Span::Compiler(*join))
                }
                DelimSpanEnum::Fallback(span) => Span::_new_fallback(*span),
            }
        }
        /// Returns a span for the opening punctuation of the group only.
        pub fn open(&self) -> Span {
            match &self.inner {
                DelimSpanEnum::Compiler { open, .. } => {
                    Span::_new(imp::Span::Compiler(*open))
                }
                DelimSpanEnum::Fallback(span) => Span::_new_fallback(span.first_byte()),
            }
        }
        /// Returns a span for the closing punctuation of the group only.
        pub fn close(&self) -> Span {
            match &self.inner {
                DelimSpanEnum::Compiler { close, .. } => {
                    Span::_new(imp::Span::Compiler(*close))
                }
                DelimSpanEnum::Fallback(span) => Span::_new_fallback(span.last_byte()),
            }
        }
    }
    impl Debug for DelimSpan {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            Debug::fmt(&self.join(), f)
        }
    }
}
#[path = "wrapper.rs"]
mod imp {
    use crate::detection::inside_proc_macro;
    use crate::fallback::{self, FromStr2 as _};
    use crate::probe::proc_macro_span;
    use crate::{Delimiter, Punct, Spacing, TokenTree};
    use alloc::string::{String, ToString as _};
    use alloc::vec::Vec;
    use core::ffi::CStr;
    use core::fmt::{self, Debug, Display};
    use core::ops::RangeBounds;
    pub(crate) enum TokenStream {
        Compiler(DeferredTokenStream),
        Fallback(fallback::TokenStream),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TokenStream {
        #[inline]
        fn clone(&self) -> TokenStream {
            match self {
                TokenStream::Compiler(__self_0) => {
                    TokenStream::Compiler(::core::clone::Clone::clone(__self_0))
                }
                TokenStream::Fallback(__self_0) => {
                    TokenStream::Fallback(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    pub(crate) struct DeferredTokenStream {
        stream: proc_macro::TokenStream,
        extra: Vec<proc_macro::TokenTree>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for DeferredTokenStream {
        #[inline]
        fn clone(&self) -> DeferredTokenStream {
            DeferredTokenStream {
                stream: ::core::clone::Clone::clone(&self.stream),
                extra: ::core::clone::Clone::clone(&self.extra),
            }
        }
    }
    pub(crate) enum LexError {
        Compiler(proc_macro::LexError),
        Fallback(fallback::LexError),
        CompilerPanic,
    }
    #[cold]
    fn mismatch(line: u32) -> ! {
        {
            {
                ::core::panicking::panic_fmt(
                    format_args!("compiler/fallback mismatch L{0}", line),
                );
            }
        }
    }
    impl DeferredTokenStream {
        fn new(stream: proc_macro::TokenStream) -> Self {
            DeferredTokenStream {
                stream,
                extra: Vec::new(),
            }
        }
        fn is_empty(&self) -> bool {
            self.stream.is_empty() && self.extra.is_empty()
        }
        fn evaluate_now(&mut self) {
            if !self.extra.is_empty() {
                self.stream.extend(self.extra.drain(..));
            }
        }
        fn into_token_stream(mut self) -> proc_macro::TokenStream {
            self.evaluate_now();
            self.stream
        }
    }
    impl TokenStream {
        pub(crate) fn new() -> Self {
            if inside_proc_macro() {
                TokenStream::Compiler(
                    DeferredTokenStream::new(proc_macro::TokenStream::new()),
                )
            } else {
                TokenStream::Fallback(fallback::TokenStream::new())
            }
        }
        pub(crate) fn from_str_checked(src: &str) -> Result<Self, LexError> {
            if inside_proc_macro() {
                Ok(
                    TokenStream::Compiler(
                        DeferredTokenStream::new(
                            proc_macro::TokenStream::from_str_checked(src)?,
                        ),
                    ),
                )
            } else {
                Ok(TokenStream::Fallback(fallback::TokenStream::from_str_checked(src)?))
            }
        }
        pub(crate) fn is_empty(&self) -> bool {
            match self {
                TokenStream::Compiler(tts) => tts.is_empty(),
                TokenStream::Fallback(tts) => tts.is_empty(),
            }
        }
        fn unwrap_nightly(self) -> proc_macro::TokenStream {
            match self {
                TokenStream::Compiler(s) => s.into_token_stream(),
                TokenStream::Fallback(_) => mismatch(120u32),
            }
        }
        fn unwrap_stable(self) -> fallback::TokenStream {
            match self {
                TokenStream::Compiler(_) => mismatch(126u32),
                TokenStream::Fallback(s) => s,
            }
        }
    }
    impl Display for TokenStream {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                TokenStream::Compiler(tts) => {
                    Display::fmt(&tts.clone().into_token_stream(), f)
                }
                TokenStream::Fallback(tts) => Display::fmt(tts, f),
            }
        }
    }
    impl From<proc_macro::TokenStream> for TokenStream {
        fn from(inner: proc_macro::TokenStream) -> Self {
            TokenStream::Compiler(DeferredTokenStream::new(inner))
        }
    }
    impl From<TokenStream> for proc_macro::TokenStream {
        fn from(inner: TokenStream) -> Self {
            match inner {
                TokenStream::Compiler(inner) => inner.into_token_stream(),
                TokenStream::Fallback(inner) => {
                    proc_macro::TokenStream::from_str_unchecked(&inner.to_string())
                }
            }
        }
    }
    impl From<fallback::TokenStream> for TokenStream {
        fn from(inner: fallback::TokenStream) -> Self {
            TokenStream::Fallback(inner)
        }
    }
    fn into_compiler_token(token: TokenTree) -> proc_macro::TokenTree {
        match token {
            TokenTree::Group(tt) => {
                proc_macro::TokenTree::Group(tt.inner.unwrap_nightly())
            }
            TokenTree::Punct(tt) => {
                let spacing = match tt.spacing() {
                    Spacing::Joint => proc_macro::Spacing::Joint,
                    Spacing::Alone => proc_macro::Spacing::Alone,
                };
                let mut punct = proc_macro::Punct::new(tt.as_char(), spacing);
                punct.set_span(tt.span().inner.unwrap_nightly());
                proc_macro::TokenTree::Punct(punct)
            }
            TokenTree::Ident(tt) => {
                proc_macro::TokenTree::Ident(tt.inner.unwrap_nightly())
            }
            TokenTree::Literal(tt) => {
                proc_macro::TokenTree::Literal(tt.inner.unwrap_nightly())
            }
        }
    }
    impl From<TokenTree> for TokenStream {
        fn from(token: TokenTree) -> Self {
            if inside_proc_macro() {
                TokenStream::Compiler(
                    DeferredTokenStream::new(
                        proc_macro::TokenStream::from(into_compiler_token(token)),
                    ),
                )
            } else {
                TokenStream::Fallback(fallback::TokenStream::from(token))
            }
        }
    }
    impl FromIterator<TokenTree> for TokenStream {
        fn from_iter<I: IntoIterator<Item = TokenTree>>(tokens: I) -> Self {
            if inside_proc_macro() {
                TokenStream::Compiler(
                    DeferredTokenStream::new(
                        tokens.into_iter().map(into_compiler_token).collect(),
                    ),
                )
            } else {
                TokenStream::Fallback(tokens.into_iter().collect())
            }
        }
    }
    impl FromIterator<TokenStream> for TokenStream {
        fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
            let mut streams = streams.into_iter();
            match streams.next() {
                Some(TokenStream::Compiler(mut first)) => {
                    first.evaluate_now();
                    first
                        .stream
                        .extend(
                            streams
                                .map(|s| match s {
                                    TokenStream::Compiler(s) => s.into_token_stream(),
                                    TokenStream::Fallback(_) => mismatch(214u32),
                                }),
                        );
                    TokenStream::Compiler(first)
                }
                Some(TokenStream::Fallback(mut first)) => {
                    first
                        .extend(
                            streams
                                .map(|s| match s {
                                    TokenStream::Fallback(s) => s,
                                    TokenStream::Compiler(_) => mismatch(221u32),
                                }),
                        );
                    TokenStream::Fallback(first)
                }
                None => TokenStream::new(),
            }
        }
    }
    impl Extend<TokenTree> for TokenStream {
        fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, tokens: I) {
            match self {
                TokenStream::Compiler(tts) => {
                    for token in tokens {
                        tts.extra.push(into_compiler_token(token));
                    }
                }
                TokenStream::Fallback(tts) => tts.extend(tokens),
            }
        }
    }
    impl Extend<TokenStream> for TokenStream {
        fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
            match self {
                TokenStream::Compiler(tts) => {
                    tts.evaluate_now();
                    tts.stream
                        .extend(streams.into_iter().map(TokenStream::unwrap_nightly));
                }
                TokenStream::Fallback(tts) => {
                    tts.extend(streams.into_iter().map(TokenStream::unwrap_stable));
                }
            }
        }
    }
    impl Debug for TokenStream {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                TokenStream::Compiler(tts) => {
                    Debug::fmt(&tts.clone().into_token_stream(), f)
                }
                TokenStream::Fallback(tts) => Debug::fmt(tts, f),
            }
        }
    }
    impl LexError {
        pub(crate) fn span(&self) -> Span {
            match self {
                LexError::Compiler(_) | LexError::CompilerPanic => Span::call_site(),
                LexError::Fallback(e) => Span::Fallback(e.span()),
            }
        }
    }
    impl From<proc_macro::LexError> for LexError {
        fn from(e: proc_macro::LexError) -> Self {
            LexError::Compiler(e)
        }
    }
    impl From<fallback::LexError> for LexError {
        fn from(e: fallback::LexError) -> Self {
            LexError::Fallback(e)
        }
    }
    impl Debug for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                LexError::Compiler(e) => Debug::fmt(e, f),
                LexError::Fallback(e) => Debug::fmt(e, f),
                LexError::CompilerPanic => {
                    let fallback = fallback::LexError::call_site();
                    Debug::fmt(&fallback, f)
                }
            }
        }
    }
    impl Display for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                LexError::Compiler(e) => Display::fmt(e, f),
                LexError::Fallback(e) => Display::fmt(e, f),
                LexError::CompilerPanic => {
                    let fallback = fallback::LexError::call_site();
                    Display::fmt(&fallback, f)
                }
            }
        }
    }
    pub(crate) enum TokenTreeIter {
        Compiler(proc_macro::token_stream::IntoIter),
        Fallback(fallback::TokenTreeIter),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TokenTreeIter {
        #[inline]
        fn clone(&self) -> TokenTreeIter {
            match self {
                TokenTreeIter::Compiler(__self_0) => {
                    TokenTreeIter::Compiler(::core::clone::Clone::clone(__self_0))
                }
                TokenTreeIter::Fallback(__self_0) => {
                    TokenTreeIter::Fallback(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl IntoIterator for TokenStream {
        type Item = TokenTree;
        type IntoIter = TokenTreeIter;
        fn into_iter(self) -> TokenTreeIter {
            match self {
                TokenStream::Compiler(tts) => {
                    TokenTreeIter::Compiler(tts.into_token_stream().into_iter())
                }
                TokenStream::Fallback(tts) => TokenTreeIter::Fallback(tts.into_iter()),
            }
        }
    }
    impl Iterator for TokenTreeIter {
        type Item = TokenTree;
        fn next(&mut self) -> Option<TokenTree> {
            let token = match self {
                TokenTreeIter::Compiler(iter) => iter.next()?,
                TokenTreeIter::Fallback(iter) => return iter.next(),
            };
            Some(
                match token {
                    proc_macro::TokenTree::Group(tt) => {
                        TokenTree::Group(crate::Group::_new(Group::Compiler(tt)))
                    }
                    proc_macro::TokenTree::Punct(tt) => {
                        let spacing = match tt.spacing() {
                            proc_macro::Spacing::Joint => Spacing::Joint,
                            proc_macro::Spacing::Alone => Spacing::Alone,
                        };
                        let mut o = Punct::new(tt.as_char(), spacing);
                        o.set_span(crate::Span::_new(Span::Compiler(tt.span())));
                        TokenTree::Punct(o)
                    }
                    proc_macro::TokenTree::Ident(s) => {
                        TokenTree::Ident(crate::Ident::_new(Ident::Compiler(s)))
                    }
                    proc_macro::TokenTree::Literal(l) => {
                        TokenTree::Literal(crate::Literal::_new(Literal::Compiler(l)))
                    }
                },
            )
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            match self {
                TokenTreeIter::Compiler(tts) => tts.size_hint(),
                TokenTreeIter::Fallback(tts) => tts.size_hint(),
            }
        }
    }
    pub(crate) enum Span {
        Compiler(proc_macro::Span),
        Fallback(fallback::Span),
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
            let _: ::core::clone::AssertParamIsClone<proc_macro::Span>;
            let _: ::core::clone::AssertParamIsClone<fallback::Span>;
            *self
        }
    }
    impl Span {
        pub(crate) fn call_site() -> Self {
            if inside_proc_macro() {
                Span::Compiler(proc_macro::Span::call_site())
            } else {
                Span::Fallback(fallback::Span::call_site())
            }
        }
        pub(crate) fn mixed_site() -> Self {
            if inside_proc_macro() {
                Span::Compiler(proc_macro::Span::mixed_site())
            } else {
                Span::Fallback(fallback::Span::mixed_site())
            }
        }
        pub(crate) fn resolved_at(&self, other: Span) -> Span {
            match (self, other) {
                (Span::Compiler(a), Span::Compiler(b)) => {
                    Span::Compiler(a.resolved_at(b))
                }
                (Span::Fallback(a), Span::Fallback(b)) => {
                    Span::Fallback(a.resolved_at(b))
                }
                (Span::Compiler(_), Span::Fallback(_)) => mismatch(409u32),
                (Span::Fallback(_), Span::Compiler(_)) => mismatch(410u32),
            }
        }
        pub(crate) fn located_at(&self, other: Span) -> Span {
            match (self, other) {
                (Span::Compiler(a), Span::Compiler(b)) => Span::Compiler(a.located_at(b)),
                (Span::Fallback(a), Span::Fallback(b)) => Span::Fallback(a.located_at(b)),
                (Span::Compiler(_), Span::Fallback(_)) => mismatch(418u32),
                (Span::Fallback(_), Span::Compiler(_)) => mismatch(419u32),
            }
        }
        pub(crate) fn unwrap(self) -> proc_macro::Span {
            match self {
                Span::Compiler(s) => s,
                Span::Fallback(_) => {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "proc_macro::Span is only available in procedural macros",
                        ),
                    );
                }
            }
        }
        pub(crate) fn join(&self, other: Span) -> Option<Span> {
            let ret = match (self, other) {
                (Span::Compiler(a), Span::Compiler(b)) => {
                    Span::Compiler(proc_macro_span::join(a, b)?)
                }
                (Span::Fallback(a), Span::Fallback(b)) => Span::Fallback(a.join(b)?),
                _ => return None,
            };
            Some(ret)
        }
        pub(crate) fn source_text(&self) -> Option<String> {
            match self {
                Span::Compiler(s) => s.source_text(),
                Span::Fallback(s) => s.source_text(),
            }
        }
        fn unwrap_nightly(self) -> proc_macro::Span {
            match self {
                Span::Compiler(s) => s,
                Span::Fallback(_) => mismatch(526u32),
            }
        }
    }
    impl From<proc_macro::Span> for crate::Span {
        fn from(proc_span: proc_macro::Span) -> Self {
            crate::Span::_new(Span::Compiler(proc_span))
        }
    }
    impl From<fallback::Span> for Span {
        fn from(inner: fallback::Span) -> Self {
            Span::Fallback(inner)
        }
    }
    impl Debug for Span {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Span::Compiler(s) => Debug::fmt(s, f),
                Span::Fallback(s) => Debug::fmt(s, f),
            }
        }
    }
    pub(crate) fn debug_span_field_if_nontrivial(
        debug: &mut fmt::DebugStruct,
        span: Span,
    ) {
        match span {
            Span::Compiler(s) => {
                debug.field("span", &s);
            }
            Span::Fallback(s) => fallback::debug_span_field_if_nontrivial(debug, s),
        }
    }
    pub(crate) enum Group {
        Compiler(proc_macro::Group),
        Fallback(fallback::Group),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Group {
        #[inline]
        fn clone(&self) -> Group {
            match self {
                Group::Compiler(__self_0) => {
                    Group::Compiler(::core::clone::Clone::clone(__self_0))
                }
                Group::Fallback(__self_0) => {
                    Group::Fallback(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl Group {
        pub(crate) fn new(delimiter: Delimiter, stream: TokenStream) -> Self {
            match stream {
                TokenStream::Compiler(tts) => {
                    let delimiter = match delimiter {
                        Delimiter::Parenthesis => proc_macro::Delimiter::Parenthesis,
                        Delimiter::Bracket => proc_macro::Delimiter::Bracket,
                        Delimiter::Brace => proc_macro::Delimiter::Brace,
                        Delimiter::None => proc_macro::Delimiter::None,
                    };
                    Group::Compiler(
                        proc_macro::Group::new(delimiter, tts.into_token_stream()),
                    )
                }
                TokenStream::Fallback(stream) => {
                    Group::Fallback(fallback::Group::new(delimiter, stream))
                }
            }
        }
        pub(crate) fn delimiter(&self) -> Delimiter {
            match self {
                Group::Compiler(g) => {
                    match g.delimiter() {
                        proc_macro::Delimiter::Parenthesis => Delimiter::Parenthesis,
                        proc_macro::Delimiter::Bracket => Delimiter::Bracket,
                        proc_macro::Delimiter::Brace => Delimiter::Brace,
                        proc_macro::Delimiter::None => Delimiter::None,
                    }
                }
                Group::Fallback(g) => g.delimiter(),
            }
        }
        pub(crate) fn stream(&self) -> TokenStream {
            match self {
                Group::Compiler(g) => {
                    TokenStream::Compiler(DeferredTokenStream::new(g.stream()))
                }
                Group::Fallback(g) => TokenStream::Fallback(g.stream()),
            }
        }
        pub(crate) fn span(&self) -> Span {
            match self {
                Group::Compiler(g) => Span::Compiler(g.span()),
                Group::Fallback(g) => Span::Fallback(g.span()),
            }
        }
        pub(crate) fn span_open(&self) -> Span {
            match self {
                Group::Compiler(g) => Span::Compiler(g.span_open()),
                Group::Fallback(g) => Span::Fallback(g.span_open()),
            }
        }
        pub(crate) fn span_close(&self) -> Span {
            match self {
                Group::Compiler(g) => Span::Compiler(g.span_close()),
                Group::Fallback(g) => Span::Fallback(g.span_close()),
            }
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            match (self, span) {
                (Group::Compiler(g), Span::Compiler(s)) => g.set_span(s),
                (Group::Fallback(g), Span::Fallback(s)) => g.set_span(s),
                (Group::Compiler(_), Span::Fallback(_)) => mismatch(629u32),
                (Group::Fallback(_), Span::Compiler(_)) => mismatch(630u32),
            }
        }
        fn unwrap_nightly(self) -> proc_macro::Group {
            match self {
                Group::Compiler(g) => g,
                Group::Fallback(_) => mismatch(637u32),
            }
        }
    }
    impl From<fallback::Group> for Group {
        fn from(g: fallback::Group) -> Self {
            Group::Fallback(g)
        }
    }
    impl Display for Group {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Group::Compiler(group) => Display::fmt(group, formatter),
                Group::Fallback(group) => Display::fmt(group, formatter),
            }
        }
    }
    impl Debug for Group {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Group::Compiler(group) => Debug::fmt(group, formatter),
                Group::Fallback(group) => Debug::fmt(group, formatter),
            }
        }
    }
    pub(crate) enum Ident {
        Compiler(proc_macro::Ident),
        Fallback(fallback::Ident),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Ident {
        #[inline]
        fn clone(&self) -> Ident {
            match self {
                Ident::Compiler(__self_0) => {
                    Ident::Compiler(::core::clone::Clone::clone(__self_0))
                }
                Ident::Fallback(__self_0) => {
                    Ident::Fallback(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl Ident {
        #[track_caller]
        pub(crate) fn new_checked(string: &str, span: Span) -> Self {
            match span {
                Span::Compiler(s) => Ident::Compiler(proc_macro::Ident::new(string, s)),
                Span::Fallback(s) => {
                    Ident::Fallback(fallback::Ident::new_checked(string, s))
                }
            }
        }
        #[track_caller]
        pub(crate) fn new_raw_checked(string: &str, span: Span) -> Self {
            match span {
                Span::Compiler(s) => {
                    Ident::Compiler(proc_macro::Ident::new_raw(string, s))
                }
                Span::Fallback(s) => {
                    Ident::Fallback(fallback::Ident::new_raw_checked(string, s))
                }
            }
        }
        pub(crate) fn span(&self) -> Span {
            match self {
                Ident::Compiler(t) => Span::Compiler(t.span()),
                Ident::Fallback(t) => Span::Fallback(t.span()),
            }
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            match (self, span) {
                (Ident::Compiler(t), Span::Compiler(s)) => t.set_span(s),
                (Ident::Fallback(t), Span::Fallback(s)) => t.set_span(s),
                (Ident::Compiler(_), Span::Fallback(_)) => mismatch(700u32),
                (Ident::Fallback(_), Span::Compiler(_)) => mismatch(701u32),
            }
        }
        fn unwrap_nightly(self) -> proc_macro::Ident {
            match self {
                Ident::Compiler(s) => s,
                Ident::Fallback(_) => mismatch(708u32),
            }
        }
    }
    impl From<fallback::Ident> for Ident {
        fn from(inner: fallback::Ident) -> Self {
            Ident::Fallback(inner)
        }
    }
    impl PartialEq for Ident {
        fn eq(&self, other: &Ident) -> bool {
            match (self, other) {
                (Ident::Compiler(t), Ident::Compiler(o)) => {
                    t.to_string() == o.to_string()
                }
                (Ident::Fallback(t), Ident::Fallback(o)) => t == o,
                (Ident::Compiler(_), Ident::Fallback(_)) => mismatch(724u32),
                (Ident::Fallback(_), Ident::Compiler(_)) => mismatch(725u32),
            }
        }
    }
    impl<T> PartialEq<T> for Ident
    where
        T: ?Sized + AsRef<str>,
    {
        fn eq(&self, other: &T) -> bool {
            let other = other.as_ref();
            match self {
                Ident::Compiler(t) => t.to_string() == other,
                Ident::Fallback(t) => t == other,
            }
        }
    }
    impl Display for Ident {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Ident::Compiler(t) => Display::fmt(t, f),
                Ident::Fallback(t) => Display::fmt(t, f),
            }
        }
    }
    impl Debug for Ident {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Ident::Compiler(t) => Debug::fmt(t, f),
                Ident::Fallback(t) => Debug::fmt(t, f),
            }
        }
    }
    pub(crate) enum Literal {
        Compiler(proc_macro::Literal),
        Fallback(fallback::Literal),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Literal {
        #[inline]
        fn clone(&self) -> Literal {
            match self {
                Literal::Compiler(__self_0) => {
                    Literal::Compiler(::core::clone::Clone::clone(__self_0))
                }
                Literal::Fallback(__self_0) => {
                    Literal::Fallback(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl Literal {
        pub(crate) fn from_str_checked(repr: &str) -> Result<Self, LexError> {
            if inside_proc_macro() {
                let literal = proc_macro::Literal::from_str_checked(repr)?;
                Ok(Literal::Compiler(literal))
            } else {
                let literal = fallback::Literal::from_str_checked(repr)?;
                Ok(Literal::Fallback(literal))
            }
        }
        pub(crate) unsafe fn from_str_unchecked(repr: &str) -> Self {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::from_str_unchecked(repr))
            } else {
                Literal::Fallback(unsafe { fallback::Literal::from_str_unchecked(repr) })
            }
        }
        pub(crate) fn u8_suffixed(n: u8) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u8_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u8_suffixed(n))
            }
        }
        pub(crate) fn u16_suffixed(n: u16) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u16_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u16_suffixed(n))
            }
        }
        pub(crate) fn u32_suffixed(n: u32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u32_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u32_suffixed(n))
            }
        }
        pub(crate) fn u64_suffixed(n: u64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u64_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u64_suffixed(n))
            }
        }
        pub(crate) fn u128_suffixed(n: u128) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u128_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u128_suffixed(n))
            }
        }
        pub(crate) fn usize_suffixed(n: usize) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::usize_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::usize_suffixed(n))
            }
        }
        pub(crate) fn i8_suffixed(n: i8) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i8_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i8_suffixed(n))
            }
        }
        pub(crate) fn i16_suffixed(n: i16) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i16_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i16_suffixed(n))
            }
        }
        pub(crate) fn i32_suffixed(n: i32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i32_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i32_suffixed(n))
            }
        }
        pub(crate) fn i64_suffixed(n: i64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i64_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i64_suffixed(n))
            }
        }
        pub(crate) fn i128_suffixed(n: i128) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i128_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i128_suffixed(n))
            }
        }
        pub(crate) fn isize_suffixed(n: isize) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::isize_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::isize_suffixed(n))
            }
        }
        pub(crate) fn f32_suffixed(n: f32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::f32_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::f32_suffixed(n))
            }
        }
        pub(crate) fn f64_suffixed(n: f64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::f64_suffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::f64_suffixed(n))
            }
        }
        pub(crate) fn u8_unsuffixed(n: u8) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u8_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u8_unsuffixed(n))
            }
        }
        pub(crate) fn u16_unsuffixed(n: u16) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u16_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u16_unsuffixed(n))
            }
        }
        pub(crate) fn u32_unsuffixed(n: u32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u32_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u32_unsuffixed(n))
            }
        }
        pub(crate) fn u64_unsuffixed(n: u64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u64_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u64_unsuffixed(n))
            }
        }
        pub(crate) fn u128_unsuffixed(n: u128) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::u128_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::u128_unsuffixed(n))
            }
        }
        pub(crate) fn usize_unsuffixed(n: usize) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::usize_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::usize_unsuffixed(n))
            }
        }
        pub(crate) fn i8_unsuffixed(n: i8) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i8_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i8_unsuffixed(n))
            }
        }
        pub(crate) fn i16_unsuffixed(n: i16) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i16_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i16_unsuffixed(n))
            }
        }
        pub(crate) fn i32_unsuffixed(n: i32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i32_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i32_unsuffixed(n))
            }
        }
        pub(crate) fn i64_unsuffixed(n: i64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i64_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i64_unsuffixed(n))
            }
        }
        pub(crate) fn i128_unsuffixed(n: i128) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::i128_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::i128_unsuffixed(n))
            }
        }
        pub(crate) fn isize_unsuffixed(n: isize) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::isize_unsuffixed(n))
            } else {
                Literal::Fallback(fallback::Literal::isize_unsuffixed(n))
            }
        }
        pub(crate) fn f32_unsuffixed(f: f32) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::f32_unsuffixed(f))
            } else {
                Literal::Fallback(fallback::Literal::f32_unsuffixed(f))
            }
        }
        pub(crate) fn f64_unsuffixed(f: f64) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::f64_unsuffixed(f))
            } else {
                Literal::Fallback(fallback::Literal::f64_unsuffixed(f))
            }
        }
        pub(crate) fn string(string: &str) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::string(string))
            } else {
                Literal::Fallback(fallback::Literal::string(string))
            }
        }
        pub(crate) fn character(ch: char) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::character(ch))
            } else {
                Literal::Fallback(fallback::Literal::character(ch))
            }
        }
        pub(crate) fn byte_character(byte: u8) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler({ { proc_macro::Literal::byte_character(byte) } })
            } else {
                Literal::Fallback(fallback::Literal::byte_character(byte))
            }
        }
        pub(crate) fn byte_string(bytes: &[u8]) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler(proc_macro::Literal::byte_string(bytes))
            } else {
                Literal::Fallback(fallback::Literal::byte_string(bytes))
            }
        }
        pub(crate) fn c_string(string: &CStr) -> Literal {
            if inside_proc_macro() {
                Literal::Compiler({ { proc_macro::Literal::c_string(string) } })
            } else {
                Literal::Fallback(fallback::Literal::c_string(string))
            }
        }
        pub(crate) fn span(&self) -> Span {
            match self {
                Literal::Compiler(lit) => Span::Compiler(lit.span()),
                Literal::Fallback(lit) => Span::Fallback(lit.span()),
            }
        }
        pub(crate) fn set_span(&mut self, span: Span) {
            match (self, span) {
                (Literal::Compiler(lit), Span::Compiler(s)) => lit.set_span(s),
                (Literal::Fallback(lit), Span::Fallback(s)) => lit.set_span(s),
                (Literal::Compiler(_), Span::Fallback(_)) => mismatch(932u32),
                (Literal::Fallback(_), Span::Compiler(_)) => mismatch(933u32),
            }
        }
        pub(crate) fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Option<Span> {
            match self {
                Literal::Compiler(lit) => {
                    proc_macro_span::subspan(lit, range).map(Span::Compiler)
                }
                Literal::Fallback(lit) => lit.subspan(range).map(Span::Fallback),
            }
        }
        fn unwrap_nightly(self) -> proc_macro::Literal {
            match self {
                Literal::Compiler(s) => s,
                Literal::Fallback(_) => mismatch(950u32),
            }
        }
    }
    impl From<fallback::Literal> for Literal {
        fn from(s: fallback::Literal) -> Self {
            Literal::Fallback(s)
        }
    }
    impl Display for Literal {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Literal::Compiler(t) => Display::fmt(t, f),
                Literal::Fallback(t) => Display::fmt(t, f),
            }
        }
    }
    impl Debug for Literal {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Literal::Compiler(t) => Debug::fmt(t, f),
                Literal::Fallback(t) => Debug::fmt(t, f),
            }
        }
    }
}
use crate::extra::DelimSpan;
use crate::marker::{ProcMacroAutoTraits, MARKER};
use alloc::string::{String, ToString as _};
use core::cmp::Ordering;
use core::ffi::CStr;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use core::ops::RangeBounds;
use core::str::FromStr;
use std::error::Error;
/// An abstract stream of tokens, or more concretely a sequence of token trees.
///
/// This type provides interfaces for iterating over token trees and for
/// collecting token trees into one stream.
///
/// Token stream is both the input and output of `#[proc_macro]`,
/// `#[proc_macro_attribute]` and `#[proc_macro_derive]` definitions.
pub struct TokenStream {
    inner: imp::TokenStream,
    _marker: ProcMacroAutoTraits,
}
#[automatically_derived]
impl ::core::clone::Clone for TokenStream {
    #[inline]
    fn clone(&self) -> TokenStream {
        TokenStream {
            inner: ::core::clone::Clone::clone(&self.inner),
            _marker: ::core::clone::Clone::clone(&self._marker),
        }
    }
}
/// Error returned from `TokenStream::from_str`.
pub struct LexError {
    inner: imp::LexError,
    _marker: ProcMacroAutoTraits,
}
impl TokenStream {
    fn _new(inner: imp::TokenStream) -> Self {
        TokenStream {
            inner,
            _marker: MARKER,
        }
    }
    fn _new_fallback(inner: fallback::TokenStream) -> Self {
        TokenStream {
            inner: imp::TokenStream::from(inner),
            _marker: MARKER,
        }
    }
    /// Returns an empty `TokenStream` containing no token trees.
    pub fn new() -> Self {
        TokenStream::_new(imp::TokenStream::new())
    }
    /// Checks if this `TokenStream` is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
/// `TokenStream::default()` returns an empty stream,
/// i.e. this is equivalent with `TokenStream::new()`.
impl Default for TokenStream {
    fn default() -> Self {
        TokenStream::new()
    }
}
/// Attempts to break the string into tokens and parse those tokens into a token
/// stream.
///
/// May fail for a number of reasons, for example, if the string contains
/// unbalanced delimiters or characters not existing in the language.
///
/// NOTE: Some errors may cause panics instead of returning `LexError`. We
/// reserve the right to change these errors into `LexError`s later.
impl FromStr for TokenStream {
    type Err = LexError;
    fn from_str(src: &str) -> Result<TokenStream, LexError> {
        match imp::TokenStream::from_str_checked(src) {
            Ok(tokens) => Ok(TokenStream::_new(tokens)),
            Err(lex) => {
                Err(LexError {
                    inner: lex,
                    _marker: MARKER,
                })
            }
        }
    }
}
impl From<proc_macro::TokenStream> for TokenStream {
    fn from(inner: proc_macro::TokenStream) -> Self {
        TokenStream::_new(imp::TokenStream::from(inner))
    }
}
impl From<TokenStream> for proc_macro::TokenStream {
    fn from(inner: TokenStream) -> Self {
        proc_macro::TokenStream::from(inner.inner)
    }
}
impl From<TokenTree> for TokenStream {
    fn from(token: TokenTree) -> Self {
        TokenStream::_new(imp::TokenStream::from(token))
    }
}
impl Extend<TokenTree> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenTree>>(&mut self, tokens: I) {
        self.inner.extend(tokens);
    }
}
impl Extend<TokenStream> for TokenStream {
    fn extend<I: IntoIterator<Item = TokenStream>>(&mut self, streams: I) {
        self.inner.extend(streams.into_iter().map(|stream| stream.inner));
    }
}
impl Extend<Group> for TokenStream {
    fn extend<I: IntoIterator<Item = Group>>(&mut self, tokens: I) {
        self.inner.extend(tokens.into_iter().map(TokenTree::Group));
    }
}
impl Extend<Ident> for TokenStream {
    fn extend<I: IntoIterator<Item = Ident>>(&mut self, tokens: I) {
        self.inner.extend(tokens.into_iter().map(TokenTree::Ident));
    }
}
impl Extend<Punct> for TokenStream {
    fn extend<I: IntoIterator<Item = Punct>>(&mut self, tokens: I) {
        self.inner.extend(tokens.into_iter().map(TokenTree::Punct));
    }
}
impl Extend<Literal> for TokenStream {
    fn extend<I: IntoIterator<Item = Literal>>(&mut self, tokens: I) {
        self.inner.extend(tokens.into_iter().map(TokenTree::Literal));
    }
}
/// Collects a number of token trees into a single stream.
impl FromIterator<TokenTree> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenTree>>(tokens: I) -> Self {
        TokenStream::_new(tokens.into_iter().collect())
    }
}
impl FromIterator<TokenStream> for TokenStream {
    fn from_iter<I: IntoIterator<Item = TokenStream>>(streams: I) -> Self {
        TokenStream::_new(streams.into_iter().map(|i| i.inner).collect())
    }
}
/// Prints the token stream as a string that is supposed to be losslessly
/// convertible back into the same token stream (modulo spans), except for
/// possibly `TokenTree::Group`s with `Delimiter::None` delimiters and negative
/// numeric literals.
impl Display for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
/// Prints token in a form convenient for debugging.
impl Debug for TokenStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl LexError {
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
}
impl Debug for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl Error for LexError {}
/// A region of source code, along with macro expansion information.
pub struct Span {
    inner: imp::Span,
    _marker: ProcMacroAutoTraits,
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
        let _: ::core::clone::AssertParamIsClone<imp::Span>;
        let _: ::core::clone::AssertParamIsClone<ProcMacroAutoTraits>;
        *self
    }
}
impl Span {
    fn _new(inner: imp::Span) -> Self {
        Span { inner, _marker: MARKER }
    }
    fn _new_fallback(inner: fallback::Span) -> Self {
        Span {
            inner: imp::Span::from(inner),
            _marker: MARKER,
        }
    }
    /// The span of the invocation of the current procedural macro.
    ///
    /// Identifiers created with this span will be resolved as if they were
    /// written directly at the macro call location (call-site hygiene) and
    /// other code at the macro call site will be able to refer to them as well.
    pub fn call_site() -> Self {
        Span::_new(imp::Span::call_site())
    }
    /// The span located at the invocation of the procedural macro, but with
    /// local variables, labels, and `$crate` resolved at the definition site
    /// of the macro. This is the same hygiene behavior as `macro_rules`.
    pub fn mixed_site() -> Self {
        Span::_new(imp::Span::mixed_site())
    }
    /// Creates a new span with the same line/column information as `self` but
    /// that resolves symbols as though it were at `other`.
    pub fn resolved_at(&self, other: Span) -> Span {
        Span::_new(self.inner.resolved_at(other.inner))
    }
    /// Creates a new span with the same name resolution behavior as `self` but
    /// with the line/column information of `other`.
    pub fn located_at(&self, other: Span) -> Span {
        Span::_new(self.inner.located_at(other.inner))
    }
    /// Convert `proc_macro2::Span` to `proc_macro::Span`.
    ///
    /// This method is available when building with a nightly compiler, or when
    /// building with rustc 1.29+ *without* semver exempt features.
    ///
    /// # Panics
    ///
    /// Panics if called from outside of a procedural macro. Unlike
    /// `proc_macro2::Span`, the `proc_macro::Span` type can only exist within
    /// the context of a procedural macro invocation.
    pub fn unwrap(self) -> proc_macro::Span {
        self.inner.unwrap()
    }
    #[doc(hidden)]
    pub fn unstable(self) -> proc_macro::Span {
        self.unwrap()
    }
    /// Create a new span encompassing `self` and `other`.
    ///
    /// Returns `None` if `self` and `other` are from different files.
    ///
    /// Warning: the underlying [`proc_macro::Span::join`] method is
    /// nightly-only. When called from within a procedural macro not using a
    /// nightly compiler, this method will always return `None`.
    pub fn join(&self, other: Span) -> Option<Span> {
        self.inner.join(other.inner).map(Span::_new)
    }
    /// Returns the source text behind a span. This preserves the original
    /// source code, including spaces and comments. It only returns a result if
    /// the span corresponds to real source code.
    ///
    /// Note: The observable result of a macro should only rely on the tokens
    /// and not on this source text. The result of this function is a best
    /// effort to be used for diagnostics only.
    pub fn source_text(&self) -> Option<String> {
        self.inner.source_text()
    }
}
/// Prints a span in a form convenient for debugging.
impl Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
/// A single token or a delimited sequence of token trees (e.g. `[1, (), ..]`).
pub enum TokenTree {
    /// A token stream surrounded by bracket delimiters.
    Group(Group),
    /// An identifier.
    Ident(Ident),
    /// A single punctuation character (`+`, `,`, `$`, etc.).
    Punct(Punct),
    /// A literal character (`'a'`), string (`"hello"`), number (`2.3`), etc.
    Literal(Literal),
}
#[automatically_derived]
impl ::core::clone::Clone for TokenTree {
    #[inline]
    fn clone(&self) -> TokenTree {
        match self {
            TokenTree::Group(__self_0) => {
                TokenTree::Group(::core::clone::Clone::clone(__self_0))
            }
            TokenTree::Ident(__self_0) => {
                TokenTree::Ident(::core::clone::Clone::clone(__self_0))
            }
            TokenTree::Punct(__self_0) => {
                TokenTree::Punct(::core::clone::Clone::clone(__self_0))
            }
            TokenTree::Literal(__self_0) => {
                TokenTree::Literal(::core::clone::Clone::clone(__self_0))
            }
        }
    }
}
impl TokenTree {
    /// Returns the span of this tree, delegating to the `span` method of
    /// the contained token or a delimited stream.
    pub fn span(&self) -> Span {
        match self {
            TokenTree::Group(t) => t.span(),
            TokenTree::Ident(t) => t.span(),
            TokenTree::Punct(t) => t.span(),
            TokenTree::Literal(t) => t.span(),
        }
    }
    /// Configures the span for *only this token*.
    ///
    /// Note that if this token is a `Group` then this method will not configure
    /// the span of each of the internal tokens, this will simply delegate to
    /// the `set_span` method of each variant.
    pub fn set_span(&mut self, span: Span) {
        match self {
            TokenTree::Group(t) => t.set_span(span),
            TokenTree::Ident(t) => t.set_span(span),
            TokenTree::Punct(t) => t.set_span(span),
            TokenTree::Literal(t) => t.set_span(span),
        }
    }
}
impl From<Group> for TokenTree {
    fn from(g: Group) -> Self {
        TokenTree::Group(g)
    }
}
impl From<Ident> for TokenTree {
    fn from(g: Ident) -> Self {
        TokenTree::Ident(g)
    }
}
impl From<Punct> for TokenTree {
    fn from(g: Punct) -> Self {
        TokenTree::Punct(g)
    }
}
impl From<Literal> for TokenTree {
    fn from(g: Literal) -> Self {
        TokenTree::Literal(g)
    }
}
/// Prints the token tree as a string that is supposed to be losslessly
/// convertible back into the same token tree (modulo spans), except for
/// possibly `TokenTree::Group`s with `Delimiter::None` delimiters and negative
/// numeric literals.
impl Display for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenTree::Group(t) => Display::fmt(t, f),
            TokenTree::Ident(t) => Display::fmt(t, f),
            TokenTree::Punct(t) => Display::fmt(t, f),
            TokenTree::Literal(t) => Display::fmt(t, f),
        }
    }
}
/// Prints token tree in a form convenient for debugging.
impl Debug for TokenTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenTree::Group(t) => Debug::fmt(t, f),
            TokenTree::Ident(t) => {
                let mut debug = f.debug_struct("Ident");
                debug.field("sym", &format_args!("{0}", t));
                imp::debug_span_field_if_nontrivial(&mut debug, t.span().inner);
                debug.finish()
            }
            TokenTree::Punct(t) => Debug::fmt(t, f),
            TokenTree::Literal(t) => Debug::fmt(t, f),
        }
    }
}
/// A delimited token stream.
///
/// A `Group` internally contains a `TokenStream` which is surrounded by
/// `Delimiter`s.
pub struct Group {
    inner: imp::Group,
}
#[automatically_derived]
impl ::core::clone::Clone for Group {
    #[inline]
    fn clone(&self) -> Group {
        Group {
            inner: ::core::clone::Clone::clone(&self.inner),
        }
    }
}
/// Describes how a sequence of token trees is delimited.
pub enum Delimiter {
    /// `( ... )`
    Parenthesis,
    /// `{ ... }`
    Brace,
    /// `[ ... ]`
    Bracket,
    /// `∅ ... ∅`
    ///
    /// An invisible delimiter, that may, for example, appear around tokens
    /// coming from a "macro variable" `$var`. It is important to preserve
    /// operator priorities in cases like `$var * 3` where `$var` is `1 + 2`.
    /// Invisible delimiters may not survive roundtrip of a token stream through
    /// a string.
    ///
    /// <div class="warning">
    ///
    /// Note: rustc currently can ignore the grouping of tokens delimited by `None` in the output
    /// of a proc_macro. Only `None`-delimited groups created by a macro_rules macro in the input
    /// of a proc_macro macro are preserved, and only in very specific circumstances.
    /// Any `None`-delimited groups (re)created by a proc_macro will therefore not preserve
    /// operator priorities as indicated above. The other `Delimiter` variants should be used
    /// instead in this context. This is a rustc bug. For details, see
    /// [rust-lang/rust#67062](https://github.com/rust-lang/rust/issues/67062).
    ///
    /// </div>
    None,
}
#[automatically_derived]
impl ::core::marker::Copy for Delimiter {}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Delimiter {}
#[automatically_derived]
impl ::core::clone::Clone for Delimiter {
    #[inline]
    fn clone(&self) -> Delimiter {
        *self
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Delimiter {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Delimiter::Parenthesis => "Parenthesis",
                Delimiter::Brace => "Brace",
                Delimiter::Bracket => "Bracket",
                Delimiter::None => "None",
            },
        )
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Delimiter {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {}
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Delimiter {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Delimiter {
    #[inline]
    fn eq(&self, other: &Delimiter) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
impl Group {
    fn _new(inner: imp::Group) -> Self {
        Group { inner }
    }
    fn _new_fallback(inner: fallback::Group) -> Self {
        Group {
            inner: imp::Group::from(inner),
        }
    }
    /// Creates a new `Group` with the given delimiter and token stream.
    ///
    /// This constructor will set the span for this group to
    /// `Span::call_site()`. To change the span you can use the `set_span`
    /// method below.
    pub fn new(delimiter: Delimiter, stream: TokenStream) -> Self {
        Group {
            inner: imp::Group::new(delimiter, stream.inner),
        }
    }
    /// Returns the punctuation used as the delimiter for this group: a set of
    /// parentheses, square brackets, or curly braces.
    pub fn delimiter(&self) -> Delimiter {
        self.inner.delimiter()
    }
    /// Returns the `TokenStream` of tokens that are delimited in this `Group`.
    ///
    /// Note that the returned token stream does not include the delimiter
    /// returned above.
    pub fn stream(&self) -> TokenStream {
        TokenStream::_new(self.inner.stream())
    }
    /// Returns the span for the delimiters of this token stream, spanning the
    /// entire `Group`.
    ///
    /// ```text
    /// pub fn span(&self) -> Span {
    ///            ^^^^^^^
    /// ```
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Returns the span pointing to the opening delimiter of this group.
    ///
    /// ```text
    /// pub fn span_open(&self) -> Span {
    ///                 ^
    /// ```
    pub fn span_open(&self) -> Span {
        Span::_new(self.inner.span_open())
    }
    /// Returns the span pointing to the closing delimiter of this group.
    ///
    /// ```text
    /// pub fn span_close(&self) -> Span {
    ///                        ^
    /// ```
    pub fn span_close(&self) -> Span {
        Span::_new(self.inner.span_close())
    }
    /// Returns an object that holds this group's `span_open()` and
    /// `span_close()` together (in a more compact representation than holding
    /// those 2 spans individually).
    pub fn delim_span(&self) -> DelimSpan {
        DelimSpan::new(&self.inner)
    }
    /// Configures the span for this `Group`'s delimiters, but not its internal
    /// tokens.
    ///
    /// This method will **not** set the span of all the internal tokens spanned
    /// by this group, but rather it will only set the span of the delimiter
    /// tokens at the level of the `Group`.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner);
    }
}
/// Prints the group as a string that should be losslessly convertible back
/// into the same group (modulo spans), except for possibly `TokenTree::Group`s
/// with `Delimiter::None` delimiters.
impl Display for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, formatter)
    }
}
impl Debug for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, formatter)
    }
}
/// A `Punct` is a single punctuation character like `+`, `-` or `#`.
///
/// Multicharacter operators like `+=` are represented as two instances of
/// `Punct` with different forms of `Spacing` returned.
pub struct Punct {
    ch: char,
    spacing: Spacing,
    span: Span,
}
#[automatically_derived]
impl ::core::clone::Clone for Punct {
    #[inline]
    fn clone(&self) -> Punct {
        Punct {
            ch: ::core::clone::Clone::clone(&self.ch),
            spacing: ::core::clone::Clone::clone(&self.spacing),
            span: ::core::clone::Clone::clone(&self.span),
        }
    }
}
/// Whether a `Punct` is followed immediately by another `Punct` or followed by
/// another token or whitespace.
pub enum Spacing {
    /// E.g. `+` is `Alone` in `+ =`, `+ident` or `+()`.
    Alone,
    /// E.g. `+` is `Joint` in `+=` or `'` is `Joint` in `'#`.
    ///
    /// Additionally, single quote `'` can join with identifiers to form
    /// lifetimes `'ident`.
    Joint,
}
#[automatically_derived]
impl ::core::marker::Copy for Spacing {}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Spacing {}
#[automatically_derived]
impl ::core::clone::Clone for Spacing {
    #[inline]
    fn clone(&self) -> Spacing {
        *self
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Spacing {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Spacing::Alone => "Alone",
                Spacing::Joint => "Joint",
            },
        )
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Spacing {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {}
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Spacing {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Spacing {
    #[inline]
    fn eq(&self, other: &Spacing) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
impl Punct {
    /// Creates a new `Punct` from the given character and spacing.
    ///
    /// The `ch` argument must be a valid punctuation character permitted by the
    /// language, otherwise the function will panic.
    ///
    /// The returned `Punct` will have the default span of `Span::call_site()`
    /// which can be further configured with the `set_span` method below.
    pub fn new(ch: char, spacing: Spacing) -> Self {
        if let '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | ',' | '-' | '.' | '/'
        | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '^' | '|' | '~' = ch {
            Punct {
                ch,
                spacing,
                span: Span::call_site(),
            }
        } else {
            {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "unsupported proc macro punctuation character {0:?}",
                        ch,
                    ),
                );
            };
        }
    }
    /// Returns the value of this punctuation character as `char`.
    pub fn as_char(&self) -> char {
        self.ch
    }
    /// Returns the spacing of this punctuation character, indicating whether
    /// it's immediately followed by another `Punct` in the token stream, so
    /// they can potentially be combined into a multicharacter operator
    /// (`Joint`), or it's followed by some other token or whitespace (`Alone`)
    /// so the operator has certainly ended.
    pub fn spacing(&self) -> Spacing {
        self.spacing
    }
    /// Returns the span for this punctuation character.
    pub fn span(&self) -> Span {
        self.span
    }
    /// Configure the span for this punctuation character.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}
/// Prints the punctuation character as a string that should be losslessly
/// convertible back into the same character.
impl Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ch, f)
    }
}
impl Debug for Punct {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut debug = fmt.debug_struct("Punct");
        debug.field("char", &self.ch);
        debug.field("spacing", &self.spacing);
        imp::debug_span_field_if_nontrivial(&mut debug, self.span.inner);
        debug.finish()
    }
}
/// A word of Rust code, which may be a keyword or legal variable name.
///
/// An identifier consists of at least one Unicode code point, the first of
/// which has the XID_Start property and the rest of which have the XID_Continue
/// property.
///
/// - The empty string is not an identifier. Use `Option<Ident>`.
/// - A lifetime is not an identifier. Use `syn::Lifetime` instead.
///
/// An identifier constructed with `Ident::new` is permitted to be a Rust
/// keyword, though parsing one through its [`Parse`] implementation rejects
/// Rust keywords. Use `input.call(Ident::parse_any)` when parsing to match the
/// behaviour of `Ident::new`.
///
/// [`Parse`]: https://docs.rs/syn/2.0/syn/parse/trait.Parse.html
///
/// # Examples
///
/// A new ident can be created from a string using the `Ident::new` function.
/// A span must be provided explicitly which governs the name resolution
/// behavior of the resulting identifier.
///
/// ```
/// use proc_macro2::{Ident, Span};
///
/// fn main() {
///     let call_ident = Ident::new("calligraphy", Span::call_site());
///
///     println!("{}", call_ident);
/// }
/// ```
///
/// An ident can be interpolated into a token stream using the `quote!` macro.
///
/// ```
/// use proc_macro2::{Ident, Span};
/// use quote::quote;
///
/// fn main() {
///     let ident = Ident::new("demo", Span::call_site());
///
///     // Create a variable binding whose name is this ident.
///     let expanded = quote! { let #ident = 10; };
///
///     // Create a variable binding with a slightly different name.
///     let temp_ident = Ident::new(&format!("new_{}", ident), Span::call_site());
///     let expanded = quote! { let #temp_ident = 10; };
/// }
/// ```
///
/// A string representation of the ident is available through the `to_string()`
/// method.
///
/// ```
/// # use proc_macro2::{Ident, Span};
/// #
/// # let ident = Ident::new("another_identifier", Span::call_site());
/// #
/// // Examine the ident as a string.
/// let ident_string = ident.to_string();
/// if ident_string.len() > 60 {
///     println!("Very long identifier: {}", ident_string)
/// }
/// ```
pub struct Ident {
    inner: imp::Ident,
    _marker: ProcMacroAutoTraits,
}
#[automatically_derived]
impl ::core::clone::Clone for Ident {
    #[inline]
    fn clone(&self) -> Ident {
        Ident {
            inner: ::core::clone::Clone::clone(&self.inner),
            _marker: ::core::clone::Clone::clone(&self._marker),
        }
    }
}
impl Ident {
    fn _new(inner: imp::Ident) -> Self {
        Ident { inner, _marker: MARKER }
    }
    fn _new_fallback(inner: fallback::Ident) -> Self {
        Ident {
            inner: imp::Ident::from(inner),
            _marker: MARKER,
        }
    }
    /// Creates a new `Ident` with the given `string` as well as the specified
    /// `span`.
    ///
    /// The `string` argument must be a valid identifier permitted by the
    /// language, otherwise the function will panic.
    ///
    /// Note that `span`, currently in rustc, configures the hygiene information
    /// for this identifier.
    ///
    /// As of this time `Span::call_site()` explicitly opts-in to "call-site"
    /// hygiene meaning that identifiers created with this span will be resolved
    /// as if they were written directly at the location of the macro call, and
    /// other code at the macro call site will be able to refer to them as well.
    ///
    /// Later spans like `Span::def_site()` will allow to opt-in to
    /// "definition-site" hygiene meaning that identifiers created with this
    /// span will be resolved at the location of the macro definition and other
    /// code at the macro call site will not be able to refer to them.
    ///
    /// Due to the current importance of hygiene this constructor, unlike other
    /// tokens, requires a `Span` to be specified at construction.
    ///
    /// # Panics
    ///
    /// Panics if the input string is neither a keyword nor a legal variable
    /// name. If you are not sure whether the string contains an identifier and
    /// need to handle an error case, use
    /// <a href="https://docs.rs/syn/2.0/syn/fn.parse_str.html"><code
    ///   style="padding-right:0;">syn::parse_str</code></a><code
    ///   style="padding-left:0;">::&lt;Ident&gt;</code>
    /// rather than `Ident::new`.
    #[track_caller]
    pub fn new(string: &str, span: Span) -> Self {
        Ident::_new(imp::Ident::new_checked(string, span.inner))
    }
    /// Same as `Ident::new`, but creates a raw identifier (`r#ident`). The
    /// `string` argument must be a valid identifier permitted by the language
    /// (including keywords, e.g. `fn`). Keywords which are usable in path
    /// segments (e.g. `self`, `super`) are not supported, and will cause a
    /// panic.
    #[track_caller]
    pub fn new_raw(string: &str, span: Span) -> Self {
        Ident::_new(imp::Ident::new_raw_checked(string, span.inner))
    }
    /// Returns the span of this `Ident`.
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Configures the span of this `Ident`, possibly changing its hygiene
    /// context.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner);
    }
}
impl PartialEq for Ident {
    fn eq(&self, other: &Ident) -> bool {
        self.inner == other.inner
    }
}
impl<T> PartialEq<T> for Ident
where
    T: ?Sized + AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        self.inner == other
    }
}
impl Eq for Ident {}
impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Ident) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ident {
    fn cmp(&self, other: &Ident) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}
impl Hash for Ident {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.to_string().hash(hasher);
    }
}
/// Prints the identifier as a string that should be losslessly convertible back
/// into the same identifier.
impl Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
/// A literal string (`"hello"`), byte string (`b"hello"`), character (`'a'`),
/// byte character (`b'a'`), an integer or floating point number with or without
/// a suffix (`1`, `1u8`, `2.3`, `2.3f32`).
///
/// Boolean literals like `true` and `false` do not belong here, they are
/// `Ident`s.
pub struct Literal {
    inner: imp::Literal,
    _marker: ProcMacroAutoTraits,
}
#[automatically_derived]
impl ::core::clone::Clone for Literal {
    #[inline]
    fn clone(&self) -> Literal {
        Literal {
            inner: ::core::clone::Clone::clone(&self.inner),
            _marker: ::core::clone::Clone::clone(&self._marker),
        }
    }
}
impl Literal {
    fn _new(inner: imp::Literal) -> Self {
        Literal { inner, _marker: MARKER }
    }
    fn _new_fallback(inner: fallback::Literal) -> Self {
        Literal {
            inner: imp::Literal::from(inner),
            _marker: MARKER,
        }
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u8_suffixed(n: u8) -> Literal {
        Literal::_new(imp::Literal::u8_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u16_suffixed(n: u16) -> Literal {
        Literal::_new(imp::Literal::u16_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u32_suffixed(n: u32) -> Literal {
        Literal::_new(imp::Literal::u32_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u64_suffixed(n: u64) -> Literal {
        Literal::_new(imp::Literal::u64_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u128_suffixed(n: u128) -> Literal {
        Literal::_new(imp::Literal::u128_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn usize_suffixed(n: usize) -> Literal {
        Literal::_new(imp::Literal::usize_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i8_suffixed(n: i8) -> Literal {
        Literal::_new(imp::Literal::i8_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i16_suffixed(n: i16) -> Literal {
        Literal::_new(imp::Literal::i16_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i32_suffixed(n: i32) -> Literal {
        Literal::_new(imp::Literal::i32_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i64_suffixed(n: i64) -> Literal {
        Literal::_new(imp::Literal::i64_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i128_suffixed(n: i128) -> Literal {
        Literal::_new(imp::Literal::i128_suffixed(n))
    }
    /// Creates a new suffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1u32` where the integer
    /// value specified is the first part of the token and the integral is
    /// also suffixed at the end. Literals created from negative numbers may
    /// not survive roundtrips through `TokenStream` or strings and may be
    /// broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn isize_suffixed(n: isize) -> Literal {
        Literal::_new(imp::Literal::isize_suffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u8_unsuffixed(n: u8) -> Literal {
        Literal::_new(imp::Literal::u8_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u16_unsuffixed(n: u16) -> Literal {
        Literal::_new(imp::Literal::u16_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u32_unsuffixed(n: u32) -> Literal {
        Literal::_new(imp::Literal::u32_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u64_unsuffixed(n: u64) -> Literal {
        Literal::_new(imp::Literal::u64_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn u128_unsuffixed(n: u128) -> Literal {
        Literal::_new(imp::Literal::u128_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn usize_unsuffixed(n: usize) -> Literal {
        Literal::_new(imp::Literal::usize_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i8_unsuffixed(n: i8) -> Literal {
        Literal::_new(imp::Literal::i8_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i16_unsuffixed(n: i16) -> Literal {
        Literal::_new(imp::Literal::i16_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i32_unsuffixed(n: i32) -> Literal {
        Literal::_new(imp::Literal::i32_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i64_unsuffixed(n: i64) -> Literal {
        Literal::_new(imp::Literal::i64_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn i128_unsuffixed(n: i128) -> Literal {
        Literal::_new(imp::Literal::i128_unsuffixed(n))
    }
    /// Creates a new unsuffixed integer literal with the specified value.
    ///
    /// This function will create an integer like `1` where the integer
    /// value specified is the first part of the token. No suffix is
    /// specified on this token, meaning that invocations like
    /// `Literal::i8_unsuffixed(1)` are equivalent to
    /// `Literal::u32_unsuffixed(1)`. Literals created from negative numbers
    /// may not survive roundtrips through `TokenStream` or strings and may
    /// be broken into two tokens (`-` and positive literal).
    ///
    /// Literals created through this method have the `Span::call_site()`
    /// span by default, which can be configured with the `set_span` method
    /// below.
    pub fn isize_unsuffixed(n: isize) -> Literal {
        Literal::_new(imp::Literal::isize_unsuffixed(n))
    }
    /// Creates a new unsuffixed floating-point literal.
    ///
    /// This constructor is similar to those like `Literal::i8_unsuffixed` where
    /// the float's value is emitted directly into the token but no suffix is
    /// used, so it may be inferred to be a `f64` later in the compiler.
    /// Literals created from negative numbers may not survive round-trips
    /// through `TokenStream` or strings and may be broken into two tokens (`-`
    /// and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f64_unsuffixed(f: f64) -> Literal {
        if !f.is_finite() {
            ::core::panicking::panic("assertion failed: f.is_finite()")
        }
        Literal::_new(imp::Literal::f64_unsuffixed(f))
    }
    /// Creates a new suffixed floating-point literal.
    ///
    /// This constructor will create a literal like `1.0f64` where the value
    /// specified is the preceding part of the token and `f64` is the suffix of
    /// the token. This token will always be inferred to be an `f64` in the
    /// compiler. Literals created from negative numbers may not survive
    /// round-trips through `TokenStream` or strings and may be broken into two
    /// tokens (`-` and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f64_suffixed(f: f64) -> Literal {
        if !f.is_finite() {
            ::core::panicking::panic("assertion failed: f.is_finite()")
        }
        Literal::_new(imp::Literal::f64_suffixed(f))
    }
    /// Creates a new unsuffixed floating-point literal.
    ///
    /// This constructor is similar to those like `Literal::i8_unsuffixed` where
    /// the float's value is emitted directly into the token but no suffix is
    /// used, so it may be inferred to be a `f64` later in the compiler.
    /// Literals created from negative numbers may not survive round-trips
    /// through `TokenStream` or strings and may be broken into two tokens (`-`
    /// and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f32_unsuffixed(f: f32) -> Literal {
        if !f.is_finite() {
            ::core::panicking::panic("assertion failed: f.is_finite()")
        }
        Literal::_new(imp::Literal::f32_unsuffixed(f))
    }
    /// Creates a new suffixed floating-point literal.
    ///
    /// This constructor will create a literal like `1.0f32` where the value
    /// specified is the preceding part of the token and `f32` is the suffix of
    /// the token. This token will always be inferred to be an `f32` in the
    /// compiler. Literals created from negative numbers may not survive
    /// round-trips through `TokenStream` or strings and may be broken into two
    /// tokens (`-` and positive literal).
    ///
    /// # Panics
    ///
    /// This function requires that the specified float is finite, for example
    /// if it is infinity or NaN this function will panic.
    pub fn f32_suffixed(f: f32) -> Literal {
        if !f.is_finite() {
            ::core::panicking::panic("assertion failed: f.is_finite()")
        }
        Literal::_new(imp::Literal::f32_suffixed(f))
    }
    /// String literal.
    pub fn string(string: &str) -> Literal {
        Literal::_new(imp::Literal::string(string))
    }
    /// Character literal.
    pub fn character(ch: char) -> Literal {
        Literal::_new(imp::Literal::character(ch))
    }
    /// Byte character literal.
    pub fn byte_character(byte: u8) -> Literal {
        Literal::_new(imp::Literal::byte_character(byte))
    }
    /// Byte string literal.
    pub fn byte_string(bytes: &[u8]) -> Literal {
        Literal::_new(imp::Literal::byte_string(bytes))
    }
    /// C string literal.
    pub fn c_string(string: &CStr) -> Literal {
        Literal::_new(imp::Literal::c_string(string))
    }
    /// Returns the span encompassing this literal.
    pub fn span(&self) -> Span {
        Span::_new(self.inner.span())
    }
    /// Configures the span associated for this literal.
    pub fn set_span(&mut self, span: Span) {
        self.inner.set_span(span.inner);
    }
    /// Returns a `Span` that is a subset of `self.span()` containing only
    /// the source bytes in range `range`. Returns `None` if the would-be
    /// trimmed span is outside the bounds of `self`.
    ///
    /// Warning: the underlying [`proc_macro::Literal::subspan`] method is
    /// nightly-only. When called from within a procedural macro not using a
    /// nightly compiler, this method will always return `None`.
    pub fn subspan<R: RangeBounds<usize>>(&self, range: R) -> Option<Span> {
        self.inner.subspan(range).map(Span::_new)
    }
    #[doc(hidden)]
    pub unsafe fn from_str_unchecked(repr: &str) -> Self {
        Literal::_new(unsafe { imp::Literal::from_str_unchecked(repr) })
    }
}
impl FromStr for Literal {
    type Err = LexError;
    fn from_str(repr: &str) -> Result<Self, LexError> {
        match imp::Literal::from_str_checked(repr) {
            Ok(lit) => Ok(Literal::_new(lit)),
            Err(lex) => {
                Err(LexError {
                    inner: lex,
                    _marker: MARKER,
                })
            }
        }
    }
}
impl Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}
impl Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
/// Public implementation details for the `TokenStream` type, such as iterators.
pub mod token_stream {
    use crate::marker::{ProcMacroAutoTraits, MARKER};
    use crate::{imp, TokenTree};
    use core::fmt::{self, Debug};
    pub use crate::TokenStream;
    /// An iterator over `TokenStream`'s `TokenTree`s.
    ///
    /// The iteration is "shallow", e.g. the iterator doesn't recurse into
    /// delimited groups, and returns whole groups as token trees.
    pub struct IntoIter {
        inner: imp::TokenTreeIter,
        _marker: ProcMacroAutoTraits,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for IntoIter {
        #[inline]
        fn clone(&self) -> IntoIter {
            IntoIter {
                inner: ::core::clone::Clone::clone(&self.inner),
                _marker: ::core::clone::Clone::clone(&self._marker),
            }
        }
    }
    impl Iterator for IntoIter {
        type Item = TokenTree;
        fn next(&mut self) -> Option<TokenTree> {
            self.inner.next()
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }
    impl Debug for IntoIter {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("TokenStream ")?;
            f.debug_list().entries(self.clone()).finish()
        }
    }
    impl IntoIterator for TokenStream {
        type Item = TokenTree;
        type IntoIter = IntoIter;
        fn into_iter(self) -> IntoIter {
            IntoIter {
                inner: self.inner.into_iter(),
                _marker: MARKER,
            }
        }
    }
}
