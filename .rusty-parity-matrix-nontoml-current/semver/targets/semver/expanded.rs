#![feature(prelude_import)]
//! [![github]](https://github.com/dtolnay/semver)&ensp;[![crates-io]](https://crates.io/crates/semver)&ensp;[![docs-rs]](https://docs.rs/semver)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! A parser and evaluator for Cargo's flavor of Semantic Versioning.
//!
//! Semantic Versioning (see <https://semver.org>) is a guideline for how
//! version numbers are assigned and incremented. It is widely followed within
//! the Cargo/crates.io ecosystem for Rust.
//!
//! <br>
//!
//! # Example
//!
//! ```
//! use semver::{BuildMetadata, Prerelease, Version, VersionReq};
//!
//! fn main() {
//!     let req = VersionReq::parse(">=1.2.3, <1.8.0").unwrap();
//!
//!     // Check whether this requirement matches version 1.2.3-alpha.1 (no)
//!     let version = Version {
//!         major: 1,
//!         minor: 2,
//!         patch: 3,
//!         pre: Prerelease::new("alpha.1").unwrap(),
//!         build: BuildMetadata::EMPTY,
//!     };
//!     assert!(!req.matches(&version));
//!
//!     // Check whether it matches 1.3.0 (yes it does)
//!     let version = Version::parse("1.3.0").unwrap();
//!     assert!(req.matches(&version));
//! }
//! ```
//!
//! <br><br>
//!
//! # Scope of this crate
//!
//! Besides Cargo, several other package ecosystems and package managers for
//! other languages also use SemVer:&ensp;RubyGems/Bundler for Ruby, npm for
//! JavaScript, Composer for PHP, CocoaPods for Objective-C...
//!
//! The `semver` crate is specifically intended to implement Cargo's
//! interpretation of Semantic Versioning.
//!
//! Where the various tools differ in their interpretation or implementation of
//! the spec, this crate follows the implementation choices made by Cargo. If
//! you are operating on version numbers from some other package ecosystem, you
//! will want to use a different semver library which is appropriate to that
//! ecosystem.
//!
//! The extent of Cargo's SemVer support is documented in the *[Specifying
//! Dependencies]* chapter of the Cargo reference.
//!
//! [Specifying Dependencies]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
#![doc(html_root_url = "https://docs.rs/semver/1.0.24")]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::incompatible_msrv,
    clippy::items_after_statements,
    clippy::manual_map,
    clippy::match_bool,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::needless_doctest_main,
    clippy::ptr_as_ptr,
    clippy::redundant_else,
    clippy::semicolon_if_nothing_returned,
    clippy::similar_names,
    clippy::unnested_or_patterns,
    clippy::unseparated_literal_suffix,
    clippy::wildcard_imports
)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2018::*;
extern crate alloc;
mod backport {
    pub(crate) use crate::alloc::vec::Vec;
}
mod display {
    use crate::{BuildMetadata, Comparator, Op, Prerelease, Version, VersionReq};
    use core::fmt::{self, Alignment, Debug, Display, Write};
    impl Display for Version {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            let do_display = |formatter: &mut fmt::Formatter| -> fmt::Result {
                formatter
                    .write_fmt(
                        format_args!("{0}.{1}.{2}", self.major, self.minor, self.patch),
                    )?;
                if !self.pre.is_empty() {
                    formatter.write_fmt(format_args!("-{0}", self.pre))?;
                }
                if !self.build.is_empty() {
                    formatter.write_fmt(format_args!("+{0}", self.build))?;
                }
                Ok(())
            };
            let do_len = || -> usize {
                digits(self.major) + 1 + digits(self.minor) + 1 + digits(self.patch)
                    + !self.pre.is_empty() as usize + self.pre.len()
                    + !self.build.is_empty() as usize + self.build.len()
            };
            pad(formatter, do_display, do_len)
        }
    }
    impl Display for VersionReq {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            if self.comparators.is_empty() {
                return formatter.write_str("*");
            }
            for (i, comparator) in self.comparators.iter().enumerate() {
                if i > 0 {
                    formatter.write_str(", ")?;
                }
                formatter.write_fmt(format_args!("{0}", comparator))?;
            }
            Ok(())
        }
    }
    impl Display for Comparator {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            let op = match self.op {
                Op::Exact => "=",
                Op::Greater => ">",
                Op::GreaterEq => ">=",
                Op::Less => "<",
                Op::LessEq => "<=",
                Op::Tilde => "~",
                Op::Caret => "^",
                Op::Wildcard => "",
            };
            formatter.write_str(op)?;
            formatter.write_fmt(format_args!("{0}", self.major))?;
            if let Some(minor) = &self.minor {
                formatter.write_fmt(format_args!(".{0}", minor))?;
                if let Some(patch) = &self.patch {
                    formatter.write_fmt(format_args!(".{0}", patch))?;
                    if !self.pre.is_empty() {
                        formatter.write_fmt(format_args!("-{0}", self.pre))?;
                    }
                } else if self.op == Op::Wildcard {
                    formatter.write_str(".*")?;
                }
            } else if self.op == Op::Wildcard {
                formatter.write_str(".*")?;
            }
            Ok(())
        }
    }
    impl Display for Prerelease {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str(self.as_str())
        }
    }
    impl Display for BuildMetadata {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str(self.as_str())
        }
    }
    impl Debug for Version {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            let mut debug = formatter.debug_struct("Version");
            debug
                .field("major", &self.major)
                .field("minor", &self.minor)
                .field("patch", &self.patch);
            if !self.pre.is_empty() {
                debug.field("pre", &self.pre);
            }
            if !self.build.is_empty() {
                debug.field("build", &self.build);
            }
            debug.finish()
        }
    }
    impl Debug for Prerelease {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_fmt(format_args!("Prerelease(\"{0}\")", self))
        }
    }
    impl Debug for BuildMetadata {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_fmt(format_args!("BuildMetadata(\"{0}\")", self))
        }
    }
    fn pad(
        formatter: &mut fmt::Formatter,
        do_display: impl FnOnce(&mut fmt::Formatter) -> fmt::Result,
        do_len: impl FnOnce() -> usize,
    ) -> fmt::Result {
        let min_width = match formatter.width() {
            Some(min_width) => min_width,
            None => return do_display(formatter),
        };
        let len = do_len();
        if len >= min_width {
            return do_display(formatter);
        }
        let default_align = Alignment::Left;
        let align = formatter.align().unwrap_or(default_align);
        let padding = min_width - len;
        let (pre_pad, post_pad) = match align {
            Alignment::Left => (0, padding),
            Alignment::Right => (padding, 0),
            Alignment::Center => (padding / 2, (padding + 1) / 2),
        };
        let fill = formatter.fill();
        for _ in 0..pre_pad {
            formatter.write_char(fill)?;
        }
        do_display(formatter)?;
        for _ in 0..post_pad {
            formatter.write_char(fill)?;
        }
        Ok(())
    }
    fn digits(val: u64) -> usize {
        if val < 10 { 1 } else { 1 + digits(val / 10) }
    }
}
mod error {
    use crate::parse::Error;
    use core::fmt::{self, Debug, Display};
    pub(crate) enum ErrorKind {
        Empty,
        UnexpectedEnd(Position),
        UnexpectedChar(Position, char),
        UnexpectedCharAfter(Position, char),
        ExpectedCommaFound(Position, char),
        LeadingZero(Position),
        Overflow(Position),
        EmptySegment(Position),
        IllegalCharacter(Position),
        WildcardNotTheOnlyComparator(char),
        UnexpectedAfterWildcard,
        ExcessiveComparators,
    }
    pub(crate) enum Position {
        Major,
        Minor,
        Patch,
        Pre,
        Build,
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
    impl ::core::cmp::Eq for Position {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) {}
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
    impl std::error::Error for Error {}
    impl Display for Error {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            match &self.kind {
                ErrorKind::Empty => {
                    formatter.write_str("empty string, expected a semver version")
                }
                ErrorKind::UnexpectedEnd(pos) => {
                    formatter
                        .write_fmt(
                            format_args!(
                                "unexpected end of input while parsing {0}",
                                pos,
                            ),
                        )
                }
                ErrorKind::UnexpectedChar(pos, ch) => {
                    formatter
                        .write_fmt(
                            format_args!(
                                "unexpected character {0} while parsing {1}",
                                QuotedChar(*ch),
                                pos,
                            ),
                        )
                }
                ErrorKind::UnexpectedCharAfter(pos, ch) => {
                    formatter
                        .write_fmt(
                            format_args!(
                                "unexpected character {0} after {1}",
                                QuotedChar(*ch),
                                pos,
                            ),
                        )
                }
                ErrorKind::ExpectedCommaFound(pos, ch) => {
                    formatter
                        .write_fmt(
                            format_args!(
                                "expected comma after {0}, found {1}",
                                pos,
                                QuotedChar(*ch),
                            ),
                        )
                }
                ErrorKind::LeadingZero(pos) => {
                    formatter.write_fmt(format_args!("invalid leading zero in {0}", pos))
                }
                ErrorKind::Overflow(pos) => {
                    formatter
                        .write_fmt(format_args!("value of {0} exceeds u64::MAX", pos))
                }
                ErrorKind::EmptySegment(pos) => {
                    formatter
                        .write_fmt(format_args!("empty identifier segment in {0}", pos))
                }
                ErrorKind::IllegalCharacter(pos) => {
                    formatter.write_fmt(format_args!("unexpected character in {0}", pos))
                }
                ErrorKind::WildcardNotTheOnlyComparator(ch) => {
                    formatter
                        .write_fmt(
                            format_args!(
                                "wildcard req ({0}) must be the only comparator in the version req",
                                ch,
                            ),
                        )
                }
                ErrorKind::UnexpectedAfterWildcard => {
                    formatter
                        .write_str("unexpected character after wildcard in version req")
                }
                ErrorKind::ExcessiveComparators => {
                    formatter.write_str("excessive number of version comparators")
                }
            }
        }
    }
    impl Display for Position {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter
                .write_str(
                    match self {
                        Position::Major => "major version number",
                        Position::Minor => "minor version number",
                        Position::Patch => "patch version number",
                        Position::Pre => "pre-release identifier",
                        Position::Build => "build metadata",
                    },
                )
        }
    }
    impl Debug for Error {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Error(\"")?;
            Display::fmt(self, formatter)?;
            formatter.write_str("\")")?;
            Ok(())
        }
    }
    struct QuotedChar(char);
    impl Display for QuotedChar {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            if self.0 == '\0' {
                formatter.write_str("'\\0'")
            } else {
                formatter.write_fmt(format_args!("{0:?}", self.0))
            }
        }
    }
}
mod eval {
    use crate::{Comparator, Op, Version, VersionReq};
    pub(crate) fn matches_req(req: &VersionReq, ver: &Version) -> bool {
        for cmp in &req.comparators {
            if !matches_impl(cmp, ver) {
                return false;
            }
        }
        if ver.pre.is_empty() {
            return true;
        }
        for cmp in &req.comparators {
            if pre_is_compatible(cmp, ver) {
                return true;
            }
        }
        false
    }
    pub(crate) fn matches_comparator(cmp: &Comparator, ver: &Version) -> bool {
        matches_impl(cmp, ver) && (ver.pre.is_empty() || pre_is_compatible(cmp, ver))
    }
    fn matches_impl(cmp: &Comparator, ver: &Version) -> bool {
        match cmp.op {
            Op::Exact | Op::Wildcard => matches_exact(cmp, ver),
            Op::Greater => matches_greater(cmp, ver),
            Op::GreaterEq => matches_exact(cmp, ver) || matches_greater(cmp, ver),
            Op::Less => matches_less(cmp, ver),
            Op::LessEq => matches_exact(cmp, ver) || matches_less(cmp, ver),
            Op::Tilde => matches_tilde(cmp, ver),
            Op::Caret => matches_caret(cmp, ver),
        }
    }
    fn matches_exact(cmp: &Comparator, ver: &Version) -> bool {
        if ver.major != cmp.major {
            return false;
        }
        if let Some(minor) = cmp.minor {
            if ver.minor != minor {
                return false;
            }
        }
        if let Some(patch) = cmp.patch {
            if ver.patch != patch {
                return false;
            }
        }
        ver.pre == cmp.pre
    }
    fn matches_greater(cmp: &Comparator, ver: &Version) -> bool {
        if ver.major != cmp.major {
            return ver.major > cmp.major;
        }
        match cmp.minor {
            None => return false,
            Some(minor) => {
                if ver.minor != minor {
                    return ver.minor > minor;
                }
            }
        }
        match cmp.patch {
            None => return false,
            Some(patch) => {
                if ver.patch != patch {
                    return ver.patch > patch;
                }
            }
        }
        ver.pre > cmp.pre
    }
    fn matches_less(cmp: &Comparator, ver: &Version) -> bool {
        if ver.major != cmp.major {
            return ver.major < cmp.major;
        }
        match cmp.minor {
            None => return false,
            Some(minor) => {
                if ver.minor != minor {
                    return ver.minor < minor;
                }
            }
        }
        match cmp.patch {
            None => return false,
            Some(patch) => {
                if ver.patch != patch {
                    return ver.patch < patch;
                }
            }
        }
        ver.pre < cmp.pre
    }
    fn matches_tilde(cmp: &Comparator, ver: &Version) -> bool {
        if ver.major != cmp.major {
            return false;
        }
        if let Some(minor) = cmp.minor {
            if ver.minor != minor {
                return false;
            }
        }
        if let Some(patch) = cmp.patch {
            if ver.patch != patch {
                return ver.patch > patch;
            }
        }
        ver.pre >= cmp.pre
    }
    fn matches_caret(cmp: &Comparator, ver: &Version) -> bool {
        if ver.major != cmp.major {
            return false;
        }
        let minor = match cmp.minor {
            None => return true,
            Some(minor) => minor,
        };
        let patch = match cmp.patch {
            None => {
                if cmp.major > 0 {
                    return ver.minor >= minor;
                } else {
                    return ver.minor == minor;
                }
            }
            Some(patch) => patch,
        };
        if cmp.major > 0 {
            if ver.minor != minor {
                return ver.minor > minor;
            } else if ver.patch != patch {
                return ver.patch > patch;
            }
        } else if minor > 0 {
            if ver.minor != minor {
                return false;
            } else if ver.patch != patch {
                return ver.patch > patch;
            }
        } else if ver.minor != minor || ver.patch != patch {
            return false;
        }
        ver.pre >= cmp.pre
    }
    fn pre_is_compatible(cmp: &Comparator, ver: &Version) -> bool {
        cmp.major == ver.major && cmp.minor == Some(ver.minor)
            && cmp.patch == Some(ver.patch) && !cmp.pre.is_empty()
    }
}
mod identifier {
    use crate::alloc::alloc::{alloc, dealloc, handle_alloc_error, Layout};
    use core::isize;
    use core::mem;
    use core::num::{NonZeroU64, NonZeroUsize};
    use core::ptr::{self, NonNull};
    use core::slice;
    use core::str;
    use core::usize;
    const PTR_BYTES: usize = mem::size_of::<NonNull<u8>>();
    const TAIL_BYTES: usize = 8 * (PTR_BYTES < 8) as usize
        - PTR_BYTES * (PTR_BYTES < 8) as usize;
    #[repr(C, align(8))]
    pub(crate) struct Identifier {
        head: NonNull<u8>,
        tail: [u8; TAIL_BYTES],
    }
    impl Identifier {
        pub(crate) const fn empty() -> Self {
            const HEAD: NonNull<u8> = unsafe { NonNull::new_unchecked(!0 as *mut u8) };
            Identifier {
                head: HEAD,
                tail: [!0; TAIL_BYTES],
            }
        }
        pub(crate) unsafe fn new_unchecked(string: &str) -> Self {
            let len = string.len();
            if true {
                if !(len <= isize::MAX as usize) {
                    ::core::panicking::panic(
                        "assertion failed: len <= isize::MAX as usize",
                    )
                }
            }
            match len as u64 {
                0 => Self::empty(),
                1..=8 => {
                    let mut bytes = [0u8; mem::size_of::<Identifier>()];
                    unsafe {
                        ptr::copy_nonoverlapping(
                            string.as_ptr(),
                            bytes.as_mut_ptr(),
                            len,
                        )
                    };
                    unsafe {
                        mem::transmute::<
                            [u8; mem::size_of::<Identifier>()],
                            Identifier,
                        >(bytes)
                    }
                }
                9..=0xff_ffff_ffff_ffff => {
                    let size = bytes_for_varint(unsafe {
                        NonZeroUsize::new_unchecked(len)
                    }) + len;
                    let align = 2;
                    if mem::size_of::<usize>() < 8 {
                        let max_alloc = usize::MAX / 2 - align;
                        if !(size <= max_alloc) {
                            ::core::panicking::panic(
                                "assertion failed: size <= max_alloc",
                            )
                        }
                    }
                    let layout = unsafe {
                        Layout::from_size_align_unchecked(size, align)
                    };
                    let ptr = unsafe { alloc(layout) };
                    if ptr.is_null() {
                        handle_alloc_error(layout);
                    }
                    let mut write = ptr;
                    let mut varint_remaining = len;
                    while varint_remaining > 0 {
                        unsafe { ptr::write(write, varint_remaining as u8 | 0x80) };
                        varint_remaining >>= 7;
                        write = unsafe { write.add(1) };
                    }
                    unsafe { ptr::copy_nonoverlapping(string.as_ptr(), write, len) };
                    Identifier {
                        head: ptr_to_repr(ptr),
                        tail: [0; TAIL_BYTES],
                    }
                }
                0x100_0000_0000_0000..=0xffff_ffff_ffff_ffff => {
                    {
                        ::core::panicking::unreachable_display(
                            &"please refrain from storing >64 petabytes of text in semver version",
                        );
                    };
                }
            }
        }
        pub(crate) fn is_empty(&self) -> bool {
            let empty = Self::empty();
            let is_empty = self.head == empty.head && self.tail == empty.tail;
            mem::forget(empty);
            is_empty
        }
        fn is_inline(&self) -> bool {
            self.head.as_ptr() as usize >> (PTR_BYTES * 8 - 1) == 0
        }
        fn is_empty_or_inline(&self) -> bool {
            self.is_empty() || self.is_inline()
        }
        pub(crate) fn as_str(&self) -> &str {
            if self.is_empty() {
                ""
            } else if self.is_inline() {
                unsafe { inline_as_str(self) }
            } else {
                unsafe { ptr_as_str(&self.head) }
            }
        }
        pub(crate) fn ptr_eq(&self, rhs: &Self) -> bool {
            self.head == rhs.head && self.tail == rhs.tail
        }
    }
    impl Clone for Identifier {
        fn clone(&self) -> Self {
            if self.is_empty_or_inline() {
                Identifier {
                    head: self.head,
                    tail: self.tail,
                }
            } else {
                let ptr = repr_to_ptr(self.head);
                let len = unsafe { decode_len(ptr) };
                let size = bytes_for_varint(len) + len.get();
                let align = 2;
                let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
                let clone = unsafe { alloc(layout) };
                if clone.is_null() {
                    handle_alloc_error(layout);
                }
                unsafe { ptr::copy_nonoverlapping(ptr, clone, size) }
                Identifier {
                    head: ptr_to_repr(clone),
                    tail: [0; TAIL_BYTES],
                }
            }
        }
    }
    impl Drop for Identifier {
        fn drop(&mut self) {
            if self.is_empty_or_inline() {
                return;
            }
            let ptr = repr_to_ptr_mut(self.head);
            let len = unsafe { decode_len(ptr) };
            let size = bytes_for_varint(len) + len.get();
            let align = 2;
            let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
            unsafe { dealloc(ptr, layout) }
        }
    }
    impl PartialEq for Identifier {
        fn eq(&self, rhs: &Self) -> bool {
            if self.ptr_eq(rhs) {
                true
            } else if self.is_empty_or_inline() || rhs.is_empty_or_inline() {
                false
            } else {
                unsafe { ptr_as_str(&self.head) == ptr_as_str(&rhs.head) }
            }
        }
    }
    unsafe impl Send for Identifier {}
    unsafe impl Sync for Identifier {}
    fn ptr_to_repr(original: *mut u8) -> NonNull<u8> {
        let modified = (original as usize | 1).rotate_right(1);
        let diff = modified.wrapping_sub(original as usize);
        let modified = original.wrapping_add(diff);
        unsafe { NonNull::new_unchecked(modified) }
    }
    fn repr_to_ptr(modified: NonNull<u8>) -> *const u8 {
        let modified = modified.as_ptr();
        let original = (modified as usize) << 1;
        let diff = original.wrapping_sub(modified as usize);
        modified.wrapping_add(diff)
    }
    fn repr_to_ptr_mut(repr: NonNull<u8>) -> *mut u8 {
        repr_to_ptr(repr) as *mut u8
    }
    unsafe fn inline_len(repr: &Identifier) -> NonZeroUsize {
        let repr = unsafe { ptr::read(repr as *const Identifier as *const NonZeroU64) };
        let zero_bits_on_string_end = repr.leading_zeros();
        let nonzero_bytes = 8 - zero_bits_on_string_end as usize / 8;
        unsafe { NonZeroUsize::new_unchecked(nonzero_bytes) }
    }
    unsafe fn inline_as_str(repr: &Identifier) -> &str {
        let ptr = repr as *const Identifier as *const u8;
        let len = unsafe { inline_len(repr) }.get();
        let slice = unsafe { slice::from_raw_parts(ptr, len) };
        unsafe { str::from_utf8_unchecked(slice) }
    }
    unsafe fn decode_len(ptr: *const u8) -> NonZeroUsize {
        let [first, second] = unsafe { ptr::read(ptr as *const [u8; 2]) };
        if second < 0x80 {
            unsafe { NonZeroUsize::new_unchecked((first & 0x7f) as usize) }
        } else {
            return unsafe { decode_len_cold(ptr) };
            #[cold]
            #[inline(never)]
            unsafe fn decode_len_cold(mut ptr: *const u8) -> NonZeroUsize {
                let mut len = 0;
                let mut shift = 0;
                loop {
                    let byte = unsafe { *ptr };
                    if byte < 0x80 {
                        return unsafe { NonZeroUsize::new_unchecked(len) };
                    }
                    ptr = unsafe { ptr.add(1) };
                    len += ((byte & 0x7f) as usize) << shift;
                    shift += 7;
                }
            }
        }
    }
    unsafe fn ptr_as_str(repr: &NonNull<u8>) -> &str {
        let ptr = repr_to_ptr(*repr);
        let len = unsafe { decode_len(ptr) };
        let header = bytes_for_varint(len);
        let slice = unsafe { slice::from_raw_parts(ptr.add(header), len.get()) };
        unsafe { str::from_utf8_unchecked(slice) }
    }
    fn bytes_for_varint(len: NonZeroUsize) -> usize {
        let usize_bits = mem::size_of::<usize>() * 8;
        let len_bits = usize_bits - len.leading_zeros() as usize;
        (len_bits + 6) / 7
    }
}
mod impls {
    use crate::backport::*;
    use crate::identifier::Identifier;
    use crate::{BuildMetadata, Comparator, Prerelease, VersionReq};
    use core::cmp::Ordering;
    use core::hash::{Hash, Hasher};
    use core::iter::FromIterator;
    use core::ops::Deref;
    impl Default for Identifier {
        fn default() -> Self {
            Identifier::empty()
        }
    }
    impl Eq for Identifier {}
    impl Hash for Identifier {
        fn hash<H: Hasher>(&self, hasher: &mut H) {
            self.as_str().hash(hasher);
        }
    }
    impl Deref for Prerelease {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            self.identifier.as_str()
        }
    }
    impl Deref for BuildMetadata {
        type Target = str;
        fn deref(&self) -> &Self::Target {
            self.identifier.as_str()
        }
    }
    impl PartialOrd for Prerelease {
        fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
            Some(self.cmp(rhs))
        }
    }
    impl PartialOrd for BuildMetadata {
        fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
            Some(self.cmp(rhs))
        }
    }
    impl Ord for Prerelease {
        fn cmp(&self, rhs: &Self) -> Ordering {
            if self.identifier.ptr_eq(&rhs.identifier) {
                return Ordering::Equal;
            }
            match self.is_empty() {
                true => return Ordering::Greater,
                false if rhs.is_empty() => return Ordering::Less,
                false => {}
            }
            let lhs = self.as_str().split('.');
            let mut rhs = rhs.as_str().split('.');
            for lhs in lhs {
                let rhs = match rhs.next() {
                    None => return Ordering::Greater,
                    Some(rhs) => rhs,
                };
                let string_cmp = || Ord::cmp(lhs, rhs);
                let is_ascii_digit = |b: u8| b.is_ascii_digit();
                let ordering = match (
                    lhs.bytes().all(is_ascii_digit),
                    rhs.bytes().all(is_ascii_digit),
                ) {
                    (true, true) => {
                        Ord::cmp(&lhs.len(), &rhs.len()).then_with(string_cmp)
                    }
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    (false, false) => string_cmp(),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            if rhs.next().is_none() { Ordering::Equal } else { Ordering::Less }
        }
    }
    impl Ord for BuildMetadata {
        fn cmp(&self, rhs: &Self) -> Ordering {
            if self.identifier.ptr_eq(&rhs.identifier) {
                return Ordering::Equal;
            }
            let lhs = self.as_str().split('.');
            let mut rhs = rhs.as_str().split('.');
            for lhs in lhs {
                let rhs = match rhs.next() {
                    None => return Ordering::Greater,
                    Some(rhs) => rhs,
                };
                let is_ascii_digit = |b: u8| b.is_ascii_digit();
                let ordering = match (
                    lhs.bytes().all(is_ascii_digit),
                    rhs.bytes().all(is_ascii_digit),
                ) {
                    (true, true) => {
                        let lhval = lhs.trim_start_matches('0');
                        let rhval = rhs.trim_start_matches('0');
                        Ord::cmp(&lhval.len(), &rhval.len())
                            .then_with(|| Ord::cmp(lhval, rhval))
                            .then_with(|| Ord::cmp(&lhs.len(), &rhs.len()))
                    }
                    (true, false) => return Ordering::Less,
                    (false, true) => return Ordering::Greater,
                    (false, false) => Ord::cmp(lhs, rhs),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            if rhs.next().is_none() { Ordering::Equal } else { Ordering::Less }
        }
    }
    impl FromIterator<Comparator> for VersionReq {
        fn from_iter<I>(iter: I) -> Self
        where
            I: IntoIterator<Item = Comparator>,
        {
            let comparators = Vec::from_iter(iter);
            VersionReq { comparators }
        }
    }
}
mod parse {
    use crate::backport::*;
    use crate::error::{ErrorKind, Position};
    use crate::identifier::Identifier;
    use crate::{BuildMetadata, Comparator, Op, Prerelease, Version, VersionReq};
    use core::str::FromStr;
    /// Error parsing a SemVer version or version requirement.
    ///
    /// # Example
    ///
    /// ```
    /// use semver::Version;
    ///
    /// fn main() {
    ///     let err = Version::parse("1.q.r").unwrap_err();
    ///
    ///     // "unexpected character 'q' while parsing minor version number"
    ///     eprintln!("{}", err);
    /// }
    /// ```
    pub struct Error {
        pub(crate) kind: ErrorKind,
    }
    impl FromStr for Version {
        type Err = Error;
        fn from_str(text: &str) -> Result<Self, Self::Err> {
            if text.is_empty() {
                return Err(Error::new(ErrorKind::Empty));
            }
            let mut pos = Position::Major;
            let (major, text) = numeric_identifier(text, pos)?;
            let text = dot(text, pos)?;
            pos = Position::Minor;
            let (minor, text) = numeric_identifier(text, pos)?;
            let text = dot(text, pos)?;
            pos = Position::Patch;
            let (patch, text) = numeric_identifier(text, pos)?;
            if text.is_empty() {
                return Ok(Version::new(major, minor, patch));
            }
            let (pre, text) = if let Some(text) = text.strip_prefix('-') {
                pos = Position::Pre;
                let (pre, text) = prerelease_identifier(text)?;
                if pre.is_empty() {
                    return Err(Error::new(ErrorKind::EmptySegment(pos)));
                }
                (pre, text)
            } else {
                (Prerelease::EMPTY, text)
            };
            let (build, text) = if let Some(text) = text.strip_prefix('+') {
                pos = Position::Build;
                let (build, text) = build_identifier(text)?;
                if build.is_empty() {
                    return Err(Error::new(ErrorKind::EmptySegment(pos)));
                }
                (build, text)
            } else {
                (BuildMetadata::EMPTY, text)
            };
            if let Some(unexpected) = text.chars().next() {
                return Err(Error::new(ErrorKind::UnexpectedCharAfter(pos, unexpected)));
            }
            Ok(Version {
                major,
                minor,
                patch,
                pre,
                build,
            })
        }
    }
    impl FromStr for VersionReq {
        type Err = Error;
        fn from_str(text: &str) -> Result<Self, Self::Err> {
            let text = text.trim_start_matches(' ');
            if let Some((ch, text)) = wildcard(text) {
                let rest = text.trim_start_matches(' ');
                if rest.is_empty() {
                    return Ok(VersionReq::STAR);
                } else if rest.starts_with(',') {
                    return Err(Error::new(ErrorKind::WildcardNotTheOnlyComparator(ch)));
                } else {
                    return Err(Error::new(ErrorKind::UnexpectedAfterWildcard));
                }
            }
            let depth = 0;
            let mut comparators = Vec::new();
            let len = version_req(text, &mut comparators, depth)?;
            unsafe { comparators.set_len(len) }
            Ok(VersionReq { comparators })
        }
    }
    impl FromStr for Comparator {
        type Err = Error;
        fn from_str(text: &str) -> Result<Self, Self::Err> {
            let text = text.trim_start_matches(' ');
            let (comparator, pos, rest) = comparator(text)?;
            if !rest.is_empty() {
                let unexpected = rest.chars().next().unwrap();
                return Err(Error::new(ErrorKind::UnexpectedCharAfter(pos, unexpected)));
            }
            Ok(comparator)
        }
    }
    impl FromStr for Prerelease {
        type Err = Error;
        fn from_str(text: &str) -> Result<Self, Self::Err> {
            let (pre, rest) = prerelease_identifier(text)?;
            if !rest.is_empty() {
                return Err(Error::new(ErrorKind::IllegalCharacter(Position::Pre)));
            }
            Ok(pre)
        }
    }
    impl FromStr for BuildMetadata {
        type Err = Error;
        fn from_str(text: &str) -> Result<Self, Self::Err> {
            let (build, rest) = build_identifier(text)?;
            if !rest.is_empty() {
                return Err(Error::new(ErrorKind::IllegalCharacter(Position::Build)));
            }
            Ok(build)
        }
    }
    impl Error {
        fn new(kind: ErrorKind) -> Self {
            Error { kind }
        }
    }
    impl Op {
        const DEFAULT: Self = Op::Caret;
    }
    fn numeric_identifier(input: &str, pos: Position) -> Result<(u64, &str), Error> {
        let mut len = 0;
        let mut value = 0u64;
        while let Some(&digit) = input.as_bytes().get(len) {
            if digit < b'0' || digit > b'9' {
                break;
            }
            if value == 0 && len > 0 {
                return Err(Error::new(ErrorKind::LeadingZero(pos)));
            }
            match value
                .checked_mul(10)
                .and_then(|value| value.checked_add((digit - b'0') as u64))
            {
                Some(sum) => value = sum,
                None => return Err(Error::new(ErrorKind::Overflow(pos))),
            }
            len += 1;
        }
        if len > 0 {
            Ok((value, &input[len..]))
        } else if let Some(unexpected) = input[len..].chars().next() {
            Err(Error::new(ErrorKind::UnexpectedChar(pos, unexpected)))
        } else {
            Err(Error::new(ErrorKind::UnexpectedEnd(pos)))
        }
    }
    fn wildcard(input: &str) -> Option<(char, &str)> {
        if let Some(rest) = input.strip_prefix('*') {
            Some(('*', rest))
        } else if let Some(rest) = input.strip_prefix('x') {
            Some(('x', rest))
        } else if let Some(rest) = input.strip_prefix('X') {
            Some(('X', rest))
        } else {
            None
        }
    }
    fn dot(input: &str, pos: Position) -> Result<&str, Error> {
        if let Some(rest) = input.strip_prefix('.') {
            Ok(rest)
        } else if let Some(unexpected) = input.chars().next() {
            Err(Error::new(ErrorKind::UnexpectedCharAfter(pos, unexpected)))
        } else {
            Err(Error::new(ErrorKind::UnexpectedEnd(pos)))
        }
    }
    fn prerelease_identifier(input: &str) -> Result<(Prerelease, &str), Error> {
        let (string, rest) = identifier(input, Position::Pre)?;
        let identifier = unsafe { Identifier::new_unchecked(string) };
        Ok((Prerelease { identifier }, rest))
    }
    fn build_identifier(input: &str) -> Result<(BuildMetadata, &str), Error> {
        let (string, rest) = identifier(input, Position::Build)?;
        let identifier = unsafe { Identifier::new_unchecked(string) };
        Ok((BuildMetadata { identifier }, rest))
    }
    fn identifier(input: &str, pos: Position) -> Result<(&str, &str), Error> {
        let mut accumulated_len = 0;
        let mut segment_len = 0;
        let mut segment_has_nondigit = false;
        loop {
            match input.as_bytes().get(accumulated_len + segment_len) {
                Some(b'A'..=b'Z') | Some(b'a'..=b'z') | Some(b'-') => {
                    segment_len += 1;
                    segment_has_nondigit = true;
                }
                Some(b'0'..=b'9') => {
                    segment_len += 1;
                }
                boundary => {
                    if segment_len == 0 {
                        if accumulated_len == 0 && boundary != Some(&b'.') {
                            return Ok(("", input));
                        } else {
                            return Err(Error::new(ErrorKind::EmptySegment(pos)));
                        }
                    }
                    if pos == Position::Pre && segment_len > 1 && !segment_has_nondigit
                        && input[accumulated_len..].starts_with('0')
                    {
                        return Err(Error::new(ErrorKind::LeadingZero(pos)));
                    }
                    accumulated_len += segment_len;
                    if boundary == Some(&b'.') {
                        accumulated_len += 1;
                        segment_len = 0;
                        segment_has_nondigit = false;
                    } else {
                        return Ok(input.split_at(accumulated_len));
                    }
                }
            }
        }
    }
    fn op(input: &str) -> (Op, &str) {
        let bytes = input.as_bytes();
        if bytes.first() == Some(&b'=') {
            (Op::Exact, &input[1..])
        } else if bytes.first() == Some(&b'>') {
            if bytes.get(1) == Some(&b'=') {
                (Op::GreaterEq, &input[2..])
            } else {
                (Op::Greater, &input[1..])
            }
        } else if bytes.first() == Some(&b'<') {
            if bytes.get(1) == Some(&b'=') {
                (Op::LessEq, &input[2..])
            } else {
                (Op::Less, &input[1..])
            }
        } else if bytes.first() == Some(&b'~') {
            (Op::Tilde, &input[1..])
        } else if bytes.first() == Some(&b'^') {
            (Op::Caret, &input[1..])
        } else {
            (Op::DEFAULT, input)
        }
    }
    fn comparator(input: &str) -> Result<(Comparator, Position, &str), Error> {
        let (mut op, text) = op(input);
        let default_op = input.len() == text.len();
        let text = text.trim_start_matches(' ');
        let mut pos = Position::Major;
        let (major, text) = numeric_identifier(text, pos)?;
        let mut has_wildcard = false;
        let (minor, text) = if let Some(text) = text.strip_prefix('.') {
            pos = Position::Minor;
            if let Some((_, text)) = wildcard(text) {
                has_wildcard = true;
                if default_op {
                    op = Op::Wildcard;
                }
                (None, text)
            } else {
                let (minor, text) = numeric_identifier(text, pos)?;
                (Some(minor), text)
            }
        } else {
            (None, text)
        };
        let (patch, text) = if let Some(text) = text.strip_prefix('.') {
            pos = Position::Patch;
            if let Some((_, text)) = wildcard(text) {
                if default_op {
                    op = Op::Wildcard;
                }
                (None, text)
            } else if has_wildcard {
                return Err(Error::new(ErrorKind::UnexpectedAfterWildcard));
            } else {
                let (patch, text) = numeric_identifier(text, pos)?;
                (Some(patch), text)
            }
        } else {
            (None, text)
        };
        let (pre, text) = if patch.is_some() && text.starts_with('-') {
            pos = Position::Pre;
            let text = &text[1..];
            let (pre, text) = prerelease_identifier(text)?;
            if pre.is_empty() {
                return Err(Error::new(ErrorKind::EmptySegment(pos)));
            }
            (pre, text)
        } else {
            (Prerelease::EMPTY, text)
        };
        let text = if patch.is_some() && text.starts_with('+') {
            pos = Position::Build;
            let text = &text[1..];
            let (build, text) = build_identifier(text)?;
            if build.is_empty() {
                return Err(Error::new(ErrorKind::EmptySegment(pos)));
            }
            text
        } else {
            text
        };
        let text = text.trim_start_matches(' ');
        let comparator = Comparator {
            op,
            major,
            minor,
            patch,
            pre,
        };
        Ok((comparator, pos, text))
    }
    fn version_req(
        input: &str,
        out: &mut Vec<Comparator>,
        depth: usize,
    ) -> Result<usize, Error> {
        let (comparator, pos, text) = match comparator(input) {
            Ok(success) => success,
            Err(mut error) => {
                if let Some((ch, mut rest)) = wildcard(input) {
                    rest = rest.trim_start_matches(' ');
                    if rest.is_empty() || rest.starts_with(',') {
                        error.kind = ErrorKind::WildcardNotTheOnlyComparator(ch);
                    }
                }
                return Err(error);
            }
        };
        if text.is_empty() {
            out.reserve_exact(depth + 1);
            unsafe { out.as_mut_ptr().add(depth).write(comparator) }
            return Ok(depth + 1);
        }
        let text = if let Some(text) = text.strip_prefix(',') {
            text.trim_start_matches(' ')
        } else {
            let unexpected = text.chars().next().unwrap();
            return Err(Error::new(ErrorKind::ExpectedCommaFound(pos, unexpected)));
        };
        const MAX_COMPARATORS: usize = 32;
        if depth + 1 == MAX_COMPARATORS {
            return Err(Error::new(ErrorKind::ExcessiveComparators));
        }
        let len = version_req(text, out, depth + 1)?;
        unsafe { out.as_mut_ptr().add(depth).write(comparator) }
        Ok(len)
    }
}
use crate::identifier::Identifier;
use core::cmp::Ordering;
use core::str::FromStr;
#[allow(unused_imports)]
use crate::backport::*;
pub use crate::parse::Error;
/// **SemVer version** as defined by <https://semver.org>.
///
/// # Syntax
///
/// - The major, minor, and patch numbers may be any integer 0 through u64::MAX.
///   When representing a SemVer version as a string, each number is written as
///   a base 10 integer. For example, `1.0.119`.
///
/// - Leading zeros are forbidden in those positions. For example `1.01.00` is
///   invalid as a SemVer version.
///
/// - The pre-release identifier, if present, must conform to the syntax
///   documented for [`Prerelease`].
///
/// - The build metadata, if present, must conform to the syntax documented for
///   [`BuildMetadata`].
///
/// - Whitespace is not allowed anywhere in the version.
///
/// # Total ordering
///
/// Given any two SemVer versions, one is less than, greater than, or equal to
/// the other. Versions may be compared against one another using Rust's usual
/// comparison operators.
///
/// - The major, minor, and patch number are compared numerically from left to
///   right, lexicographically ordered as a 3-tuple of integers. So for example
///   version `1.5.0` is less than version `1.19.0`, despite the fact that
///   "1.19.0" &lt; "1.5.0" as ASCIIbetically compared strings and 1.19 &lt; 1.5
///   as real numbers.
///
/// - When major, minor, and patch are equal, a pre-release version is
///   considered less than the ordinary release:&ensp;version `1.0.0-alpha.1` is
///   less than version `1.0.0`.
///
/// - Two pre-releases of the same major, minor, patch are compared by
///   lexicographic ordering of dot-separated components of the pre-release
///   string.
///
///   - Identifiers consisting of only digits are compared
///     numerically:&ensp;`1.0.0-pre.8` is less than `1.0.0-pre.12`.
///
///   - Identifiers that contain a letter or hyphen are compared in ASCII sort
///     order:&ensp;`1.0.0-pre12` is less than `1.0.0-pre8`.
///
///   - Any numeric identifier is always less than any non-numeric
///     identifier:&ensp;`1.0.0-pre.1` is less than `1.0.0-pre.x`.
///
/// Example:&ensp;`1.0.0-alpha`&ensp;&lt;&ensp;`1.0.0-alpha.1`&ensp;&lt;&ensp;`1.0.0-alpha.beta`&ensp;&lt;&ensp;`1.0.0-beta`&ensp;&lt;&ensp;`1.0.0-beta.2`&ensp;&lt;&ensp;`1.0.0-beta.11`&ensp;&lt;&ensp;`1.0.0-rc.1`&ensp;&lt;&ensp;`1.0.0`
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Prerelease,
    pub build: BuildMetadata,
}
#[automatically_derived]
impl ::core::clone::Clone for Version {
    #[inline]
    fn clone(&self) -> Version {
        Version {
            major: ::core::clone::Clone::clone(&self.major),
            minor: ::core::clone::Clone::clone(&self.minor),
            patch: ::core::clone::Clone::clone(&self.patch),
            pre: ::core::clone::Clone::clone(&self.pre),
            build: ::core::clone::Clone::clone(&self.build),
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Version {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<u64>;
        let _: ::core::cmp::AssertParamIsEq<Prerelease>;
        let _: ::core::cmp::AssertParamIsEq<BuildMetadata>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Version {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Version {
    #[inline]
    fn eq(&self, other: &Version) -> bool {
        self.major == other.major && self.minor == other.minor
            && self.patch == other.patch && self.pre == other.pre
            && self.build == other.build
    }
}
#[automatically_derived]
impl ::core::cmp::Ord for Version {
    #[inline]
    fn cmp(&self, other: &Version) -> ::core::cmp::Ordering {
        match ::core::cmp::Ord::cmp(&self.major, &other.major) {
            ::core::cmp::Ordering::Equal => {
                match ::core::cmp::Ord::cmp(&self.minor, &other.minor) {
                    ::core::cmp::Ordering::Equal => {
                        match ::core::cmp::Ord::cmp(&self.patch, &other.patch) {
                            ::core::cmp::Ordering::Equal => {
                                match ::core::cmp::Ord::cmp(&self.pre, &other.pre) {
                                    ::core::cmp::Ordering::Equal => {
                                        ::core::cmp::Ord::cmp(&self.build, &other.build)
                                    }
                                    cmp => cmp,
                                }
                            }
                            cmp => cmp,
                        }
                    }
                    cmp => cmp,
                }
            }
            cmp => cmp,
        }
    }
}
#[automatically_derived]
impl ::core::cmp::PartialOrd for Version {
    #[inline]
    fn partial_cmp(
        &self,
        other: &Version,
    ) -> ::core::option::Option<::core::cmp::Ordering> {
        match ::core::cmp::PartialOrd::partial_cmp(&self.major, &other.major) {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                match ::core::cmp::PartialOrd::partial_cmp(&self.minor, &other.minor) {
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal) => {
                        match ::core::cmp::PartialOrd::partial_cmp(
                            &self.patch,
                            &other.patch,
                        ) {
                            ::core::option::Option::Some(
                                ::core::cmp::Ordering::Equal,
                            ) => {
                                match ::core::cmp::PartialOrd::partial_cmp(
                                    &self.pre,
                                    &other.pre,
                                ) {
                                    ::core::option::Option::Some(
                                        ::core::cmp::Ordering::Equal,
                                    ) => {
                                        ::core::cmp::PartialOrd::partial_cmp(
                                            &self.build,
                                            &other.build,
                                        )
                                    }
                                    cmp => cmp,
                                }
                            }
                            cmp => cmp,
                        }
                    }
                    cmp => cmp,
                }
            }
            cmp => cmp,
        }
    }
}
#[automatically_derived]
impl ::core::hash::Hash for Version {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        ::core::hash::Hash::hash(&self.major, state);
        ::core::hash::Hash::hash(&self.minor, state);
        ::core::hash::Hash::hash(&self.patch, state);
        ::core::hash::Hash::hash(&self.pre, state);
        ::core::hash::Hash::hash(&self.build, state)
    }
}
/// **SemVer version requirement** describing the intersection of some version
/// comparators, such as `>=1.2.3, <1.8`.
///
/// # Syntax
///
/// - Either `*` (meaning "any"), or one or more comma-separated comparators.
///
/// - A [`Comparator`] is an operator ([`Op`]) and a partial version, separated
///   by optional whitespace. For example `>=1.0.0` or `>=1.0`.
///
/// - Build metadata is syntactically permitted on the partial versions, but is
///   completely ignored, as it's never relevant to whether any comparator
///   matches a particular version.
///
/// - Whitespace is permitted around commas and around operators. Whitespace is
///   not permitted within a partial version, i.e. anywhere between the major
///   version number and its minor, patch, pre-release, or build metadata.
pub struct VersionReq {
    pub comparators: Vec<Comparator>,
}
#[automatically_derived]
impl ::core::clone::Clone for VersionReq {
    #[inline]
    fn clone(&self) -> VersionReq {
        VersionReq {
            comparators: ::core::clone::Clone::clone(&self.comparators),
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for VersionReq {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<Vec<Comparator>>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for VersionReq {}
#[automatically_derived]
impl ::core::cmp::PartialEq for VersionReq {
    #[inline]
    fn eq(&self, other: &VersionReq) -> bool {
        self.comparators == other.comparators
    }
}
#[automatically_derived]
impl ::core::hash::Hash for VersionReq {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        ::core::hash::Hash::hash(&self.comparators, state)
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for VersionReq {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field1_finish(
            f,
            "VersionReq",
            "comparators",
            &&self.comparators,
        )
    }
}
/// A pair of comparison operator and partial version, such as `>=1.2`. Forms
/// one piece of a VersionReq.
pub struct Comparator {
    pub op: Op,
    pub major: u64,
    pub minor: Option<u64>,
    /// Patch is only allowed if minor is Some.
    pub patch: Option<u64>,
    /// Non-empty pre-release is only allowed if patch is Some.
    pub pre: Prerelease,
}
#[automatically_derived]
impl ::core::clone::Clone for Comparator {
    #[inline]
    fn clone(&self) -> Comparator {
        Comparator {
            op: ::core::clone::Clone::clone(&self.op),
            major: ::core::clone::Clone::clone(&self.major),
            minor: ::core::clone::Clone::clone(&self.minor),
            patch: ::core::clone::Clone::clone(&self.patch),
            pre: ::core::clone::Clone::clone(&self.pre),
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Comparator {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<Op>;
        let _: ::core::cmp::AssertParamIsEq<u64>;
        let _: ::core::cmp::AssertParamIsEq<Option<u64>>;
        let _: ::core::cmp::AssertParamIsEq<Option<u64>>;
        let _: ::core::cmp::AssertParamIsEq<Prerelease>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Comparator {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Comparator {
    #[inline]
    fn eq(&self, other: &Comparator) -> bool {
        self.major == other.major && self.op == other.op && self.minor == other.minor
            && self.patch == other.patch && self.pre == other.pre
    }
}
#[automatically_derived]
impl ::core::hash::Hash for Comparator {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        ::core::hash::Hash::hash(&self.op, state);
        ::core::hash::Hash::hash(&self.major, state);
        ::core::hash::Hash::hash(&self.minor, state);
        ::core::hash::Hash::hash(&self.patch, state);
        ::core::hash::Hash::hash(&self.pre, state)
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Comparator {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field5_finish(
            f,
            "Comparator",
            "op",
            &self.op,
            "major",
            &self.major,
            "minor",
            &self.minor,
            "patch",
            &self.patch,
            "pre",
            &&self.pre,
        )
    }
}
/// SemVer comparison operator: `=`, `>`, `>=`, `<`, `<=`, `~`, `^`, `*`.
///
/// # Op::Exact
/// - &ensp;**`=I.J.K`**&emsp;&mdash;&emsp;exactly the version I.J.K
/// - &ensp;**`=I.J`**&emsp;&mdash;&emsp;equivalent to `>=I.J.0, <I.(J+1).0`
/// - &ensp;**`=I`**&emsp;&mdash;&emsp;equivalent to `>=I.0.0, <(I+1).0.0`
///
/// # Op::Greater
/// - &ensp;**`>I.J.K`**
/// - &ensp;**`>I.J`**&emsp;&mdash;&emsp;equivalent to `>=I.(J+1).0`
/// - &ensp;**`>I`**&emsp;&mdash;&emsp;equivalent to `>=(I+1).0.0`
///
/// # Op::GreaterEq
/// - &ensp;**`>=I.J.K`**
/// - &ensp;**`>=I.J`**&emsp;&mdash;&emsp;equivalent to `>=I.J.0`
/// - &ensp;**`>=I`**&emsp;&mdash;&emsp;equivalent to `>=I.0.0`
///
/// # Op::Less
/// - &ensp;**`<I.J.K`**
/// - &ensp;**`<I.J`**&emsp;&mdash;&emsp;equivalent to `<I.J.0`
/// - &ensp;**`<I`**&emsp;&mdash;&emsp;equivalent to `<I.0.0`
///
/// # Op::LessEq
/// - &ensp;**`<=I.J.K`**
/// - &ensp;**`<=I.J`**&emsp;&mdash;&emsp;equivalent to `<I.(J+1).0`
/// - &ensp;**`<=I`**&emsp;&mdash;&emsp;equivalent to `<(I+1).0.0`
///
/// # Op::Tilde&emsp;("patch" updates)
/// *Tilde requirements allow the **patch** part of the semver version (the third number) to increase.*
/// - &ensp;**`~I.J.K`**&emsp;&mdash;&emsp;equivalent to `>=I.J.K, <I.(J+1).0`
/// - &ensp;**`~I.J`**&emsp;&mdash;&emsp;equivalent to `=I.J`
/// - &ensp;**`~I`**&emsp;&mdash;&emsp;equivalent to `=I`
///
/// # Op::Caret&emsp;("compatible" updates)
/// *Caret requirements allow parts that are **right of the first nonzero** part of the semver version to increase.*
/// - &ensp;**`^I.J.K`**&ensp;(for I\>0)&emsp;&mdash;&emsp;equivalent to `>=I.J.K, <(I+1).0.0`
/// - &ensp;**`^0.J.K`**&ensp;(for J\>0)&emsp;&mdash;&emsp;equivalent to `>=0.J.K, <0.(J+1).0`
/// - &ensp;**`^0.0.K`**&emsp;&mdash;&emsp;equivalent to `=0.0.K`
/// - &ensp;**`^I.J`**&ensp;(for I\>0 or J\>0)&emsp;&mdash;&emsp;equivalent to `^I.J.0`
/// - &ensp;**`^0.0`**&emsp;&mdash;&emsp;equivalent to `=0.0`
/// - &ensp;**`^I`**&emsp;&mdash;&emsp;equivalent to `=I`
///
/// # Op::Wildcard
/// - &ensp;**`I.J.*`**&emsp;&mdash;&emsp;equivalent to `=I.J`
/// - &ensp;**`I.*`**&ensp;or&ensp;**`I.*.*`**&emsp;&mdash;&emsp;equivalent to `=I`
#[non_exhaustive]
pub enum Op {
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
    Wildcard,
}
#[automatically_derived]
impl ::core::marker::Copy for Op {}
#[automatically_derived]
#[doc(hidden)]
unsafe impl ::core::clone::TrivialClone for Op {}
#[automatically_derived]
impl ::core::clone::Clone for Op {
    #[inline]
    fn clone(&self) -> Op {
        *self
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Op {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {}
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Op {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Op {
    #[inline]
    fn eq(&self, other: &Op) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
#[automatically_derived]
impl ::core::hash::Hash for Op {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        ::core::hash::Hash::hash(&__self_discr, state)
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Op {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Op::Exact => "Exact",
                Op::Greater => "Greater",
                Op::GreaterEq => "GreaterEq",
                Op::Less => "Less",
                Op::LessEq => "LessEq",
                Op::Tilde => "Tilde",
                Op::Caret => "Caret",
                Op::Wildcard => "Wildcard",
            },
        )
    }
}
/// Optional pre-release identifier on a version string. This comes after `-` in
/// a SemVer version, like `1.0.0-alpha.1`
///
/// # Examples
///
/// Some real world pre-release idioms drawn from crates.io:
///
/// - **[mio]** <code>0.7.0-<b>alpha.1</b></code> &mdash; the most common style
///   for numbering pre-releases.
///
/// - **[pest]** <code>1.0.0-<b>beta.8</b></code>,&ensp;<code>1.0.0-<b>rc.0</b></code>
///   &mdash; this crate makes a distinction between betas and release
///   candidates.
///
/// - **[sassers]** <code>0.11.0-<b>shitshow</b></code> &mdash; ???.
///
/// - **[atomic-utils]** <code>0.0.0-<b>reserved</b></code> &mdash; a squatted
///   crate name.
///
/// [mio]: https://crates.io/crates/mio
/// [pest]: https://crates.io/crates/pest
/// [atomic-utils]: https://crates.io/crates/atomic-utils
/// [sassers]: https://crates.io/crates/sassers
///
/// *Tip:* Be aware that if you are planning to number your own pre-releases,
/// you should prefer to separate the numeric part from any non-numeric
/// identifiers by using a dot in between. That is, prefer pre-releases
/// `alpha.1`, `alpha.2`, etc rather than `alpha1`, `alpha2` etc. The SemVer
/// spec's rule for pre-release precedence has special treatment of numeric
/// components in the pre-release string, but only if there are no non-digit
/// characters in the same dot-separated component. So you'd have `alpha.2` &lt;
/// `alpha.11` as intended, but `alpha11` &lt; `alpha2`.
///
/// # Syntax
///
/// Pre-release strings are a series of dot separated identifiers immediately
/// following the patch version. Identifiers must comprise only ASCII
/// alphanumerics and hyphens: `0-9`, `A-Z`, `a-z`, `-`. Identifiers must not be
/// empty. Numeric identifiers must not include leading zeros.
///
/// # Total ordering
///
/// Pre-releases have a total order defined by the SemVer spec. It uses
/// lexicographic ordering of dot-separated components. Identifiers consisting
/// of only digits are compared numerically. Otherwise, identifiers are compared
/// in ASCII sort order. Any numeric identifier is always less than any
/// non-numeric identifier.
///
/// Example:&ensp;`alpha`&ensp;&lt;&ensp;`alpha.85`&ensp;&lt;&ensp;`alpha.90`&ensp;&lt;&ensp;`alpha.200`&ensp;&lt;&ensp;`alpha.0a`&ensp;&lt;&ensp;`alpha.1a0`&ensp;&lt;&ensp;`alpha.a`&ensp;&lt;&ensp;`beta`
pub struct Prerelease {
    identifier: Identifier,
}
#[automatically_derived]
impl ::core::default::Default for Prerelease {
    #[inline]
    fn default() -> Prerelease {
        Prerelease {
            identifier: ::core::default::Default::default(),
        }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for Prerelease {
    #[inline]
    fn clone(&self) -> Prerelease {
        Prerelease {
            identifier: ::core::clone::Clone::clone(&self.identifier),
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Prerelease {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<Identifier>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Prerelease {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Prerelease {
    #[inline]
    fn eq(&self, other: &Prerelease) -> bool {
        self.identifier == other.identifier
    }
}
#[automatically_derived]
impl ::core::hash::Hash for Prerelease {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        ::core::hash::Hash::hash(&self.identifier, state)
    }
}
/// Optional build metadata identifier. This comes after `+` in a SemVer
/// version, as in `0.8.1+zstd.1.5.0`.
///
/// # Examples
///
/// Some real world build metadata idioms drawn from crates.io:
///
/// - **[libgit2-sys]** <code>0.12.20+<b>1.1.0</b></code> &mdash; for this
///   crate, the build metadata indicates the version of the C libgit2 library
///   that the Rust crate is built against.
///
/// - **[mashup]** <code>0.1.13+<b>deprecated</b></code> &mdash; just the word
///   "deprecated" for a crate that has been superseded by another. Eventually
///   people will take notice of this in Cargo's build output where it lists the
///   crates being compiled.
///
/// - **[google-bigquery2]** <code>2.0.4+<b>20210327</b></code> &mdash; this
///   library is automatically generated from an official API schema, and the
///   build metadata indicates the date on which that schema was last captured.
///
/// - **[fbthrift-git]** <code>0.0.6+<b>c7fcc0e</b></code> &mdash; this crate is
///   published from snapshots of a big company monorepo. In monorepo
///   development, there is no concept of versions, and all downstream code is
///   just updated atomically in the same commit that breaking changes to a
///   library are landed. Therefore for crates.io purposes, every published
///   version must be assumed to be incompatible with the previous. The build
///   metadata provides the source control hash of the snapshotted code.
///
/// [libgit2-sys]: https://crates.io/crates/libgit2-sys
/// [mashup]: https://crates.io/crates/mashup
/// [google-bigquery2]: https://crates.io/crates/google-bigquery2
/// [fbthrift-git]: https://crates.io/crates/fbthrift-git
///
/// # Syntax
///
/// Build metadata is a series of dot separated identifiers immediately
/// following the patch or pre-release version. Identifiers must comprise only
/// ASCII alphanumerics and hyphens: `0-9`, `A-Z`, `a-z`, `-`. Identifiers must
/// not be empty. Leading zeros *are* allowed, unlike any other place in the
/// SemVer grammar.
///
/// # Total ordering
///
/// Build metadata is ignored in evaluating `VersionReq`; it plays no role in
/// whether a `Version` matches any one of the comparison operators.
///
/// However for comparing build metadatas among one another, they do have a
/// total order which is determined by lexicographic ordering of dot-separated
/// components. Identifiers consisting of only digits are compared numerically.
/// Otherwise, identifiers are compared in ASCII sort order. Any numeric
/// identifier is always less than any non-numeric identifier.
///
/// Example:&ensp;`demo`&ensp;&lt;&ensp;`demo.85`&ensp;&lt;&ensp;`demo.90`&ensp;&lt;&ensp;`demo.090`&ensp;&lt;&ensp;`demo.200`&ensp;&lt;&ensp;`demo.1a0`&ensp;&lt;&ensp;`demo.a`&ensp;&lt;&ensp;`memo`
pub struct BuildMetadata {
    identifier: Identifier,
}
#[automatically_derived]
impl ::core::default::Default for BuildMetadata {
    #[inline]
    fn default() -> BuildMetadata {
        BuildMetadata {
            identifier: ::core::default::Default::default(),
        }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for BuildMetadata {
    #[inline]
    fn clone(&self) -> BuildMetadata {
        BuildMetadata {
            identifier: ::core::clone::Clone::clone(&self.identifier),
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for BuildMetadata {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) {
        let _: ::core::cmp::AssertParamIsEq<Identifier>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for BuildMetadata {}
#[automatically_derived]
impl ::core::cmp::PartialEq for BuildMetadata {
    #[inline]
    fn eq(&self, other: &BuildMetadata) -> bool {
        self.identifier == other.identifier
    }
}
#[automatically_derived]
impl ::core::hash::Hash for BuildMetadata {
    #[inline]
    fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
        ::core::hash::Hash::hash(&self.identifier, state)
    }
}
impl Version {
    /// Create `Version` with an empty pre-release and build metadata.
    ///
    /// Equivalent to:
    ///
    /// ```
    /// # use semver::{BuildMetadata, Prerelease, Version};
    /// #
    /// # const fn new(major: u64, minor: u64, patch: u64) -> Version {
    /// Version {
    ///     major,
    ///     minor,
    ///     patch,
    ///     pre: Prerelease::EMPTY,
    ///     build: BuildMetadata::EMPTY,
    /// }
    /// # }
    /// ```
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Version {
            major,
            minor,
            patch,
            pre: Prerelease::EMPTY,
            build: BuildMetadata::EMPTY,
        }
    }
    /// Create `Version` by parsing from string representation.
    ///
    /// # Errors
    ///
    /// Possible reasons for the parse to fail include:
    ///
    /// - `1.0` &mdash; too few numeric components. A SemVer version must have
    ///   exactly three. If you are looking at something that has fewer than
    ///   three numbers in it, it's possible it is a `VersionReq` instead (with
    ///   an implicit default `^` comparison operator).
    ///
    /// - `1.0.01` &mdash; a numeric component has a leading zero.
    ///
    /// - `1.0.unknown` &mdash; unexpected character in one of the components.
    ///
    /// - `1.0.0-` or `1.0.0+` &mdash; the pre-release or build metadata are
    ///   indicated present but empty.
    ///
    /// - `1.0.0-alpha_123` &mdash; pre-release or build metadata have something
    ///   outside the allowed characters, which are `0-9`, `A-Z`, `a-z`, `-`,
    ///   and `.` (dot).
    ///
    /// - `23456789999999999999.0.0` &mdash; overflow of a u64.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Version::from_str(text)
    }
    /// Compare the major, minor, patch, and pre-release value of two versions,
    /// disregarding build metadata. Versions that differ only in build metadata
    /// are considered equal. This comparison is what the SemVer spec refers to
    /// as "precedence".
    ///
    /// # Example
    ///
    /// ```
    /// use semver::Version;
    ///
    /// let mut versions = [
    ///     "1.20.0+c144a98".parse::<Version>().unwrap(),
    ///     "1.20.0".parse().unwrap(),
    ///     "1.0.0".parse().unwrap(),
    ///     "1.0.0-alpha".parse().unwrap(),
    ///     "1.20.0+bc17664".parse().unwrap(),
    /// ];
    ///
    /// // This is a stable sort, so it preserves the relative order of equal
    /// // elements. The three 1.20.0 versions differ only in build metadata so
    /// // they are not reordered relative to one another.
    /// versions.sort_by(Version::cmp_precedence);
    /// assert_eq!(versions, [
    ///     "1.0.0-alpha".parse().unwrap(),
    ///     "1.0.0".parse().unwrap(),
    ///     "1.20.0+c144a98".parse().unwrap(),
    ///     "1.20.0".parse().unwrap(),
    ///     "1.20.0+bc17664".parse().unwrap(),
    /// ]);
    ///
    /// // Totally order the versions, including comparing the build metadata.
    /// versions.sort();
    /// assert_eq!(versions, [
    ///     "1.0.0-alpha".parse().unwrap(),
    ///     "1.0.0".parse().unwrap(),
    ///     "1.20.0".parse().unwrap(),
    ///     "1.20.0+bc17664".parse().unwrap(),
    ///     "1.20.0+c144a98".parse().unwrap(),
    /// ]);
    /// ```
    pub fn cmp_precedence(&self, other: &Self) -> Ordering {
        Ord::cmp(
            &(self.major, self.minor, self.patch, &self.pre),
            &(other.major, other.minor, other.patch, &other.pre),
        )
    }
}
impl VersionReq {
    /// A `VersionReq` with no constraint on the version numbers it matches.
    /// Equivalent to `VersionReq::parse("*").unwrap()`.
    ///
    /// In terms of comparators this is equivalent to `>=0.0.0`.
    ///
    /// Counterintuitively a `*` VersionReq does not match every possible
    /// version number. In particular, in order for *any* `VersionReq` to match
    /// a pre-release version, the `VersionReq` must contain at least one
    /// `Comparator` that has an explicit major, minor, and patch version
    /// identical to the pre-release being matched, and that has a nonempty
    /// pre-release component. Since `*` is not written with an explicit major,
    /// minor, and patch version, and does not contain a nonempty pre-release
    /// component, it does not match any pre-release versions.
    pub const STAR: Self = VersionReq {
        comparators: Vec::new(),
    };
    /// Create `VersionReq` by parsing from string representation.
    ///
    /// # Errors
    ///
    /// Possible reasons for the parse to fail include:
    ///
    /// - `>a.b` &mdash; unexpected characters in the partial version.
    ///
    /// - `@1.0.0` &mdash; unrecognized comparison operator.
    ///
    /// - `^1.0.0, ` &mdash; unexpected end of input.
    ///
    /// - `>=1.0 <2.0` &mdash; missing comma between comparators.
    ///
    /// - `*.*` &mdash; unsupported wildcard syntax.
    pub fn parse(text: &str) -> Result<Self, Error> {
        VersionReq::from_str(text)
    }
    /// Evaluate whether the given `Version` satisfies the version requirement
    /// described by `self`.
    pub fn matches(&self, version: &Version) -> bool {
        eval::matches_req(self, version)
    }
}
/// The default VersionReq is the same as [`VersionReq::STAR`].
impl Default for VersionReq {
    fn default() -> Self {
        VersionReq::STAR
    }
}
impl Comparator {
    pub fn parse(text: &str) -> Result<Self, Error> {
        Comparator::from_str(text)
    }
    pub fn matches(&self, version: &Version) -> bool {
        eval::matches_comparator(self, version)
    }
}
impl Prerelease {
    pub const EMPTY: Self = Prerelease {
        identifier: Identifier::empty(),
    };
    pub fn new(text: &str) -> Result<Self, Error> {
        Prerelease::from_str(text)
    }
    pub fn as_str(&self) -> &str {
        self.identifier.as_str()
    }
    pub fn is_empty(&self) -> bool {
        self.identifier.is_empty()
    }
}
impl BuildMetadata {
    pub const EMPTY: Self = BuildMetadata {
        identifier: Identifier::empty(),
    };
    pub fn new(text: &str) -> Result<Self, Error> {
        BuildMetadata::from_str(text)
    }
    pub fn as_str(&self) -> &str {
        self.identifier.as_str()
    }
    pub fn is_empty(&self) -> bool {
        self.identifier.is_empty()
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
